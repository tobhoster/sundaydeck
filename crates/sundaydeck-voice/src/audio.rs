use async_channel::Sender;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Sample, SampleFormat, Stream};

use crate::config::AudioInputDevice;
use crate::error::VoiceError;

pub const TARGET_SAMPLE_RATE_HZ: u32 = 16_000;

/// Chunk of mono 16 kHz PCM samples + instantaneous RMS energy level (`0.0..=1.0`).
#[derive(Debug, Clone)]
pub struct AudioChunk {
    pub samples_16k: Vec<i16>,
    pub rms_level: f32,
    pub voice_active: bool,
}

/// Discovers connected OS audio input devices via `cpal` (CoreAudio on macOS, ALSA/PipeWire on
/// Linux, WASAPI on Windows). Never panics on headless hosts without audio hardware.
pub fn list_input_devices() -> Vec<AudioInputDevice> {
    let host = cpal::default_host();
    let default_name = host
        .default_input_device()
        .and_then(|d| d.description().ok().map(|desc| desc.name().to_owned()));

    let Ok(devices) = host.input_devices() else {
        return Vec::new();
    };

    let mut out = Vec::new();
    for dev in devices {
        if let Ok(desc) = dev.description() {
            let name = desc.name().to_owned();
            let is_default = default_name.as_deref() == Some(name.as_str());
            out.push(AudioInputDevice { name, is_default });
        }
    }
    out
}

/// Active `cpal` microphone capture stream pushing downsampled 16 kHz chunks off the UI thread.
pub struct LiveAudioCapture {
    _stream: Stream,
    pub device_name: String,
}

impl LiveAudioCapture {
    pub fn start(
        requested_device: Option<&str>,
        tx: Sender<AudioChunk>,
    ) -> Result<Self, VoiceError> {
        let host = cpal::default_host();
        let device: Device = match requested_device {
            Some(wanted) if !wanted.trim().is_empty() => host
                .input_devices()
                .map_err(|err| VoiceError::AudioStreamFailed {
                    detail: err.to_string(),
                })?
                .find(|d| d.description().is_ok_and(|desc| desc.name() == wanted))
                .ok_or_else(|| VoiceError::DeviceUnavailable {
                    name: wanted.to_owned(),
                })?,
            _ => host
                .default_input_device()
                .ok_or(VoiceError::NoInputDevice)?,
        };

        let device_name = device
            .description()
            .map(|d| d.name().to_owned())
            .unwrap_or_else(|_| "System Default Input".to_owned());

        let supported =
            device
                .default_input_config()
                .map_err(|err| VoiceError::AudioStreamFailed {
                    detail: err.to_string(),
                })?;

        let sample_rate = supported.sample_rate();
        let channels = usize::from(supported.channels()).max(1);
        let err_fn = |err| {
            let _ = err;
        };

        let stream = match supported.sample_format() {
            SampleFormat::F32 => {
                let tx_c = tx.clone();
                device.build_input_stream(
                    supported.into(),
                    move |data: &[f32], _| push_mono_chunk(data, channels, sample_rate, &tx_c),
                    err_fn,
                    None,
                )
            }
            SampleFormat::I16 => {
                let tx_c = tx.clone();
                device.build_input_stream(
                    supported.into(),
                    move |data: &[i16], _| {
                        let floats: Vec<f32> = data.iter().map(|s| s.to_sample::<f32>()).collect();
                        push_mono_chunk(&floats, channels, sample_rate, &tx_c);
                    },
                    err_fn,
                    None,
                )
            }
            SampleFormat::U16 => {
                let tx_c = tx.clone();
                device.build_input_stream(
                    supported.into(),
                    move |data: &[u16], _| {
                        let floats: Vec<f32> = data.iter().map(|s| s.to_sample::<f32>()).collect();
                        push_mono_chunk(&floats, channels, sample_rate, &tx_c);
                    },
                    err_fn,
                    None,
                )
            }
            other => {
                return Err(VoiceError::AudioStreamFailed {
                    detail: format!("unsupported sample format {other:?}"),
                });
            }
        }
        .map_err(|err| VoiceError::AudioStreamFailed {
            detail: err.to_string(),
        })?;

        stream.play().map_err(|err| VoiceError::AudioStreamFailed {
            detail: err.to_string(),
        })?;

        Ok(Self {
            _stream: stream,
            device_name,
        })
    }
}

fn push_mono_chunk(data: &[f32], channels: usize, src_rate_hz: u32, tx: &Sender<AudioChunk>) {
    if data.is_empty() {
        return;
    }
    let frame_count = data.len() / channels;
    if frame_count == 0 {
        return;
    }

    let mut mono = Vec::with_capacity(frame_count);
    let mut sum_sq = 0.0f32;
    for frame in data.chunks_exact(channels) {
        let avg = frame.iter().copied().sum::<f32>() / channels as f32;
        sum_sq += avg * avg;
        mono.push(avg);
    }

    let rms = (sum_sq / mono.len() as f32).sqrt();
    let rms_level = (rms * 5.5).clamp(0.0, 1.0);
    let voice_active = rms >= 0.015;
    let samples_16k = downsample_to_16k_i16(&mono, src_rate_hz);

    let _ = tx.try_send(AudioChunk {
        samples_16k,
        rms_level,
        voice_active,
    });
}

/// Converts normalized audio level (`0.0..=1.0`) into 0..=3 level meter bars.
pub fn level_to_meter_bars(level: f32) -> usize {
    if level >= 0.58 {
        3
    } else if level >= 0.25 {
        2
    } else if level >= 0.06 {
        1
    } else {
        0
    }
}

/// Linear resampler converting arbitrary input sample rate (e.g. 44.1/48 kHz) to 16 kHz `i16` PCM.
pub fn downsample_to_16k_i16(mono: &[f32], src_rate_hz: u32) -> Vec<i16> {
    if mono.is_empty() || src_rate_hz == 0 {
        return Vec::new();
    }
    if src_rate_hz == TARGET_SAMPLE_RATE_HZ {
        return mono
            .iter()
            .map(|&s| (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)
            .collect();
    }
    let ratio = src_rate_hz as f64 / f64::from(TARGET_SAMPLE_RATE_HZ);
    let out_len = ((mono.len() as f64) / ratio).max(1.0) as usize;
    let mut out = Vec::with_capacity(out_len);
    for idx in 0..out_len {
        let src_pos = (idx as f64) * ratio;
        let i0 = (src_pos.floor() as usize).min(mono.len() - 1);
        let i1 = (i0 + 1).min(mono.len() - 1);
        let frac = (src_pos - src_pos.floor()) as f32;
        let sample = mono[i0] * (1.0 - frac) + mono[i1] * frac;
        out.push((sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16);
    }
    out
}

/// Encodes mono 16-bit PCM samples as a canonical RIFF/WAVE byte buffer in memory (zero disk I/O).
pub fn encode_wav_pcm16(samples: &[i16], sample_rate_hz: u32) -> Vec<u8> {
    let data_bytes = (samples.len() * 2) as u32;
    let riff_size = 36u32.saturating_add(data_bytes);
    let byte_rate = sample_rate_hz.saturating_mul(2);

    let mut buf = Vec::with_capacity(44 + samples.len() * 2);
    buf.extend_from_slice(b"RIFF");
    buf.extend_from_slice(&riff_size.to_le_bytes());
    buf.extend_from_slice(b"WAVE");
    buf.extend_from_slice(b"fmt ");
    buf.extend_from_slice(&16u32.to_le_bytes()); // PCM fmt chunk size
    buf.extend_from_slice(&1u16.to_le_bytes()); // PCM format
    buf.extend_from_slice(&1u16.to_le_bytes()); // Mono
    buf.extend_from_slice(&sample_rate_hz.to_le_bytes());
    buf.extend_from_slice(&byte_rate.to_le_bytes());
    buf.extend_from_slice(&2u16.to_le_bytes()); // Block align
    buf.extend_from_slice(&16u16.to_le_bytes()); // Bits per sample
    buf.extend_from_slice(b"data");
    buf.extend_from_slice(&data_bytes.to_le_bytes());
    for &sample in samples {
        buf.extend_from_slice(&sample.to_le_bytes());
    }
    buf
}
