use std::time::{Duration, Instant};

use sundaydeck_voice::{
    VoiceCommand, VoiceConfig, VoiceEngineHandle, VoiceError, VoiceEvent, VoiceSyncMode,
    VoiceTelemetry,
};

const MISSING_DEVICE: &str = "__sundaydeck_missing__";

/// Polls the worker until it reports the given mode, bounded so a silent worker cannot hang.
fn wait_for_mode_telemetry(handle: &VoiceEngineHandle, mode: VoiceSyncMode) -> VoiceTelemetry {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        for event in handle.drain_events() {
            if let VoiceEvent::Telemetry(t) = event
                && t.mode == mode
            {
                return t;
            }
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    panic!("voice worker emitted no {mode:?} telemetry within 5 s");
}

#[test]
fn missing_mic_surfaces_its_error_reason_in_status_label() {
    let handle = VoiceEngineHandle::spawn(VoiceConfig {
        device_name: Some(MISSING_DEVICE.to_owned()),
        ..VoiceConfig::default()
    });
    handle
        .send(VoiceCommand::SetMode(VoiceSyncMode::Auto))
        .expect("worker accepts the mode");
    let telemetry = wait_for_mode_telemetry(&handle, VoiceSyncMode::Auto);
    handle
        .send(VoiceCommand::Shutdown)
        .expect("worker accepts shutdown");

    let label = &telemetry.status_label;
    let reason = label
        .strip_prefix("Mic unavailable (")
        .and_then(|rest| rest.rfind(')').map(|end| &rest[..end]))
        .unwrap_or_else(|| panic!("expected `Mic unavailable (<reason>) …`, got {label:?}"));
    assert!(
        !reason.trim().is_empty(),
        "empty mic error reason in {label:?}"
    );

    // Hosts that can enumerate inputs report the missing device; hosts without an audio
    // backend (headless CI) fail enumeration itself.
    let device_unavailable = VoiceError::DeviceUnavailable {
        name: MISSING_DEVICE.to_owned(),
    }
    .to_string();
    let stream_failed_prefix = VoiceError::AudioStreamFailed {
        detail: String::new(),
    }
    .to_string();
    assert!(
        reason == device_unavailable || reason.starts_with(&stream_failed_prefix),
        "status reason {reason:?} is not the VoiceError text for the missing mic"
    );
    // Rehearsal (Test Auto-Follow) must stay usable without a microphone.
    assert!(telemetry.listening, "{telemetry:?}");
}
