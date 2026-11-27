use crate::custom_slide::{SlideTransition, chroma_key_green_theme};
use crate::ids::{MediaAssetId, SUNDAYDECK_NAMESPACE, ThemeId};
use crate::theme::{
    BackgroundSpec, HorizontalAlign, ImageFit, Rgba8, SafeMargins, TextShadow, Theme, ThemeTarget,
    VerticalAlign,
};

fn v5_media(tag: &str) -> MediaAssetId {
    MediaAssetId::from_v5(
        &SUNDAYDECK_NAMESPACE,
        format!("seed:media:{tag}").as_bytes(),
    )
}

fn v5_theme(tag: &str) -> ThemeId {
    ThemeId::from_v5(
        &SUNDAYDECK_NAMESPACE,
        format!("seed:theme:{tag}").as_bytes(),
    )
}

/// Curated Public Domain photography, looping motion video, and OBS chroma-key themes bundled offline.
pub fn media_and_broadcast_themes() -> Vec<Theme> {
    vec![
        Theme {
            id: v5_theme("sanctuary-mountains"),
            name: "Sanctuary Mountains".to_owned(),
            target: ThemeTarget::Song,
            is_default: false,
            background: BackgroundSpec::Image {
                asset_id: v5_media("sanctuary-mountains"),
                uri: Some("backgrounds/sanctuary-mountains.jpg".to_owned()),
                fit: ImageFit::Cover,
                scrim_opacity: 0.48,
            },
            font_family: "Manrope".to_owned(),
            font_size_pt: 76.0,
            font_weight: 800,
            line_height: 1.28,
            text_color: Rgba8::from_rgb(245, 243, 239),
            h_align: HorizontalAlign::Center,
            v_align: VerticalAlign::Middle,
            stroke_width_px: 2.0,
            stroke_color: Rgba8::from_rgba(0, 0, 0, 190),
            shadow: Some(TextShadow {
                offset_x_px: 0.0,
                offset_y_px: 6.0,
                blur_radius_px: 16.0,
                color: Rgba8::from_rgba(0, 0, 0, 210),
            }),
            margins: SafeMargins::DEFAULT,
            footer_font_size_pt: 22.0,
            transition_ms: 400,
            transition: SlideTransition::Crossfade { duration_ms: 400 },
        },
        Theme {
            id: v5_theme("golden-light-motion"),
            name: "Golden Light Motion".to_owned(),
            target: ThemeTarget::Song,
            is_default: false,
            background: BackgroundSpec::VideoLoop {
                asset_id: v5_media("golden-light-loop"),
                uri: "video/golden-light-loop.mp4".to_owned(),
                poster_uri: "backgrounds/golden-light.jpg".to_owned(),
                fps: 1.0,
                scrim_opacity: 0.46,
            },
            font_family: "Manrope".to_owned(),
            font_size_pt: 76.0,
            font_weight: 800,
            line_height: 1.28,
            text_color: Rgba8::from_rgb(247, 244, 238),
            h_align: HorizontalAlign::Center,
            v_align: VerticalAlign::Middle,
            stroke_width_px: 2.0,
            stroke_color: Rgba8::from_rgba(0, 0, 0, 190),
            shadow: Some(TextShadow {
                offset_x_px: 0.0,
                offset_y_px: 6.0,
                blur_radius_px: 16.0,
                color: Rgba8::from_rgba(0, 0, 0, 210),
            }),
            margins: SafeMargins::DEFAULT,
            footer_font_size_pt: 22.0,
            transition_ms: 450,
            transition: SlideTransition::SoftRise { duration_ms: 280 },
        },
        Theme {
            id: v5_theme("still-waters-scripture"),
            name: "Still Waters Scripture".to_owned(),
            target: ThemeTarget::Scripture,
            is_default: false,
            background: BackgroundSpec::Image {
                asset_id: v5_media("still-waters"),
                uri: Some("backgrounds/still-waters.jpg".to_owned()),
                fit: ImageFit::Cover,
                scrim_opacity: 0.54,
            },
            font_family: "Newsreader".to_owned(),
            font_size_pt: 64.0,
            font_weight: 400,
            line_height: 1.40,
            text_color: Rgba8::from_rgb(245, 243, 239),
            h_align: HorizontalAlign::Left,
            v_align: VerticalAlign::Middle,
            stroke_width_px: 0.0,
            stroke_color: Rgba8::TRANSPARENT,
            shadow: Some(TextShadow {
                offset_x_px: 0.0,
                offset_y_px: 4.0,
                blur_radius_px: 12.0,
                color: Rgba8::from_rgba(0, 0, 0, 180),
            }),
            margins: SafeMargins {
                top_pct: 0.09,
                right_pct: 0.10,
                bottom_pct: 0.11,
                left_pct: 0.10,
            },
            footer_font_size_pt: 24.0,
            transition_ms: 360,
            transition: SlideTransition::ZoomDissolve { duration_ms: 320 },
        },
        chroma_key_green_theme(),
    ]
}
