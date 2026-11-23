use serde::{Deserialize, Serialize};

/// Plain-English Auto-Follow operating mode surfaced in the Top Bar popover, Design Gallery,
/// and live `sundaydeck-voice` alignment engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VoiceSyncMode {
    #[default]
    Off,
    Suggest,
    Auto,
}

impl VoiceSyncMode {
    pub const ALL: [Self; 3] = [Self::Off, Self::Suggest, Self::Auto];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::Suggest => "Suggest next slide",
            Self::Auto => "Advance automatically",
        }
    }

    pub const fn pill_label(self) -> &'static str {
        match self {
            Self::Off => "Auto-Follow",
            Self::Suggest => "Auto-Follow: Suggest",
            Self::Auto => "Auto-Follow: On",
        }
    }
}

/// Discovered OS microphone / audio-interface input device.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AudioInputDevice {
    pub name: String,
    pub is_default: bool,
}

/// Runtime tuning parameters for offline lyric & scripture voice alignment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VoiceConfig {
    pub mode: VoiceSyncMode,
    pub device_name: Option<String>,
    /// Local `whisper.cpp` HTTP server endpoint (100% offline localhost).
    pub asr_endpoint_url: String,
    /// Minimum alignment confidence in `[0.0, 1.0]` required for automatic slide advance.
    pub advance_confidence_threshold: f32,
    /// Minimum alignment confidence in `[0.0, 1.0]` required to highlight a suggested next slide.
    pub suggest_confidence_threshold: f32,
    /// Debounce cooldown after any slide change so sustained singing notes never double-advance.
    pub cooldown_ms: u64,
    /// Extra score margin required before jumping out-of-sequence (e.g. spontaneous Chorus repeat).
    pub jump_penalty: f32,
    /// Keep Auto-Follow paused while `Black Screen` or `Show Logo` override is latched.
    pub lock_during_override: bool,
}

impl Default for VoiceConfig {
    fn default() -> Self {
        Self {
            mode: VoiceSyncMode::Off,
            device_name: None,
            asr_endpoint_url: "http://127.0.0.1:8081/inference".to_owned(),
            advance_confidence_threshold: 0.70,
            suggest_confidence_threshold: 0.48,
            cooldown_ms: 1600,
            jump_penalty: 0.14,
            lock_during_override: true,
        }
    }
}

/// Snapshot of live voice-following telemetry pushed from the background worker to GPUI.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VoiceTelemetry {
    pub mode: VoiceSyncMode,
    pub listening: bool,
    pub mic_level: f32,
    pub meter_bars: usize,
    pub active_slide_index: usize,
    pub active_slide_progress: f32,
    pub suggested_slide_index: Option<usize>,
    pub confidence: f32,
    pub last_transcript: String,
    pub status_label: String,
    pub backend_label: String,
    pub device_name: String,
    pub locked_by_override: bool,
    pub rehearsal_running: bool,
}

impl Default for VoiceTelemetry {
    fn default() -> Self {
        Self {
            mode: VoiceSyncMode::Off,
            listening: false,
            mic_level: 0.0,
            meter_bars: 0,
            active_slide_index: 0,
            active_slide_progress: 0.0,
            suggested_slide_index: None,
            confidence: 0.0,
            last_transcript: String::new(),
            status_label: "Auto-Follow paused".to_owned(),
            backend_label: "Local Whisper (Offline)".to_owned(),
            device_name: "System Default Input".to_owned(),
            locked_by_override: false,
            rehearsal_running: false,
        }
    }
}
