//! Worker-side unit tests for `engine.rs` (a sibling file keeps `engine.rs` under 500 lines).
//! They drive `WorkerState` directly, so no audio hardware or worker thread is involved.

use super::*;

/// Nothing listens on the discard port, so the Whisper probe fails fast and offline.
const OFFLINE_ASR_URL: &str = "http://127.0.0.1:9/inference";

fn offline_state() -> WorkerState {
    WorkerState::new(VoiceConfig {
        asr_endpoint_url: OFFLINE_ASR_URL.to_owned(),
        ..VoiceConfig::default()
    })
}

/// Queues one silent chunk at `rms_level` and runs the worker's audio drain step.
fn feed_level(state: &mut WorkerState, rms_level: f32, event_tx: &Sender<VoiceEvent>) {
    let chunk = AudioChunk {
        samples_16k: Vec::new(),
        rms_level,
        voice_active: false,
    };
    assert!(
        state.audio_tx.try_send(chunk).is_ok(),
        "audio queue accepts a chunk"
    );
    drain_audio_chunks(state, event_tx);
}

fn telemetry_events(event_rx: &Receiver<VoiceEvent>) -> Vec<VoiceTelemetry> {
    let mut out = Vec::new();
    while let Ok(event) = event_rx.try_recv() {
        if let VoiceEvent::Telemetry(telemetry) = event {
            out.push(telemetry);
        }
    }
    out
}

#[test]
fn mic_level_jitter_emits_telemetry_only_when_the_meter_bars_change() {
    let mut state = offline_state();
    let (event_tx, event_rx) = bounded(EVENT_QUEUE_CAP);
    emit_telemetry(&mut state, &event_tx);
    assert_eq!(telemetry_events(&event_rx).len(), 1, "startup telemetry");

    // 0.30..=0.50 all show two meter bars: the first level changes the meter (0 → 2 bars), the
    // jitter after it changes only the undisplayed raw level.
    for level in [0.30, 0.31, 0.42, 0.35, 0.50] {
        feed_level(&mut state, level, &event_tx);
    }
    let emitted = telemetry_events(&event_rx);
    assert_eq!(emitted.len(), 1, "steady two-bar level must emit once");
    assert_eq!(emitted[0].meter_bars, 2);
    assert_eq!(
        state.telemetry.mic_level, 0.50,
        "the worker still tracks the latest raw level"
    );

    feed_level(&mut state, 0.70, &event_tx);
    let emitted = telemetry_events(&event_rx);
    assert_eq!(emitted.len(), 1, "a meter-bar change is visible again");
    assert_eq!(emitted[0].meter_bars, 3);
}

#[test]
fn a_dropped_telemetry_event_does_not_suppress_the_next_visible_change() {
    let mut state = offline_state();
    // A full event queue: the GPUI side has not drained yet.
    let (event_tx, event_rx) = bounded(1);
    emit_telemetry(&mut state, &event_tx);
    feed_level(&mut state, 0.30, &event_tx);
    assert_eq!(
        telemetry_events(&event_rx)
            .iter()
            .map(|t| t.meter_bars)
            .collect::<Vec<_>>(),
        vec![0],
        "the 2-bar update was dropped by the full queue"
    );

    // Same level again: it was never delivered, so it must still be sent.
    feed_level(&mut state, 0.31, &event_tx);
    let emitted = telemetry_events(&event_rx);
    assert_eq!(emitted.len(), 1);
    assert_eq!(emitted[0].meter_bars, 2);
}
