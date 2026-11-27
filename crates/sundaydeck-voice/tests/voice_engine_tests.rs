use sundaydeck_voice::{VoiceSyncMode, VoiceTelemetry};

/// Changes one displayed telemetry field.
type TelemetryMutator = fn(&mut VoiceTelemetry);

fn listening_telemetry() -> VoiceTelemetry {
    VoiceTelemetry {
        mode: VoiceSyncMode::Auto,
        listening: true,
        mic_level: 0.12,
        meter_bars: 1,
        active_slide_index: 2,
        active_slide_progress: 0.4,
        suggested_slide_index: None,
        confidence: 0.55,
        last_transcript: "amazing grace how sweet the sound".to_owned(),
        status_label: "Listening · USB Mic".to_owned(),
        backend_label: "Offline Aligner · Rehearsal Ready".to_owned(),
        device_name: "USB Mic".to_owned(),
        locked_by_override: false,
        rehearsal_running: false,
    }
}

#[test]
fn same_display_ignores_only_mic_level() {
    let base = listening_telemetry();
    assert!(base.same_display(&base.clone()));

    for mic_level in [0.0, 0.13, 0.85, 1.0] {
        let other = VoiceTelemetry {
            mic_level,
            ..listening_telemetry()
        };
        assert!(
            base.same_display(&other) && other.same_display(&base),
            "mic_level {mic_level} alone must not count as a display change"
        );
    }
}

#[test]
fn same_display_detects_every_displayed_field_change() {
    let cases: [(&str, TelemetryMutator); 13] = [
        ("mode", |t| t.mode = VoiceSyncMode::Suggest),
        ("listening", |t| t.listening = false),
        ("meter_bars", |t| t.meter_bars = 3),
        ("active_slide_index", |t| t.active_slide_index = 3),
        ("active_slide_progress", |t| t.active_slide_progress = 0.9),
        ("suggested_slide_index", |t| {
            t.suggested_slide_index = Some(3)
        }),
        ("confidence", |t| t.confidence = 0.8),
        ("last_transcript", |t| {
            t.last_transcript = "that saved a wretch like me".to_owned()
        }),
        ("status_label", |t| {
            t.status_label = "Microphone disconnected · pick an input or reconnect".to_owned()
        }),
        ("backend_label", |t| {
            t.backend_label = "Local Whisper Server (Ready)".to_owned()
        }),
        ("device_name", |t| t.device_name = "Built-in Mic".to_owned()),
        ("locked_by_override", |t| t.locked_by_override = true),
        ("rehearsal_running", |t| t.rehearsal_running = true),
    ];

    let base = listening_telemetry();
    for (field, change) in cases {
        let mut other = listening_telemetry();
        change(&mut other);
        // A simultaneous mic_level change must not mask the displayed-field change.
        other.mic_level = 0.9;
        assert!(
            !base.same_display(&other) && !other.same_display(&base),
            "changing `{field}` must count as a display change"
        );
    }
}
