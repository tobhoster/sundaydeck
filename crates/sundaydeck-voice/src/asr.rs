use std::time::Duration;

use serde::Deserialize;
use sundaydeck_core::Slide;
use ureq::Agent;

use crate::audio::{TARGET_SAMPLE_RATE_HZ, encode_wav_pcm16};
use crate::error::VoiceError;

#[derive(Debug, Deserialize)]
struct WhisperInferenceResponse {
    #[serde(default)]
    text: String,
}

/// Lightweight offline HTTP client targeting `whisper.cpp`'s `whisper-server` (`POST /inference`).
/// Keeps SundayDeck's binary lean and lets macOS Apple Silicon offload ASR to CoreML / Metal.
pub struct WhisperHttpClient {
    agent: Agent,
    endpoint_url: String,
}

impl WhisperHttpClient {
    pub fn new(endpoint_url: impl Into<String>) -> Self {
        let agent: Agent = Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(6)))
            .build()
            .into();
        Self {
            agent,
            endpoint_url: endpoint_url.into(),
        }
    }

    /// Probes whether a local `whisper-server` is currently listening on localhost.
    pub fn is_server_reachable(&self) -> bool {
        let health_url = self
            .endpoint_url
            .strip_suffix("/inference")
            .map(|base| format!("{base}/health"))
            .unwrap_or_else(|| self.endpoint_url.clone());
        let quick_agent: Agent = Agent::config_builder()
            .timeout_global(Some(Duration::from_millis(350)))
            .build()
            .into();
        quick_agent.get(&health_url).call().is_ok()
    }

    /// Sends a short 16 kHz mono PCM speech window as multipart/form-data WAV to `whisper-server`.
    pub fn transcribe_pcm16(&self, samples_16k: &[i16]) -> Result<String, VoiceError> {
        let wav_bytes = encode_wav_pcm16(samples_16k, TARGET_SAMPLE_RATE_HZ);
        let boundary = "----SundayDeckWhisperBoundary9f4a";
        let mut body = Vec::with_capacity(wav_bytes.len() + 512);

        body.extend_from_slice(
            format!(
                "--{boundary}\r\n\
                 Content-Disposition: form-data; name=\"file\"; filename=\"chunk.wav\"\r\n\
                 Content-Type: audio/wav\r\n\r\n"
            )
            .as_bytes(),
        );
        body.extend_from_slice(&wav_bytes);
        body.extend_from_slice(
            format!(
                "\r\n--{boundary}\r\n\
                 Content-Disposition: form-data; name=\"response_format\"\r\n\r\n\
                 json\r\n\
                 --{boundary}\r\n\
                 Content-Disposition: form-data; name=\"temperature\"\r\n\r\n\
                 0.0\r\n\
                 --{boundary}--\r\n"
            )
            .as_bytes(),
        );

        let content_type = format!("multipart/form-data; boundary={boundary}");
        let mut resp = self
            .agent
            .post(&self.endpoint_url)
            .header("Content-Type", &content_type)
            .send(body.as_slice())
            .map_err(|err| VoiceError::AsrUnavailable {
                endpoint: self.endpoint_url.clone(),
                detail: err.to_string(),
            })?;

        let text =
            resp.body_mut()
                .read_to_string()
                .map_err(|err| VoiceError::AsrInvalidResponse {
                    detail: err.to_string(),
                })?;

        let parsed: WhisperInferenceResponse =
            serde_json::from_str(&text).map_err(|err| VoiceError::AsrInvalidResponse {
                detail: err.to_string(),
            })?;

        Ok(parsed.text.trim().to_owned())
    }
}

/// Breaks a slide's lyrics or scripture text into realistic spoken/sung chunks for built-in
/// offline Auto-Follow rehearsal testing (no microphone required).
pub fn rehearsal_chunks_for_slide(slide: &Slide) -> Vec<String> {
    let lines: Vec<String> = slide
        .content
        .primary_text()
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(ToOwned::to_owned)
        .collect();
    if lines.is_empty() {
        return Vec::new();
    }
    lines
}
