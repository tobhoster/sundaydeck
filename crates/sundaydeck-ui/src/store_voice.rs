use std::sync::Arc;

use gpui::Context;
use sundaydeck_core::{OutputOverride, PresentationCommand, ScriptureRef, Theme};
use sundaydeck_voice::config::{AUTO_FOLLOW_PAUSED_STATUS, DEFAULT_INPUT_DEVICE_NAME};
use sundaydeck_voice::{VoiceCommand, VoiceEngineHandle, VoiceEvent, VoiceSyncMode};

use crate::store::PresentationStore;

/// Spoken references kept in the Auto-Follow Scripture Queue (newest first).
const MAX_QUEUED_SCRIPTURES: usize = 5;

impl PresentationStore {
    /// Attaches a running background `sundaydeck-voice` worker and syncs the current live deck.
    pub fn attach_voice_engine(&mut self, handle: VoiceEngineHandle) {
        self.voice_devices = VoiceEngineHandle::list_devices();
        self.voice_engine = Some(handle);
        self.last_voice_deck = None;
        self.sync_voice_engine_state();
    }

    /// Switches Auto-Follow mode (`Off` / `Suggest` / `Auto`) and forwards to the background worker.
    pub fn set_voice_sync_mode(&mut self, mode: VoiceSyncMode, cx: &mut Context<Self>) {
        self.voice_sync_mode = mode;
        self.voice_telemetry.mode = mode;
        self.voice_telemetry.listening = mode != VoiceSyncMode::Off;
        if mode == VoiceSyncMode::Off {
            self.voice_telemetry.meter_bars = 0;
            self.voice_telemetry.suggested_slide_index = None;
            self.voice_telemetry.rehearsal_running = false;
            self.voice_telemetry.status_label = AUTO_FOLLOW_PAUSED_STATUS.to_owned();
        } else {
            self.voice_telemetry.status_label = format!("Auto-Follow: {}", mode.label());
        }
        self.forward_to_voice(VoiceCommand::SetMode(mode));
        cx.notify();
    }

    /// Selects a specific OS microphone input device for live `cpal` capture.
    pub fn select_voice_device(&mut self, device_name: Option<String>, cx: &mut Context<Self>) {
        self.voice_telemetry.device_name = device_name
            .clone()
            .unwrap_or_else(|| DEFAULT_INPUT_DEVICE_NAME.to_owned());
        self.forward_to_voice(VoiceCommand::SelectDevice(device_name));
        cx.notify();
    }

    /// Starts or stops the built-in offline rehearsal simulator over the active live slide deck.
    pub fn toggle_voice_rehearsal(&mut self, cx: &mut Context<Self>) {
        let running = self.voice_telemetry.rehearsal_running;
        if running {
            self.voice_telemetry.rehearsal_running = false;
            self.forward_to_voice(VoiceCommand::StopRehearsalDemo);
        } else {
            if self.voice_sync_mode == VoiceSyncMode::Off {
                self.set_voice_sync_mode(VoiceSyncMode::Suggest, cx);
            }
            self.sync_voice_engine_state();
            self.voice_telemetry.rehearsal_running = true;
            self.forward_to_voice(VoiceCommand::StartRehearsalDemo);
        }
        cx.notify();
    }

    /// Synchronizes the current live deck slides, active index, and override lock state
    /// to the background voice worker whenever live presentation state changes. The deck itself
    /// is only re-sent when it changed; a plain slide step sends just the new index.
    pub fn sync_voice_engine_state(&mut self) {
        let Some(engine) = &self.voice_engine else {
            return;
        };
        let snap = self.snapshot();
        let locked = matches!(
            snap.output_override,
            OutputOverride::Black | OutputOverride::Logo
        );
        self.forward_to_voice(VoiceCommand::SetOverrideLocked(locked));
        let Some(live) = &snap.live else {
            return;
        };
        let active_index = live.active_index;
        let deck_known = self
            .last_voice_deck
            .as_ref()
            .is_some_and(|deck| Arc::ptr_eq(deck, &live.slides));
        let command = if deck_known {
            VoiceCommand::NoteSlideChanged { active_index }
        } else {
            VoiceCommand::UpdateLiveDeck {
                slides: Arc::clone(&live.slides),
                active_index,
            }
        };
        // A dropped command leaves the worker behind; forgetting the deck makes the next sync
        // resend it whole (with the current index) instead of stepping an outdated deck.
        self.last_voice_deck = match engine.send(command) {
            Ok(()) => Some(Arc::clone(&live.slides)),
            Err(err) => {
                tracing::warn!("voice deck sync dropped: {err}");
                None
            }
        };
    }

    /// Best-effort forward to the attached voice worker (none attached: nothing to do). A dropped
    /// command is logged; the operator can repeat the action.
    pub(crate) fn forward_to_voice(&self, command: VoiceCommand) {
        if let Some(engine) = &self.voice_engine
            && let Err(err) = engine.send(command)
        {
            tracing::warn!("voice command dropped: {err}");
        }
    }

    /// Drains asynchronous telemetry & auto-advance decisions from the background voice thread.
    /// Notifies only when something the operator can see changed; a telemetry update that only
    /// moves the (undisplayed) raw mic level is applied silently.
    pub fn poll_voice_events(&mut self, cx: &mut Context<Self>) {
        let Some(engine) = self.voice_engine.clone() else {
            return;
        };
        let mut visible_change = false;
        for event in engine.drain_events() {
            match event {
                VoiceEvent::Telemetry(t) => {
                    visible_change |= !t.same_display(&self.voice_telemetry);
                    self.voice_sync_mode = t.mode;
                    self.voice_telemetry = t;
                }
                VoiceEvent::AutoAdvance { target_index, .. } => {
                    let _ = self.dispatch(PresentationCommand::SelectLiveSlide(target_index), cx);
                    visible_change = true;
                }
                VoiceEvent::ScriptureMentioned { reference, .. } => {
                    visible_change |= self.queue_scripture(reference);
                }
            }
        }
        if visible_change {
            cx.notify();
        }
    }

    /// Queues a spoken reference newest-first, keeping at most [`MAX_QUEUED_SCRIPTURES`].
    /// Like `HashSet::insert`, returns whether it was newly queued.
    fn queue_scripture(&mut self, reference: ScriptureRef) -> bool {
        if self.queued_scriptures.contains(&reference) {
            return false;
        }
        self.queued_scriptures.insert(0, reference);
        self.queued_scriptures.truncate(MAX_QUEUED_SCRIPTURES);
        true
    }

    /// Deterministic preview helper for screenshot verification & offline rehearsal inspection.
    pub fn set_voice_preview_state(
        &mut self,
        mode: VoiceSyncMode,
        progress: f32,
        suggested_idx: Option<usize>,
        transcript: impl Into<String>,
        confidence: f32,
        cx: &mut Context<Self>,
    ) {
        self.voice_sync_mode = mode;
        self.voice_telemetry.mode = mode;
        self.voice_telemetry.listening = mode != VoiceSyncMode::Off;
        self.voice_telemetry.active_slide_progress = progress.clamp(0.0, 1.0);
        self.voice_telemetry.suggested_slide_index = suggested_idx;
        self.voice_telemetry.last_transcript = transcript.into();
        self.voice_telemetry.confidence = confidence.clamp(0.0, 1.0);
        self.voice_telemetry.meter_bars = match mode {
            VoiceSyncMode::Off => 0,
            VoiceSyncMode::Suggest => 2,
            VoiceSyncMode::Auto => 3,
        };
        self.voice_telemetry.status_label = match (mode, suggested_idx) {
            (VoiceSyncMode::Off, _) => AUTO_FOLLOW_PAUSED_STATUS.to_owned(),
            (VoiceSyncMode::Suggest, Some(idx)) => {
                format!("Suggested slide {} ({:.0}%)", idx + 1, confidence * 100.0)
            }
            (VoiceSyncMode::Auto, _) => {
                format!("Listening · {:.0}% match", confidence * 100.0)
            }
            (VoiceSyncMode::Suggest, None) => {
                format!("Listening · {:.0}% progress", progress * 100.0)
            }
        };
        cx.notify();
    }

    /// Adds or replaces a custom Photo / Video / Public Domain theme in memory and optionally
    /// applies it live right away. Returns the owned `Theme` for off-thread SQLite persistence.
    pub fn upsert_theme_local(
        &mut self,
        theme: Theme,
        apply_live: bool,
        cx: &mut Context<Self>,
    ) -> Theme {
        let arc_theme = Arc::new(theme.clone());
        if let Some(existing) = self.themes.iter_mut().find(|t| t.id == theme.id) {
            *existing = Arc::clone(&arc_theme);
        } else {
            self.themes.push(Arc::clone(&arc_theme));
        }
        if apply_live {
            let _ = self.dispatch(PresentationCommand::ApplyLiveTheme(arc_theme), cx);
        } else {
            cx.notify();
        }
        self.set_status(format!("Saved theme \"{}\"", theme.name), cx);
        theme
    }
}
