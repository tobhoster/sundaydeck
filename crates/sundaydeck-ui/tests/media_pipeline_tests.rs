//! Media pipeline regressions (plan WP3, AC1.4, C11): bounded and tiered decodes, and a cache
//! that never decodes on the calling (UI) thread. Prints `PERF` decode metrics for AC3.
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use gpui::RenderImage;
use gpui_screenshot::ScreenshotApp;
use sundaydeck_ui::media_cache::{background_image, video_keyframes};
use sundaydeck_ui::media_decode::{ImageTier, decode_background};

const EMBEDDED_1080P: &str = "backgrounds/sanctuary-mountains.jpg";
const BUILTIN_GOLDEN_LOOP: &str = "video/golden-light-loop.mp4";
/// 2 × the 256×144 preview tile, for Retina.
const THUMBNAIL_BOX: (i32, i32) = (512, 288);

fn dims(image: &RenderImage) -> (i32, i32) {
    let size = image.size(0);
    (size.width.0, size.height.0)
}

fn resident_bytes(image: &RenderImage) -> usize {
    image.as_bytes(0).map_or(0, <[u8]>::len)
}

/// Per-test scratch directory under the system temp dir, removed on drop.
struct Scratch(PathBuf);

impl Scratch {
    fn new(test: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "sundaydeck-media-tests-{}-{test}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).expect("create scratch dir");
        Self(dir)
    }

    fn png(&self, name: &str, width: u32, height: u32) -> String {
        let path = self.0.join(name);
        image::RgbaImage::from_pixel(width, height, image::Rgba([200, 160, 90, 255]))
            .save(&path)
            .expect("write png fixture");
        path.to_string_lossy().into_owned()
    }

    fn file(&self, name: &str, bytes: &[u8]) -> String {
        let path = self.0.join(name);
        std::fs::write(&path, bytes).expect("write fixture");
        path.to_string_lossy().into_owned()
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn decodes_are_downscaled_to_their_tier_with_cover_semantics() {
    let scratch = Scratch::new("tiers");
    let wide = scratch.png("wide.png", 4000, 1000);
    let small = scratch.png("small.png", 300, 200);
    let cases = [
        (EMBEDDED_1080P, ImageTier::Thumbnail, (512, 288)),
        (EMBEDDED_1080P, ImageTier::Projection, (1920, 1080)),
        // Cover: the thumbnail keeps 288 px height so a 16:9 tile is still fully covered.
        (wide.as_str(), ImageTier::Thumbnail, (1152, 288)),
        (wide.as_str(), ImageTier::Projection, (4000, 1000)),
        // Never upscale.
        (small.as_str(), ImageTier::Thumbnail, (300, 200)),
        (small.as_str(), ImageTier::Projection, (300, 200)),
    ];
    for (uri, tier, (w, h)) in cases {
        let image = decode_background(uri, tier).expect("decodable fixture");
        let (got_w, got_h) = dims(&image);
        assert!(
            (got_w - w).abs() <= 1 && (got_h - h).abs() <= 1,
            "{uri} at {tier:?}: got {got_w}×{got_h}, expected {w}×{h}"
        );
        assert_eq!(resident_bytes(&image), (got_w * got_h * 4) as usize);
    }
}

#[test]
fn oversized_corrupt_and_missing_images_yield_none() {
    let scratch = Scratch::new("reject");
    let too_wide = scratch.png("too-wide.png", 16_385, 1);
    let too_tall = scratch.png("too-tall.png", 1, 16_385);
    let truncated_png = {
        let mut bytes = std::fs::read(scratch.png("ok.png", 64, 64)).expect("read png");
        bytes.truncate(bytes.len() / 2);
        scratch.file("truncated.png", &bytes)
    };
    let garbage = scratch.file("garbage.jpg", b"definitely not a jpeg");
    let empty = scratch.file("empty.png", b"");
    let missing = scratch.0.join("missing.jpg").to_string_lossy().into_owned();
    let uris = [
        too_wide.as_str(),
        too_tall.as_str(),
        truncated_png.as_str(),
        garbage.as_str(),
        empty.as_str(),
        missing.as_str(),
        "backgrounds/does-not-exist.jpg",
        "",
    ];
    for uri in uris {
        for tier in [ImageTier::Thumbnail, ImageTier::Projection] {
            assert!(
                decode_background(uri, tier).is_none(),
                "{uri:?} at {tier:?} must be rejected"
            );
        }
    }
}

#[test]
fn cache_never_decodes_on_the_calling_thread_and_serves_warm_hits() {
    let mut app = ScreenshotApp::new().expect("headless app");
    for tier in [ImageTier::Thumbnail, ImageTier::Projection] {
        let first = app.update(|cx| background_image(EMBEDDED_1080P, tier, cx));
        assert!(
            first.is_none(),
            "{tier:?}: an uncached image must be decoded in the background, not inline"
        );
        app.run_until_parked();
        let ready = app
            .update(|cx| background_image(EMBEDDED_1080P, tier, cx))
            .expect("decode finished on the background executor");
        let again = app
            .update(|cx| background_image(EMBEDDED_1080P, tier, cx))
            .expect("warm hit");
        assert!(
            Arc::ptr_eq(&ready, &again),
            "{tier:?}: warm hits reuse the decode"
        );
        if matches!(tier, ImageTier::Thumbnail) {
            let (w, h) = dims(&ready);
            assert!(
                w <= THUMBNAIL_BOX.0 && h <= THUMBNAIL_BOX.1,
                "thumbnail {w}×{h}"
            );
        }
    }

    let missing = "/nonexistent/sundaydeck/never.jpg";
    assert!(
        app.update(|cx| background_image(missing, ImageTier::Thumbnail, cx))
            .is_none()
    );
    app.run_until_parked();
    assert!(
        app.update(|cx| background_image(missing, ImageTier::Thumbnail, cx))
            .is_none(),
        "a failed decode stays None"
    );
}

#[test]
fn builtin_motion_loop_keyframes_fit_their_tier() {
    let mut app = ScreenshotApp::new().expect("headless app");
    let cases = [
        (ImageTier::Thumbnail, THUMBNAIL_BOX, 3_538_944usize),
        (ImageTier::Projection, (1280, 720), 22_118_400usize),
    ];
    for (tier, (max_w, max_h), max_total) in cases {
        let _ = app.update(|cx| video_keyframes(BUILTIN_GOLDEN_LOOP, tier, cx));
        app.run_until_parked();
        let frames = app
            .update(|cx| video_keyframes(BUILTIN_GOLDEN_LOOP, tier, cx))
            .expect("builtin loop keyframes decoded");
        assert_eq!(frames.len(), 6, "{tier:?}: builtin loops ship 6 keyframes");
        let mut total = 0;
        for frame in frames.iter() {
            let (w, h) = dims(frame);
            assert!(w <= max_w && h <= max_h, "{tier:?} keyframe {w}×{h}");
            total += resident_bytes(frame);
        }
        println!("PERF video_loop_resident_bytes_{tier:?}={total}");
        assert!(total <= max_total, "{tier:?}: {total} resident bytes");
    }
}

#[test]
fn decode_cost_metrics() {
    let t0 = Instant::now();
    let projection = decode_background(EMBEDDED_1080P, ImageTier::Projection).expect("decode");
    let cold_ms = t0.elapsed().as_secs_f64() * 1000.0;
    let t1 = Instant::now();
    let thumbnail = decode_background(EMBEDDED_1080P, ImageTier::Thumbnail).expect("decode");
    let thumb_ms = t1.elapsed().as_secs_f64() * 1000.0;
    println!("PERF decode_cold_ms={cold_ms:.2}");
    println!("PERF decode_cold_thumbnail_ms={thumb_ms:.2}");
    println!("PERF tile_tier_image_bytes={}", resident_bytes(&thumbnail));
    assert_eq!(dims(&projection), (1920, 1080));
    assert!(resident_bytes(&thumbnail) <= 589_824);

    let mut app = ScreenshotApp::new().expect("headless app");
    let _ = app.update(|cx| background_image(EMBEDDED_1080P, ImageTier::Projection, cx));
    app.run_until_parked();
    let warm_ms = app.update(|cx| {
        let t = Instant::now();
        let hit = background_image(EMBEDDED_1080P, ImageTier::Projection, cx);
        let ms = t.elapsed().as_secs_f64() * 1000.0;
        assert!(hit.is_some(), "warm hit");
        ms
    });
    println!("PERF decode_warm_ms={warm_ms:.4}");
}
