use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex, OnceLock};

use gpui::{
    Div, ObjectFit, ParentElement, RenderImage, Styled, StyledImage, div, img, linear_color_stop,
    linear_gradient, rgb, rgba,
};
use image::Frame;
use rust_embed::RustEmbed;
use sundaydeck_core::{BackgroundSpec, ImageFit};

use crate::design::tokens::{TOKENS, rgba8_to_gpui};

#[derive(RustEmbed)]
#[folder = "assets/backgrounds"]
struct EmbeddedBackgrounds;

const BUILTIN_GOLDEN_LOOP_URI: &str = "video/golden-light-loop.mp4";
const BUILTIN_GOLDEN_LOOP_FRAMES: &[&str] = &[
    "backgrounds/golden-loop-01.jpg",
    "backgrounds/golden-loop-02.jpg",
    "backgrounds/golden-loop-03.jpg",
    "backgrounds/golden-loop-04.jpg",
    "backgrounds/golden-loop-05.jpg",
    "backgrounds/golden-loop-06.jpg",
];

const BUILTIN_STARRY_LOOP_URI: &str = "video/starry-clouds-loop.mp4";
const BUILTIN_STARRY_LOOP_FRAMES: &[&str] = &[
    "backgrounds/stars-loop-01.jpg",
    "backgrounds/stars-loop-02.jpg",
    "backgrounds/stars-loop-03.jpg",
    "backgrounds/stars-loop-04.jpg",
    "backgrounds/stars-loop-05.jpg",
    "backgrounds/stars-loop-06.jpg",
];

fn image_cache() -> &'static Mutex<HashMap<String, Option<Arc<RenderImage>>>> {
    static CACHE: OnceLock<Mutex<HashMap<String, Option<Arc<RenderImage>>>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn external_video_frames_cache() -> &'static Mutex<HashMap<String, Vec<Arc<RenderImage>>>> {
    static CACHE: OnceLock<Mutex<HashMap<String, Vec<Arc<RenderImage>>>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Loads and caches a decoded BGRA `RenderImage` synchronously from embedded sanctuary assets
/// (e.g. `"backgrounds/sanctuary-mountains.jpg"`) or a local filesystem path.
pub fn load_background_image(uri: &str) -> Option<Arc<RenderImage>> {
    if let Ok(guard) = image_cache().lock()
        && let Some(cached) = guard.get(uri)
    {
        return cached.clone();
    }

    let decoded = decode_uri_to_render_image(uri);
    if let Ok(mut guard) = image_cache().lock() {
        guard.insert(uri.to_owned(), decoded.clone());
    }
    decoded
}

/// Returns `(current_frame, next_frame, blend_alpha)` for a looping video background at `now_ms`.
pub fn video_loop_frames(
    uri: &str,
    poster_uri: &str,
    fps: f32,
    now_ms: i64,
) -> Option<(Arc<RenderImage>, Option<Arc<RenderImage>>, f32)> {
    let frames = resolve_video_keyframes(uri);
    if frames.is_empty() {
        return load_background_image(poster_uri).map(|poster| (poster, None, 0.0));
    }
    if frames.len() == 1 {
        return Some((Arc::clone(&frames[0]), None, 0.0));
    }

    let rate = fps.clamp(0.2, 12.0) as f64;
    let seconds = (now_ms.max(0) as f64) / 1000.0;
    let pos = (seconds * rate) % (frames.len() as f64);
    let idx_a = (pos.floor() as usize) % frames.len();
    let idx_b = (idx_a + 1) % frames.len();
    let frac = (pos - pos.floor()).clamp(0.0, 1.0) as f32;

    Some((
        Arc::clone(&frames[idx_a]),
        Some(Arc::clone(&frames[idx_b])),
        frac,
    ))
}

/// Applies solid/gradient/photo/looping-video background layers + contrast scrim to a `relative()`
/// container (`Audience Output`, `Live Monitor`, `16:9 Slide Cards`, and `Themes Drawer`).
pub fn apply_theme_background(mut container: Div, spec: &BackgroundSpec, now_ms: i64) -> Div {
    match spec {
        BackgroundSpec::Solid { color } => container.bg(rgba8_to_gpui(*color)),
        BackgroundSpec::LinearGradient {
            start,
            end,
            angle_deg,
        } => container.bg(linear_gradient(
            *angle_deg,
            linear_color_stop(rgba8_to_gpui(*start), 0.0),
            linear_color_stop(rgba8_to_gpui(*end), 1.0),
        )),
        BackgroundSpec::Image {
            uri,
            fit,
            scrim_opacity,
            ..
        } => {
            container = container.bg(TOKENS.canvas);
            if let Some(uri_str) = uri.as_deref()
                && let Some(render_img) = load_background_image(uri_str)
            {
                container = container.child(
                    img(render_img)
                        .absolute()
                        .inset_0()
                        .size_full()
                        .object_fit(map_image_fit(*fit)),
                );
            }
            add_scrim(container, *scrim_opacity)
        }
        BackgroundSpec::VideoLoop {
            uri,
            poster_uri,
            fps,
            scrim_opacity,
            ..
        } => {
            container = container.bg(TOKENS.canvas);
            if let Some((frame_a, frame_b, blend)) =
                video_loop_frames(uri, poster_uri, *fps, now_ms)
            {
                container = container.child(
                    img(frame_a)
                        .absolute()
                        .inset_0()
                        .size_full()
                        .object_fit(ObjectFit::Cover),
                );
                if let Some(next_frame) = frame_b
                    && blend > 0.01
                {
                    container = container.child(
                        div()
                            .absolute()
                            .inset_0()
                            .size_full()
                            .opacity(blend)
                            .child(img(next_frame).size_full().object_fit(ObjectFit::Cover)),
                    );
                }
            }
            add_scrim(container, *scrim_opacity)
        }
        BackgroundSpec::LiveCamera { scrim_opacity, .. } => {
            container = container.bg(linear_gradient(
                160.0,
                linear_color_stop(rgb(0x161920), 0.0),
                linear_color_stop(rgb(0x0D0F13), 1.0),
            ));
            add_scrim(container, *scrim_opacity)
        }
    }
}

fn add_scrim(container: Div, scrim_opacity: f32) -> Div {
    if scrim_opacity > 0.0 {
        container.child(
            div()
                .absolute()
                .inset_0()
                .bg(rgba(0x0B0C_0EFF).alpha(scrim_opacity.clamp(0.0, 0.92))),
        )
    } else {
        container
    }
}

fn map_image_fit(fit: ImageFit) -> ObjectFit {
    match fit {
        ImageFit::Cover => ObjectFit::Cover,
        ImageFit::Contain => ObjectFit::Contain,
        ImageFit::Stretch => ObjectFit::Fill,
    }
}

fn resolve_video_keyframes(uri: &str) -> Vec<Arc<RenderImage>> {
    if uri == BUILTIN_GOLDEN_LOOP_URI {
        return BUILTIN_GOLDEN_LOOP_FRAMES
            .iter()
            .filter_map(|frame_uri| load_background_image(frame_uri))
            .collect();
    }
    if uri == BUILTIN_STARRY_LOOP_URI {
        return BUILTIN_STARRY_LOOP_FRAMES
            .iter()
            .filter_map(|frame_uri| load_background_image(frame_uri))
            .collect();
    }

    if let Ok(guard) = external_video_frames_cache().lock()
        && let Some(cached) = guard.get(uri)
    {
        return cached.clone();
    }

    let extracted = extract_external_video_frames(Path::new(uri));
    if let Ok(mut guard) = external_video_frames_cache().lock() {
        guard.insert(uri.to_owned(), extracted.clone());
    }
    extracted
}

fn extract_external_video_frames(video_path: &Path) -> Vec<Arc<RenderImage>> {
    if !video_path.is_file() {
        return Vec::new();
    }
    let key: u64 = video_path
        .to_string_lossy()
        .bytes()
        .fold(14_695_981_039_346_656_037u64, |acc, b| {
            acc.wrapping_mul(1_099_511_628_211) ^ u64::from(b)
        });
    let cache_dir: PathBuf = std::env::temp_dir()
        .join("sundaydeck-video-frames")
        .join(format!("{key:016x}"));
    if std::fs::create_dir_all(&cache_dir).is_err() {
        return Vec::new();
    }

    let pattern = cache_dir.join("frame-%02d.jpg");
    let first_frame = cache_dir.join("frame-01.jpg");
    if !first_frame.exists() {
        let status = Command::new("ffmpeg")
            .args(["-y", "-i"])
            .arg(video_path)
            .args([
                "-vf",
                "fps=1,scale=1280:720:force_original_aspect_ratio=increase,crop=1280:720",
                "-frames:v",
                "12",
                "-q:v",
                "4",
            ])
            .arg(&pattern)
            .status();
        if !status.is_ok_and(|s| s.success()) {
            return Vec::new();
        }
    }

    let mut out = Vec::new();
    for idx in 1..=12 {
        let path = cache_dir.join(format!("frame-{idx:02}.jpg"));
        if !path.is_file() {
            break;
        }
        if let Some(img) = decode_file_to_render_image(&path) {
            out.push(img);
        }
    }
    out
}

fn decode_uri_to_render_image(uri: &str) -> Option<Arc<RenderImage>> {
    let embedded_key = uri.strip_prefix("backgrounds/").unwrap_or(uri);
    if let Some(asset) = EmbeddedBackgrounds::get(embedded_key) {
        return decode_bytes_to_render_image(asset.data.as_ref());
    }
    let path = Path::new(uri);
    if path.is_file() {
        return decode_file_to_render_image(path);
    }
    None
}

fn decode_file_to_render_image(path: &Path) -> Option<Arc<RenderImage>> {
    let bytes = std::fs::read(path).ok()?;
    decode_bytes_to_render_image(&bytes)
}

fn decode_bytes_to_render_image(bytes: &[u8]) -> Option<Arc<RenderImage>> {
    let dyn_img = image::load_from_memory(bytes).ok()?;
    let mut bgra = dyn_img.into_rgba8();
    // GPUI's Vulkan/Metal RenderImage pipeline expects BGRA pixel order.
    for px in bgra.as_chunks_mut::<4>().0 {
        px.swap(0, 2);
    }
    Some(Arc::new(RenderImage::new(vec![Frame::new(bgra)])))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_public_domain_and_motion_video_frames_decode_cleanly() {
        for uri in [
            "backgrounds/sanctuary-mountains.jpg",
            "backgrounds/golden-light.jpg",
            "backgrounds/still-waters.jpg",
            "backgrounds/starry-night-ridge.jpg",
            "backgrounds/autumn-forest-mist.jpg",
            "backgrounds/warm-dawn-shore.jpg",
            "backgrounds/default-church-logo.png",
        ] {
            assert!(
                load_background_image(uri).is_some(),
                "failed to decode embedded public-domain asset {uri}"
            );
        }

        for (uri, poster) in [
            (BUILTIN_GOLDEN_LOOP_URI, "backgrounds/golden-light.jpg"),
            (
                BUILTIN_STARRY_LOOP_URI,
                "backgrounds/starry-night-ridge.jpg",
            ),
        ] {
            let (frame_a, frame_b, blend) =
                video_loop_frames(uri, poster, 1.0, 1_450).expect("motion video loop decodes");
            assert!(frame_b.is_some());
            assert!(blend > 0.4 && blend < 0.5);
            assert_ne!(frame_a.id, frame_b.expect("next frame").id);
        }
    }
}
