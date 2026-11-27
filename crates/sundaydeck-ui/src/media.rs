use std::sync::Arc;

use gpui::{
    App, Div, IntoElement, ObjectFit, ParentElement, RenderImage, Styled, StyledImage, div, img,
    linear_color_stop, linear_gradient, rgb, rgba,
};
use sundaydeck_core::{BackgroundSpec, ImageFit};

use crate::design::tokens::{TOKENS, rgba8_to_gpui};
use crate::media_cache::{background_image, video_keyframes};
use crate::media_decode::ImageTier;

/// Slowest and fastest keyframe rate a looping video background cycles at.
const MIN_LOOP_FPS: f32 = 0.2;
const MAX_LOOP_FPS: f32 = 12.0;
/// Below this blend the incoming keyframe is invisible, so its layer is skipped.
const MIN_VISIBLE_BLEND: f32 = 0.01;
const MAX_SCRIM_OPACITY: f32 = 0.92;

/// What to paint behind a slide: the theme background, the clock driving motion loops, and the
/// decode tier matching the canvas size.
pub struct BackgroundPaint<'a> {
    pub spec: &'a BackgroundSpec,
    pub now_ms: i64,
    pub tier: ImageTier,
}

/// Applies solid/gradient/photo/looping-video background layers + contrast scrim to a `relative()`
/// container (`Audience Output`, `Live Monitor`, `16:9 Slide Cards`, and `Themes Drawer`).
/// Media that is still decoding paints the matte canvas (a motion loop shows its poster once
/// that is ready) and the window repaints when the decode lands.
pub fn apply_theme_background(container: Div, paint: BackgroundPaint<'_>, cx: &App) -> Div {
    match paint.spec {
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
            let photo = uri
                .as_deref()
                .and_then(|uri| background_image(uri, paint.tier, cx));
            let container = container.bg(TOKENS.canvas);
            let layered = match photo {
                Some(render_img) => container.child(full_bleed(render_img, map_image_fit(*fit))),
                None => container,
            };
            add_scrim(layered, *scrim_opacity)
        }
        BackgroundSpec::VideoLoop {
            uri,
            poster_uri,
            fps,
            scrim_opacity,
            ..
        } => {
            let keyframes =
                video_keyframes(uri, paint.tier, cx).filter(|frames| !frames.is_empty());
            let current = match keyframes {
                Some(frames) => loop_frames_at(&frames, *fps, paint.now_ms),
                None => background_image(poster_uri, paint.tier, cx).map(|poster| LoopFrame {
                    current: poster,
                    next: None,
                    blend: 0.0,
                }),
            };
            add_scrim(
                paint_loop_frames(container.bg(TOKENS.canvas), current),
                *scrim_opacity,
            )
        }
        BackgroundSpec::LiveCamera { scrim_opacity, .. } => {
            let container = container.bg(linear_gradient(
                160.0,
                linear_color_stop(rgb(0x161920), 0.0),
                linear_color_stop(rgb(0x0D0F13), 1.0),
            ));
            add_scrim(container, *scrim_opacity)
        }
    }
}

/// The keyframe a looping video background shows, the one fading in over it, and how far the
/// fade has progressed (`0.0..=1.0`).
struct LoopFrame {
    current: Arc<RenderImage>,
    next: Option<Arc<RenderImage>>,
    blend: f32,
}

/// Returns the [`LoopFrame`] of a looping video background at `now_ms` (`None` for no frames).
fn loop_frames_at(frames: &[Arc<RenderImage>], fps: f32, now_ms: i64) -> Option<LoopFrame> {
    if let [only] = frames {
        return Some(LoopFrame {
            current: Arc::clone(only),
            next: None,
            blend: 0.0,
        });
    }
    let rate = f64::from(fps.clamp(MIN_LOOP_FPS, MAX_LOOP_FPS));
    let seconds = (now_ms.max(0) as f64) / 1000.0;
    let pos = (seconds * rate) % (frames.len() as f64);
    let idx_a = (pos.floor() as usize) % frames.len().max(1);
    let idx_b = (idx_a + 1) % frames.len().max(1);
    let frac = (pos - pos.floor()).clamp(0.0, 1.0) as f32;

    Some(LoopFrame {
        current: Arc::clone(frames.get(idx_a)?),
        next: frames.get(idx_b).cloned(),
        blend: frac,
    })
}

fn paint_loop_frames(container: Div, frame: Option<LoopFrame>) -> Div {
    let Some(LoopFrame {
        current,
        next,
        blend,
    }) = frame
    else {
        return container;
    };
    let container = container.child(full_bleed(current, ObjectFit::Cover));
    match next {
        Some(next_frame) if blend > MIN_VISIBLE_BLEND => container.child(
            div()
                .absolute()
                .inset_0()
                .size_full()
                .opacity(blend)
                .child(img(next_frame).size_full().object_fit(ObjectFit::Cover)),
        ),
        _ => container,
    }
}

fn full_bleed(render_img: Arc<RenderImage>, fit: ObjectFit) -> impl IntoElement {
    img(render_img)
        .absolute()
        .inset_0()
        .size_full()
        .object_fit(fit)
}

fn add_scrim(container: Div, scrim_opacity: f32) -> Div {
    if scrim_opacity > 0.0 {
        container.child(
            div()
                .absolute()
                .inset_0()
                .bg(rgba(0x0B0C_0EFF).alpha(scrim_opacity.clamp(0.0, MAX_SCRIM_OPACITY))),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media_decode::{BUILTIN_GOLDEN_LOOP_URI, decode_video_keyframes};

    #[test]
    fn motion_loop_blends_between_consecutive_keyframes() {
        let frames = decode_video_keyframes(BUILTIN_GOLDEN_LOOP_URI, ImageTier::Thumbnail);
        let LoopFrame {
            current: frame_a,
            next: frame_b,
            blend,
        } = loop_frames_at(&frames, 1.0, 1_450).expect("motion video loop decodes");
        assert!(blend > 0.4 && blend < 0.5);
        assert_ne!(frame_a.id, frame_b.expect("next frame").id);
    }
}
