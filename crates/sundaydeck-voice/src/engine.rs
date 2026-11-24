use std::sync::mpsc::{self, RecvTimeoutError, SyncSender};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use async_channel::{Receiver, Sender, bounded};
use sundaydeck_core::{ScriptureRef, Slide, extract_scripture_refs_from_text};

use crate::aligner::{DeckVoiceAligner, VoiceDecision};
use crate::asr::{WhisperHttpClient, rehearsal_chunks_for_slide};
use crate::audio::{
    AudioChunk, LiveAudioCapture, TARGET_SAMPLE_RATE_HZ, level_to_meter_bars, list_input_devices,
};
use crate::config::{AudioInputDevice, VoiceConfig, VoiceSyncMode, VoiceTelemetry};

const COMMAND_QUEUE_CAP: usize = 64;
const EVENT_QUEUE_CAP: usize = 128;
const AUDIO_QUEUE_CAP: usize = 32;
const ASR_WINDOW_SAMPLES: usize = (TARGET_SAMPLE_RATE_HZ as usize) * 2; // 2.0s rolling window

/// Commands dispatched from the GPUI main thread to the background `sundaydeck-voice` worker.
#[derive(Debug, Clone)]
pub enum VoiceCommand {
    SetMode(VoiceSyncMode),
    SelectDevice(Option<String>),
    UpdateLiveDeck {
        slides: Vec<Slide>,
        active_index: usize,
    },
    NoteSlideChanged {
        active_index: usize,
    },
    SetOverrideLocked(bool),
    /// Feeds an external or simulated transcript directly through the real alignment pipeline.
    InjectTranscript {
        text: String,
        mic_level: f32,
    },
    /// Runs a live timed rehearsal of the active deck so volunteers can verify Auto-Follow offline.
    StartRehearsalDemo,
    StopRehearsalDemo,
    /// Applies updated endpoint URL, advance confidence threshold, and override-lock settings live.
    UpdateConfig {
        asr_endpoint_url: String,
        advance_confidence_threshold: f32,
        lock_during_override: bool,
    },
    Shutdown,
}

/// Events emitted asynchronously from the voice worker back to GPUI.
#[derive(Debug, Clone)]
pub enum VoiceEvent {
    Telemetry(VoiceTelemetry),
    AutoAdvance {
        target_index: usize,
        confidence: f32,
        reason: String,
    },
    /// Detected spoken Bible reference during sermon preaching (queued for 1-click staging,
    /// never auto-fired live).
    ScriptureMentioned {
        reference: ScriptureRef,
        raw_excerpt: String,
    },
}

/// Non-blocking handle owning the background voice-sync thread.
#[derive(Clone)]
pub struct VoiceEngineHandle {
    cmd_tx: SyncSender<VoiceCommand>,
    event_rx: Receiver<VoiceEvent>,
}

impl VoiceEngineHandle {
    pub fn spawn(config: VoiceConfig) -> Self {
        let (cmd_tx, cmd_rx) = mpsc::sync_channel::<VoiceCommand>(COMMAND_QUEUE_CAP);
        let (event_tx, event_rx) = bounded::<VoiceEvent>(EVENT_QUEUE_CAP);

        let _ = thread::Builder::new()
            .name("sundaydeck-voice".to_owned())
            .spawn(move || voice_worker_loop(config, cmd_rx, event_tx));

        Self { cmd_tx, event_rx }
    }

    pub fn send(&self, cmd: VoiceCommand) {
        let _ = self.cmd_tx.try_send(cmd);
    }

    pub fn drain_events(&self) -> Vec<VoiceEvent> {
        let mut out = Vec::new();
        while let Ok(ev) = self.event_rx.try_recv() {
            out.push(ev);
        }
        out
    }

    pub fn list_devices() -> Vec<AudioInputDevice> {
        list_input_devices()
    }
}

struct WorkerState {
    config: VoiceConfig,
    aligner: DeckVoiceAligner,
    slides: Vec<Slide>,
    telemetry: VoiceTelemetry,
    capture: Option<LiveAudioCapture>,
    audio_tx: Sender<AudioChunk>,
    audio_rx: Receiver<AudioChunk>,
    pcm_ring: Vec<i16>,
    last_asr_call: Instant,
    whisper: WhisperHttpClient,
    whisper_online: bool,
    rehearsal_queue: Vec<(usize, String)>,
    next_rehearsal_tick: Instant,
    last_scripture_ref: Option<ScriptureRef>,
}

fn voice_worker_loop(
    config: VoiceConfig,
    cmd_rx: mpsc::Receiver<VoiceCommand>,
    event_tx: Sender<VoiceEvent>,
) {
    let (audio_tx, audio_rx) = bounded::<AudioChunk>(AUDIO_QUEUE_CAP);
    let whisper = WhisperHttpClient::new(config.asr_endpoint_url.clone());
    let whisper_online = whisper.is_server_reachable();

    let mut state = WorkerState {
        telemetry: VoiceTelemetry {
            mode: config.mode,
            backend_label: if whisper_online {
                "Local Whisper Server (Ready)".to_owned()
            } else {
                "Offline Aligner · Whisper server idle".to_owned()
            },
            ..VoiceTelemetry::default()
        },
        config,
        aligner: DeckVoiceAligner::new(),
        slides: Vec::new(),
        capture: None,
        audio_tx,
        audio_rx,
        pcm_ring: Vec::with_capacity(ASR_WINDOW_SAMPLES * 2),
        last_asr_call: Instant::now(),
        whisper,
        whisper_online,
        rehearsal_queue: Vec::new(),
        next_rehearsal_tick: Instant::now(),
        last_scripture_ref: None,
    };

    emit_telemetry(&state.telemetry, &event_tx);

    loop {
        match cmd_rx.recv_timeout(Duration::from_millis(75)) {
            Ok(VoiceCommand::Shutdown) => break,
            Ok(cmd) => handle_command(&mut state, cmd, &event_tx),
            Err(RecvTimeoutError::Disconnected) => break,
            Err(RecvTimeoutError::Timeout) => {}
        }

        drain_audio_chunks(&mut state, &event_tx);
        step_rehearsal_demo(&mut state, &event_tx);
    }
}

fn handle_command(state: &mut WorkerState, cmd: VoiceCommand, event_tx: &Sender<VoiceEvent>) {
    let now_ms = current_epoch_ms();
    match cmd {
        VoiceCommand::SetMode(mode) => {
            state.config.mode = mode;
            state.telemetry.mode = mode;
            state.telemetry.suggested_slide_index = None;
            if mode == VoiceSyncMode::Off {
                state.capture = None;
                state.rehearsal_queue.clear();
                state.telemetry.listening = false;
                state.telemetry.rehearsal_running = false;
                state.telemetry.mic_level = 0.0;
                state.telemetry.meter_bars = 0;
                state.telemetry.status_label = "Auto-Follow paused".to_owned();
            } else {
                state.whisper_online = state.whisper.is_server_reachable();
                state.telemetry.backend_label = if state.whisper_online {
                    "Local Whisper Server (Ready)".to_owned()
                } else {
                    "Offline Aligner · Rehearsal Ready".to_owned()
                };
                start_mic_if_available(state);
            }
            emit_telemetry(&state.telemetry, event_tx);
        }
        VoiceCommand::SelectDevice(name) => {
            state.config.device_name = name;
            if state.config.mode != VoiceSyncMode::Off {
                state.capture = None;
                start_mic_if_available(state);
            }
            emit_telemetry(&state.telemetry, event_tx);
        }
        VoiceCommand::UpdateLiveDeck {
            slides,
            active_index,
        } => {
            state.slides = slides;
            state
                .aligner
                .update_deck(&state.slides, active_index, now_ms);
            state.telemetry.active_slide_index = state.aligner.active_index();
            state.telemetry.active_slide_progress = 0.0;
            state.telemetry.suggested_slide_index = None;
            emit_telemetry(&state.telemetry, event_tx);
        }
        VoiceCommand::NoteSlideChanged { active_index } => {
            state.aligner.note_slide_changed(active_index, now_ms);
            state.telemetry.active_slide_index = state.aligner.active_index();
            state.telemetry.active_slide_progress = 0.0;
            state.telemetry.suggested_slide_index = None;
            emit_telemetry(&state.telemetry, event_tx);
        }
        VoiceCommand::SetOverrideLocked(locked) => {
            state.aligner.set_override_locked(locked);
            state.telemetry.locked_by_override = locked;
            if locked && state.telemetry.listening {
                state.telemetry.status_label = "Paused while screen override is active".to_owned();
            }
            emit_telemetry(&state.telemetry, event_tx);
        }
        VoiceCommand::InjectTranscript { text, mic_level } => {
            process_transcript(state, &text, mic_level, now_ms, event_tx);
        }
        VoiceCommand::StartRehearsalDemo => {
            if state.config.mode == VoiceSyncMode::Off {
                state.config.mode = VoiceSyncMode::Suggest;
                state.telemetry.mode = VoiceSyncMode::Suggest;
            }
            state.rehearsal_queue.clear();
            let start_idx = state.aligner.active_index();
            for (offset, slide) in state.slides.iter().enumerate().skip(start_idx).take(3) {
                for chunk in rehearsal_chunks_for_slide(slide) {
                    state.rehearsal_queue.push((offset, chunk));
                }
            }
            state.telemetry.listening = true;
            state.telemetry.rehearsal_running = !state.rehearsal_queue.is_empty();
            state.telemetry.status_label = "Rehearsal listening to live slides".to_owned();
            state.next_rehearsal_tick = Instant::now();
            emit_telemetry(&state.telemetry, event_tx);
        }
        VoiceCommand::StopRehearsalDemo => {
            state.rehearsal_queue.clear();
            state.telemetry.rehearsal_running = false;
            emit_telemetry(&state.telemetry, event_tx);
        }
        VoiceCommand::UpdateConfig {
            asr_endpoint_url,
            advance_confidence_threshold,
            lock_during_override,
        } => {
            state.config.asr_endpoint_url = asr_endpoint_url.clone();
            state.config.advance_confidence_threshold =
                advance_confidence_threshold.clamp(0.45, 0.95);
            state.config.lock_during_override = lock_during_override;
            state.whisper = WhisperHttpClient::new(asr_endpoint_url);
            state.whisper_online = state.whisper.is_server_reachable();
            state.telemetry.backend_label = if state.whisper_online {
                "Local Whisper Server (Ready)".to_owned()
            } else {
                "Offline Aligner · Rehearsal Ready".to_owned()
            };
            emit_telemetry(&state.telemetry, event_tx);
        }
        VoiceCommand::Shutdown => {}
    }
}

fn start_mic_if_available(state: &mut WorkerState) {
    match LiveAudioCapture::start(state.config.device_name.as_deref(), state.audio_tx.clone()) {
        Ok(cap) => {
            state.telemetry.device_name = cap.device_name.clone();
            state.telemetry.listening = true;
            state.telemetry.status_label = format!("Listening · {}", cap.device_name);
            state.capture = Some(cap);
        }
        Err(_) => {
            state.telemetry.listening = true;
            state.telemetry.device_name = state
                .config
                .device_name
                .clone()
                .unwrap_or_else(|| "System Default Input".to_owned());
            state.telemetry.status_label = "Ready (use Test Auto-Follow or attach mic)".to_owned();
        }
    }
}

fn drain_audio_chunks(state: &mut WorkerState, event_tx: &Sender<VoiceEvent>) {
    let mut latest_level = None;
    let mut any_voice = false;

    while let Ok(chunk) = state.audio_rx.try_recv() {
        latest_level = Some(chunk.rms_level);
        if chunk.voice_active {
            any_voice = true;
            state.pcm_ring.extend_from_slice(&chunk.samples_16k);
            if state.pcm_ring.len() > ASR_WINDOW_SAMPLES * 2 {
                let drop_n = state.pcm_ring.len() - ASR_WINDOW_SAMPLES;
                state.pcm_ring.drain(..drop_n);
            }
        }
    }

    if let Some(level) = latest_level {
        state.telemetry.mic_level = level;
        state.telemetry.meter_bars = level_to_meter_bars(level);
        emit_telemetry(&state.telemetry, event_tx);
    }

    if any_voice
        && state.whisper_online
        && state.pcm_ring.len() >= ASR_WINDOW_SAMPLES
        && state.last_asr_call.elapsed() >= Duration::from_millis(1200)
    {
        state.last_asr_call = Instant::now();
        let slice_start = state.pcm_ring.len().saturating_sub(ASR_WINDOW_SAMPLES);
        let window = state.pcm_ring[slice_start..].to_vec();
        if let Ok(text) = state.whisper.transcribe_pcm16(&window)
            && !text.is_empty()
        {
            process_transcript(
                state,
                &text,
                state.telemetry.mic_level.max(0.35),
                current_epoch_ms(),
                event_tx,
            );
        }
    }
}

fn step_rehearsal_demo(state: &mut WorkerState, event_tx: &Sender<VoiceEvent>) {
    if state.rehearsal_queue.is_empty() || Instant::now() < state.next_rehearsal_tick {
        return;
    }
    let (_, phrase) = state.rehearsal_queue.remove(0);
    state.next_rehearsal_tick = Instant::now() + Duration::from_millis(950);
    process_transcript(state, &phrase, 0.68, current_epoch_ms(), event_tx);
    if state.rehearsal_queue.is_empty() {
        state.telemetry.rehearsal_running = false;
        emit_telemetry(&state.telemetry, event_tx);
    }
}

fn process_transcript(
    state: &mut WorkerState,
    text: &str,
    mic_level: f32,
    now_ms: i64,
    event_tx: &Sender<VoiceEvent>,
) {
    let update = state.aligner.ingest_transcript(text, now_ms, &state.config);
    state.telemetry.mic_level = mic_level;
    state.telemetry.meter_bars = level_to_meter_bars(mic_level);
    state.telemetry.last_transcript = text.to_owned();
    state.telemetry.active_slide_index = state.aligner.active_index();
    state.telemetry.active_slide_progress = update.slide_progress;
    state.telemetry.confidence = update.confidence;

    match update.decision {
        VoiceDecision::Hold => {
            state.telemetry.status_label = format!(
                "Listening · {:.0}% slide progress",
                update.slide_progress * 100.0
            );
        }
        VoiceDecision::Suggest {
            target_index,
            confidence,
            reason,
        } => {
            state.telemetry.suggested_slide_index = Some(target_index);
            state.telemetry.status_label = format!("{reason} ({:.0}%)", confidence * 100.0);
        }
        VoiceDecision::Advance {
            target_index,
            confidence,
            reason,
        } => {
            state.telemetry.suggested_slide_index = None;
            state.telemetry.status_label = format!("Advanced · {reason}");
            let _ = event_tx.try_send(VoiceEvent::AutoAdvance {
                target_index,
                confidence,
                reason,
            });
        }
    }

    emit_telemetry(&state.telemetry, event_tx);

    // Sermon Scripture Listener: surface spoken Bible references into the 1-Click Staging Queue.
    for sref in extract_scripture_refs_from_text(text) {
        if state.last_scripture_ref != Some(sref) {
            state.last_scripture_ref = Some(sref);
            let _ = event_tx.try_send(VoiceEvent::ScriptureMentioned {
                reference: sref,
                raw_excerpt: text.to_owned(),
            });
        }
    }
}

fn emit_telemetry(telemetry: &VoiceTelemetry, tx: &Sender<VoiceEvent>) {
    let _ = tx.try_send(VoiceEvent::Telemetry(telemetry.clone()));
}

fn current_epoch_ms() -> i64 {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    i64::try_from(millis).unwrap_or(i64::MAX)
}
