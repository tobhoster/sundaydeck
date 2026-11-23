use thiserror::Error;

#[derive(Debug, Error)]
pub enum VoiceError {
    #[error("no audio input device detected on this system")]
    NoInputDevice,
    #[error("requested microphone '{name}' is not available")]
    DeviceUnavailable { name: String },
    #[error("could not open audio capture stream: {detail}")]
    AudioStreamFailed { detail: String },
    #[error("local speech server unreachable at {endpoint}: {detail}")]
    AsrUnavailable { endpoint: String, detail: String },
    #[error("local speech server returned invalid response: {detail}")]
    AsrInvalidResponse { detail: String },
}
