//! Bounded, downscaled, synchronous decoding of background images and video keyframes.
//!
//! Pure functions with no GPUI state: they run on the GPUI background executor (via
//! `media_cache`) and in unit tests, never on the UI thread.

use std::borrow::Cow;
use std::io::Cursor;
use std::path::Path;
use std::process::Command;
use std::sync::Arc;

use gpui::RenderImage;
use image::{DynamicImage, Frame, ImageReader, Limits};
use rust_embed::RustEmbed;

use crate::design::tokens::{TILE_H_PX, TILE_W_PX};
use crate::media_keyframe_cache::keyframe_cache_dir;

/// Device-pixel ratio thumbnails are decoded for, so 16:9 tiles stay sharp on Retina displays.
const THUMBNAIL_PIXEL_RATIO: u32 = 2;
/// Box a thumbnail-tier image is scaled down to cover: the 256×144 slide tile at
/// [`THUMBNAIL_PIXEL_RATIO`] (physical pixels), so tiles stay sharp on Retina displays.
pub const THUMBNAIL_MAX: (u32, u32) = (
    TILE_W_PX as u32 * THUMBNAIL_PIXEL_RATIO,
    TILE_H_PX as u32 * THUMBNAIL_PIXEL_RATIO,
);
/// Box a projection-tier image is scaled down to cover (1440p projectors and Retina monitors).
pub const PROJECTION_MAX: (u32, u32) = (2560, 1440);
/// Images wider or taller than this are rejected before any pixel buffer is allocated.
pub const MAX_DECODE_DIMENSION: u32 = 16_384;
/// Strict cap on the decoder's allocations; fits a 48 MP phone photo, rejects 100 MP+ decodes.
pub const MAX_DECODE_ALLOC_BYTES: u64 = 256 * 1024 * 1024;
/// Bundled emblem shown on the logo splash when the church has no logo of its own.
pub const DEFAULT_CHURCH_LOGO_URI: &str = "backgrounds/default-church-logo.png";

pub const BUILTIN_GOLDEN_LOOP_URI: &str = "video/golden-light-loop.mp4";
const BUILTIN_GOLDEN_LOOP_FRAMES: &[&str] = &[
    "backgrounds/golden-loop-01.jpg",
    "backgrounds/golden-loop-02.jpg",
    "backgrounds/golden-loop-03.jpg",
    "backgrounds/golden-loop-04.jpg",
    "backgrounds/golden-loop-05.jpg",
    "backgrounds/golden-loop-06.jpg",
];

pub const BUILTIN_STARRY_LOOP_URI: &str = "video/starry-clouds-loop.mp4";
const BUILTIN_STARRY_LOOP_FRAMES: &[&str] = &[
    "backgrounds/stars-loop-01.jpg",
    "backgrounds/stars-loop-02.jpg",
    "backgrounds/stars-loop-03.jpg",
    "backgrounds/stars-loop-04.jpg",
    "backgrounds/stars-loop-05.jpg",
    "backgrounds/stars-loop-06.jpg",
];

const EMBEDDED_BACKGROUND_PREFIX: &str = "backgrounds/";
/// Keyframes `ffmpeg` samples from an external video (one per second).
const MAX_VIDEO_KEYFRAMES: usize = 12;
const KEYFRAME_FILTER: &str =
    "fps=1,scale=1280:720:force_original_aspect_ratio=increase,crop=1280:720";
const KEYFRAME_JPEG_QUALITY: &str = "4";

#[derive(RustEmbed)]
#[folder = "assets/backgrounds"]
struct EmbeddedBackgrounds;

/// Size class an image is decoded for: small slide tiles or full projector canvases.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ImageTier {
    Thumbnail,
    Projection,
}

impl ImageTier {
    /// Thumbnail when the canvas fits the thumbnail box at [`THUMBNAIL_PIXEL_RATIO`] (i.e. up to
    /// the tile width), otherwise Projection.
    pub fn for_canvas_width(width_px: f32) -> Self {
        if width_px * THUMBNAIL_PIXEL_RATIO as f32 <= THUMBNAIL_MAX.0 as f32 {
            Self::Thumbnail
        } else {
            Self::Projection
        }
    }

    /// The `(width, height)` box a decoded image of this tier is scaled down to cover.
    pub fn max_size(self) -> (u32, u32) {
        match self {
            Self::Thumbnail => THUMBNAIL_MAX,
            Self::Projection => PROJECTION_MAX,
        }
    }
}

/// Decodes an embedded sanctuary asset (e.g. `"backgrounds/sanctuary-mountains.jpg"`) or a local
/// file into a BGRA `RenderImage`, bounded by [`MAX_DECODE_DIMENSION`] and
/// [`MAX_DECODE_ALLOC_BYTES`] and downscaled to cover the tier's size.
pub fn decode_background(uri: &str, tier: ImageTier) -> Option<Arc<RenderImage>> {
    let bytes = read_background_bytes(uri)?;
    decode_bytes(&bytes, tier)
}

/// Keyframes for a looping video background: the embedded frames of a builtin motion loop,
/// otherwise frames extracted from the external video file at `uri`.
pub fn decode_video_keyframes(uri: &str, tier: ImageTier) -> Vec<Arc<RenderImage>> {
    match builtin_loop_frames(uri) {
        Some(frame_uris) => frame_uris
            .iter()
            .filter_map(|frame_uri| decode_background(frame_uri, tier))
            .collect(),
        None => extract_video_keyframes(Path::new(uri), tier),
    }
}

/// Samples up to [`MAX_VIDEO_KEYFRAMES`] keyframes from an external video with `ffmpeg` (cached
/// as JPEGs in the private media store) and decodes them for `tier`. Empty when the file or
/// `ffmpeg` is missing, or the cache directory is untrusted.
pub fn extract_video_keyframes(path: &Path, tier: ImageTier) -> Vec<Arc<RenderImage>> {
    let Some(cache_dir) = keyframe_cache_dir(path) else {
        return Vec::new();
    };
    if !cache_dir.join(keyframe_file_name(1)).exists() && run_ffmpeg(path, &cache_dir).is_err() {
        return Vec::new();
    }
    (1..=MAX_VIDEO_KEYFRAMES)
        .map(|index| cache_dir.join(keyframe_file_name(index)))
        .take_while(|frame_path| frame_path.is_file())
        .filter_map(|frame_path| decode_file(&frame_path, tier))
        .collect()
}

fn builtin_loop_frames(uri: &str) -> Option<&'static [&'static str]> {
    match uri {
        BUILTIN_GOLDEN_LOOP_URI => Some(BUILTIN_GOLDEN_LOOP_FRAMES),
        BUILTIN_STARRY_LOOP_URI => Some(BUILTIN_STARRY_LOOP_FRAMES),
        _ => None,
    }
}

fn read_background_bytes(uri: &str) -> Option<Cow<'static, [u8]>> {
    let embedded_key = uri.strip_prefix(EMBEDDED_BACKGROUND_PREFIX).unwrap_or(uri);
    if let Some(asset) = EmbeddedBackgrounds::get(embedded_key) {
        return Some(asset.data);
    }
    let path = Path::new(uri);
    if path.is_file() {
        return std::fs::read(path).ok().map(Cow::Owned);
    }
    None
}

/// `ffmpeg` output pattern for extracted keyframes; must agree with [`keyframe_file_name`].
const KEYFRAME_FFMPEG_PATTERN: &str = "frame-%02d.jpg";

fn keyframe_file_name(index: usize) -> String {
    format!("frame-{index:02}.jpg")
}

fn run_ffmpeg(video_path: &Path, cache_dir: &Path) -> std::io::Result<()> {
    let status = Command::new("ffmpeg")
        .args(["-nostdin", "-y", "-i"])
        .arg(video_path)
        .args(["-vf", KEYFRAME_FILTER, "-frames:v"])
        .arg(MAX_VIDEO_KEYFRAMES.to_string())
        .args(["-q:v", KEYFRAME_JPEG_QUALITY])
        .arg(cache_dir.join(KEYFRAME_FFMPEG_PATTERN))
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(std::io::Error::other(format!(
            "ffmpeg exited with {status}"
        )))
    }
}

fn decode_file(path: &Path, tier: ImageTier) -> Option<Arc<RenderImage>> {
    let bytes = std::fs::read(path).ok()?;
    decode_bytes(&bytes, tier)
}

fn decode_bytes(bytes: &[u8], tier: ImageTier) -> Option<Arc<RenderImage>> {
    let mut reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .ok()?;
    reader.limits(decode_limits());
    let decoded = reader.decode().ok()?;
    Some(into_render_image(cover_downscale(decoded, tier)))
}

fn decode_limits() -> Limits {
    let mut limits = Limits::default();
    limits.max_image_width = Some(MAX_DECODE_DIMENSION);
    limits.max_image_height = Some(MAX_DECODE_DIMENSION);
    limits.max_alloc = Some(MAX_DECODE_ALLOC_BYTES);
    limits
}

/// Shrinks `image` to the smallest size that still covers the tier's box; never upscales.
/// Integer area averaging works in 8-bit space (no float intermediate buffer) and is alias-free.
fn cover_downscale(image: DynamicImage, tier: ImageTier) -> DynamicImage {
    let (max_w, max_h) = tier.max_size();
    let (width, height) = (f64::from(image.width()), f64::from(image.height()));
    let scale = f64::max(f64::from(max_w) / width, f64::from(max_h) / height);
    if scale >= 1.0 {
        return image;
    }
    let target_w = ((width * scale).round() as u32).max(1);
    let target_h = ((height * scale).round() as u32).max(1);
    image.thumbnail_exact(target_w, target_h)
}

fn into_render_image(image: DynamicImage) -> Arc<RenderImage> {
    let mut bgra = image.into_rgba8();
    // GPUI's Vulkan/Metal RenderImage pipeline expects BGRA pixel order.
    for px in bgra.as_chunks_mut::<4>().0 {
        px.swap(0, 2);
    }
    Arc::new(RenderImage::new(vec![Frame::new(bgra)]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canvas_width_picks_thumbnail_only_up_to_tile_width() {
        assert_eq!(ImageTier::for_canvas_width(256.0), ImageTier::Thumbnail);
        assert_eq!(ImageTier::for_canvas_width(332.0), ImageTier::Projection);
        assert_eq!(ImageTier::for_canvas_width(513.0), ImageTier::Projection);
    }

    #[test]
    fn bundled_public_domain_and_motion_video_frames_decode_cleanly() {
        for uri in [
            "backgrounds/sanctuary-mountains.jpg",
            "backgrounds/golden-light.jpg",
            "backgrounds/still-waters.jpg",
            "backgrounds/starry-night-ridge.jpg",
            "backgrounds/autumn-forest-mist.jpg",
            "backgrounds/warm-dawn-shore.jpg",
            DEFAULT_CHURCH_LOGO_URI,
        ] {
            assert!(
                decode_background(uri, ImageTier::Projection).is_some(),
                "failed to decode embedded public-domain asset {uri}"
            );
        }

        for uri in [BUILTIN_GOLDEN_LOOP_URI, BUILTIN_STARRY_LOOP_URI] {
            assert_eq!(
                decode_video_keyframes(uri, ImageTier::Thumbnail).len(),
                BUILTIN_GOLDEN_LOOP_FRAMES.len(),
                "builtin motion loop {uri} decodes every embedded keyframe"
            );
        }
    }
}
