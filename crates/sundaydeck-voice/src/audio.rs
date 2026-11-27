use std::fmt::Display;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use async_channel::Sender;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{
    Device, ErrorKind, FromSample, InputCallbackInfo, Sample, SampleFormat, SizedSample, Stream,
    StreamConfig,
};

use crate::config::{AudioInputDevice, DEFAULT_INPUT_DEVICE_NAME};
use crate::error::VoiceError;

pub const TARGET_SAMPLE_RATE_HZ: u32 = 16_000;
/// Gain mapping raw mono RMS onto the `0.0..=1.0` meter level.
const RMS_METER_GAIN: f32 = 5.5;
/// Mono RMS at or above which a chunk counts as voice and feeds the ASR window.
const VOICE_ACTIVITY_RMS: f32 = 0.015;

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
    /// Set from the cpal error callback (realtime-safe) once the stream died, e.g. mic unplugged.
    stream_failed: Arc<AtomicBool>,
    pub device_name: String,
}

impl LiveAudioCapture {
    pub fn start(
        requested_device: Option<&str>,
        tx: Sender<AudioChunk>,
    ) -> Result<Self, VoiceError> {
        let device = resolve_input_device(requested_device)?;
        let device_name = device
            .description()
            .map(|d| d.name().to_owned())
            .unwrap_or_else(|_| DEFAULT_INPUT_DEVICE_NAME.to_owned());
        let supported = device.default_input_config().map_err(stream_error)?;
        let sink = ChunkSink {
            config: supported.config(),
            tx,
        };
        let stream_failed = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&stream_failed);

        let stream = match supported.sample_format() {
            SampleFormat::F32 => build_stream::<f32>(&device, sink, flag),
            SampleFormat::I16 => build_stream::<i16>(&device, sink, flag),
            SampleFormat::U16 => build_stream::<u16>(&device, sink, flag),
            other => {
                return Err(VoiceError::AudioStreamFailed {
                    detail: format!("unsupported sample format {other:?}"),
                });
            }
        }
        .map_err(stream_error)?;
        stream.play().map_err(stream_error)?;

        Ok(Self {
            _stream: stream,
            stream_failed,
            device_name,
        })
    }

    /// Whether cpal reported a fatal stream error since `start` (device unplugged, stream
    /// invalidated, ...). A failed capture delivers no more audio and should be dropped.
    pub fn has_failed(&self) -> bool {
        self.stream_failed.load(Ordering::Relaxed)
    }
}

fn resolve_input_device(requested_device: Option<&str>) -> Result<Device, VoiceError> {
    let host = cpal::default_host();
    match requested_device {
        Some(wanted) if !wanted.trim().is_empty() => host
            .input_devices()
            .map_err(stream_error)?
            .find(|d| d.description().is_ok_and(|desc| desc.name() == wanted))
            .ok_or_else(|| VoiceError::DeviceUnavailable {
                name: wanted.to_owned(),
            }),
        _ => host.default_input_device().ok_or(VoiceError::NoInputDevice),
    }
}

fn stream_error(err: impl Display) -> VoiceError {
    VoiceError::AudioStreamFailed {
        detail: err.to_string(),
    }
}

/// Stream format plus the channel each mono 16 kHz chunk is sent on.
struct ChunkSink {
    config: StreamConfig,
    tx: Sender<AudioChunk>,
}

impl ChunkSink {
    /// Interleaved channel count, never 0 so frame chunking cannot panic.
    fn channels(&self) -> usize {
        usize::from(self.config.channels).max(1)
    }
}

/// Builds one input stream for any cpal sample type. The callback owns a reusable mono scratch
/// buffer; the error callback flags fatal errors for `LiveAudioCapture::has_failed`.
fn build_stream<T>(
    device: &Device,
    sink: ChunkSink,
    stream_failed: Arc<AtomicBool>,
) -> Result<Stream, cpal::Error>
where
    T: SizedSample,
    f32: FromSample<T>,
{
    let config = sink.config;
    let mut scratch = Vec::new();
    device.build_input_stream(
        config,
        move |data: &[T], _: &InputCallbackInfo| push_mono_chunk(data, &mut scratch, &sink),
        move |err: cpal::Error| {
            if is_fatal_stream_error(err.kind()) {
                stream_failed.store(true, Ordering::Relaxed);
            }
        },
        None,
    )
}

/// Glitches (`Xrun`), automatic default-device rerouting and refused realtime priority leave the
/// stream delivering audio; every other cpal error means the capture is gone.
fn is_fatal_stream_error(kind: ErrorKind) -> bool {
    !matches!(
        kind,
        ErrorKind::Xrun | ErrorKind::DeviceChanged | ErrorKind::RealtimeDenied
    )
}

/// Mixes interleaved `data` down to mono in `scratch` (cleared, capacity kept) and sends one
/// 16 kHz chunk. Only the outgoing `samples_16k` allocates; a trailing partial frame is ignored.
fn push_mono_chunk<T>(data: &[T], scratch: &mut Vec<f32>, sink: &ChunkSink)
where
    T: Sample,
    f32: FromSample<T>,
{
    let channels = sink.channels();
    scratch.clear();
    scratch
        .extend(data.chunks_exact(channels).map(|frame| {
            frame.iter().map(|s| s.to_sample::<f32>()).sum::<f32>() / channels as f32
        }));
    if scratch.is_empty() {
        return;
    }

    let sum_sq: f32 = scratch.iter().map(|s| s * s).sum();
    let rms = (sum_sq / scratch.len() as f32).sqrt();
    let _ = sink.tx.try_send(AudioChunk {
        samples_16k: downsample_to_16k_i16(scratch, sink.config.sample_rate),
        rms_level: (rms * RMS_METER_GAIN).clamp(0.0, 1.0),
        voice_active: rms >= VOICE_ACTIVITY_RMS,
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

#[cfg(test)]
mod tests {
    use async_channel::{Receiver, bounded};
    use cpal::BufferSize;

    use super::*;

    fn sink(channels: u16) -> (ChunkSink, Receiver<AudioChunk>) {
        let (tx, rx) = bounded(4);
        let config = StreamConfig {
            channels,
            sample_rate: TARGET_SAMPLE_RATE_HZ,
            buffer_size: BufferSize::Default,
        };
        (ChunkSink { config, tx }, rx)
    }

    /// Interleaved 8-channel frames alternating 0.5 / 0.1 (mono average 0.3).
    fn alternating(len: usize) -> Vec<f32> {
        (0..len)
            .map(|i| if i % 2 == 0 { 0.5 } else { 0.1 })
            .collect()
    }

    #[test]
    fn eight_channel_audio_mixes_to_mono_and_reuses_the_scratch_buffer() {
        let (sink, rx) = sink(8);
        let mut scratch = Vec::with_capacity(16);
        let buffer = scratch.as_ptr();

        // Three whole frames plus a 5-sample partial frame (an odd sample count).
        push_mono_chunk(&alternating(29), &mut scratch, &sink);
        let chunk = rx.try_recv().expect("one chunk per callback");
        assert_eq!(scratch.len(), 3, "the partial trailing frame is ignored");
        assert!(scratch.iter().all(|s| (s - 0.3).abs() < 1e-6));
        assert_eq!(chunk.samples_16k.len(), 3);
        assert!(chunk.voice_active);

        push_mono_chunk(&alternating(16), &mut scratch, &sink);
        assert!(rx.try_recv().is_ok());
        assert_eq!(scratch.len(), 2, "the buffer is cleared, not appended to");
        assert_eq!(
            scratch.as_ptr(),
            buffer,
            "the callback must not reallocate its scratch buffer"
        );
    }

    #[test]
    fn fewer_samples_than_channels_sends_nothing() {
        let (sink, rx) = sink(8);
        let mut scratch = vec![1.0; 4];
        push_mono_chunk(&alternating(5), &mut scratch, &sink);
        assert!(scratch.is_empty());
        assert!(rx.try_recv().is_err(), "no whole frame, no chunk");
    }

    #[test]
    fn a_zero_channel_config_is_treated_as_mono() {
        let (sink, rx) = sink(0);
        let mut scratch = Vec::new();
        push_mono_chunk(&[0i16, i16::MAX], &mut scratch, &sink);
        let chunk = rx.try_recv().expect("mono chunk");
        assert_eq!(chunk.samples_16k.len(), 2);
        assert!(chunk.rms_level > 0.0);
    }

    #[test]
    fn only_stream_ending_cpal_errors_drop_the_capture() {
        for kind in [
            ErrorKind::Xrun,
            ErrorKind::DeviceChanged,
            ErrorKind::RealtimeDenied,
        ] {
            assert!(
                !is_fatal_stream_error(kind),
                "{kind:?} leaves the stream delivering audio"
            );
        }
        for kind in [
            ErrorKind::DeviceNotAvailable,
            ErrorKind::StreamInvalidated,
            ErrorKind::PermissionDenied,
            ErrorKind::DeviceBusy,
            ErrorKind::BackendError,
        ] {
            assert!(is_fatal_stream_error(kind), "{kind:?} ends the capture");
        }
    }
}
