use gpui::{
    Div, FontWeight, ObjectFit, ParentElement, Rgba, Styled, StyledImage, div, img, px, rgba,
};
use sundaydeck_core::SectionKind;

use crate::design::primitives::{
    keycap, live_meta_indicator, next_up_caption, section_meta_label, status_dot_label,
};
use crate::design::tokens::{
    FONT_BODY, FONT_DISPLAY, FONT_LYRIC_BOLD, FONT_MONO, FONT_SCRIPTURE_SERIF, RADIUS_TILE, TOKENS,
};
use crate::media::load_background_image;

/// Open, borderless gallery section header — generous breathing room with zero box-inside-box clutter.
pub(crate) fn section_frame(title: &str, subtitle: &str) -> Div {
    div().w_full().flex().flex_col().gap(px(14.0)).child(
        div()
            .flex()
            .items_baseline()
            .justify_between()
            .gap(px(12.0))
            .child(
                div()
                    .font_family(FONT_DISPLAY)
                    .text_color(TOKENS.ink_primary)
                    .text_size(px(15.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(title.to_owned()),
            )
            .child(
                div()
                    .font_family(FONT_BODY)
                    .text_color(TOKENS.ink_muted)
                    .text_size(px(11.5))
                    .child(subtitle.to_owned()),
            ),
    )
}

pub(crate) fn render_color_swatches() -> Div {
    let entries: &[(&str, &str, &str, Rgba)] = &[
        ("Canvas", "#141517", "Warm linen", TOKENS.canvas),
        ("Panel", "#1A1C20", "Sidebar & rail", TOKENS.pane),
        ("Raised", "#22252B", "Elevated key", TOKENS.elevated),
        ("Latched", "#15171C", "Engaged slot", TOKENS.recessed),
        ("Status", "#1C1F26", "Neutral pill", TOKENS.status_surface),
        ("Ivory", "#F2EFE9", "Primary action", TOKENS.ivory_bg),
        (
            "Secondary",
            "#B4B8C2",
            "Metadata text",
            TOKENS.ink_secondary,
        ),
        ("Muted", "#6E737E", "Quiet caption", TOKENS.ink_muted),
        ("Live Ring", "#C94A44", "On screen now", TOKENS.signal_live),
        ("Next Up", "#D9A036", "Staged ring", TOKENS.signal_preview),
        ("Listening", "#38B780", "Ready dot", TOKENS.signal_voice),
        (
            "Override",
            "#D97736",
            "Latched ring",
            TOKENS.signal_override,
        ),
    ];

    let mut row = div().w_full().flex().flex_wrap().gap(px(10.0));
    for &(name, hex, role, swatch) in entries {
        row = row.child(
            div()
                .w(px(106.0))
                .bg(TOKENS.pane)
                .border_1()
                .border_color(TOKENS.hairline)
                .rounded(px(RADIUS_TILE))
                .overflow_hidden()
                .flex()
                .flex_col()
                .child(
                    div()
                        .w_full()
                        .h(px(28.0))
                        .bg(swatch)
                        .border_b_1()
                        .border_color(TOKENS.hairline),
                )
                .child(
                    div()
                        .px(px(8.0))
                        .py(px(7.0))
                        .flex()
                        .flex_col()
                        .gap(px(1.5))
                        .child(
                            div()
                                .font_family(FONT_BODY)
                                .text_color(TOKENS.ink_primary)
                                .text_size(px(11.0))
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(name),
                        )
                        .child(
                            div()
                                .font_family(FONT_BODY)
                                .text_color(TOKENS.ink_muted)
                                .text_size(px(9.5))
                                .child(role),
                        )
                        .child(
                            div()
                                .font_family(FONT_MONO)
                                .text_color(TOKENS.ink_secondary)
                                .text_size(px(9.5))
                                .child(hex),
                        ),
                ),
        );
    }

    section_frame(
        "Warm Dark Linen Palette",
        "Quiet charcoal surfaces with reduced blue cast — color reserved strictly for precision dots and focus rings",
    )
    .child(row)
}

pub(crate) fn render_operator_typography() -> Div {
    let type_col = |label: &'static str| {
        div().flex_1().flex().flex_col().gap(px(9.0)).child(
            div()
                .font_family(FONT_BODY)
                .text_color(TOKENS.ink_muted)
                .text_size(px(11.0))
                .font_weight(FontWeight::MEDIUM)
                .child(label),
        )
    };

    let col_ui = type_col("Primary UI — Plus Jakarta Sans")
        .child(
            div()
                .font_family(FONT_DISPLAY)
                .text_color(TOKENS.ink_primary)
                .text_size(px(19.0))
                .font_weight(FontWeight::SEMIBOLD)
                .child("Sunday Morning · Nov 23"),
        )
        .child(
            div()
                .font_family(FONT_BODY)
                .text_color(TOKENS.ink_secondary)
                .text_size(px(13.0))
                .child("Service Order · Amazing Grace · Romans 8:28 · Hide Words"),
        );

    let col_meta = type_col("Typographic Section Metadata (Zero Pill Bubbles)")
        .child(
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap(px(14.0))
                .child(section_meta_label("Verse 1", Some(SectionKind::Verse)))
                .child(section_meta_label("Chorus", Some(SectionKind::Chorus)))
                .child(section_meta_label("Bridge", Some(SectionKind::Bridge)))
                .child(section_meta_label("Romans 8:28", None))
                .child(live_meta_indicator())
                .child(status_dot_label(
                    "Next Up",
                    TOKENS.signal_preview,
                    TOKENS.ink_secondary,
                )),
        )
        .child(
            div()
                .font_family(FONT_BODY)
                .text_color(TOKENS.ink_muted)
                .text_size(px(11.5))
                .child("Crisp 6px colored dots paired with plain SemiBold labels — never translucent bubble pills."),
        );

    let col_timers = type_col("Monospace — Clocks, Timers & Tooltip Hints Only")
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(16.0))
                .font_family(FONT_MONO)
                .text_size(px(14.0))
                .font_weight(FontWeight::SEMIBOLD)
                .child(div().text_color(TOKENS.ink_primary).child("10:15 AM"))
                .child(div().text_color(TOKENS.signal_preview).child("04:32"))
                .child(div().text_color(TOKENS.signal_live_dot).child("-00:18")),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .child(
                    div()
                        .font_family(FONT_BODY)
                        .text_color(TOKENS.ink_muted)
                        .text_size(px(11.0))
                        .child("Shortcuts:"),
                )
                .child(keycap("Space"))
                .child(keycap("F1"))
                .child(keycap("F2"))
                .child(keycap("F3")),
        );

    let v_rule = || div().w(px(1.0)).bg(TOKENS.hairline);

    section_frame(
        "Editorial Typography & Metadata",
        "Bundled SIL OFL fonts with quiet typographic cues",
    )
    .child(
        div()
            .w_full()
            .flex()
            .gap(px(22.0))
            .child(col_ui)
            .child(v_rule())
            .child(col_meta)
            .child(v_rule())
            .child(col_timers),
    )
}

#[derive(Clone, Copy)]
enum GallerySlideState {
    Default,
    NextUp,
    Live { progress: f32 },
}

pub(crate) fn render_slide_tile_showcase() -> Div {
    let tiles = div()
        .w_full()
        .flex()
        .flex_wrap()
        .gap(px(16.0))
        .child(mock_slide_tile(
            "Verse 1",
            Some(SectionKind::Verse),
            "Amazing grace! How sweet the sound\nThat saved a wretch like me!",
            FONT_LYRIC_BOLD,
            None,
            GallerySlideState::Default,
            None,
        ))
        .child(mock_slide_tile(
            "Chorus",
            Some(SectionKind::Chorus),
            "It is well with my soul,\nIt is well, it is well with my soul.",
            FONT_LYRIC_BOLD,
            None,
            GallerySlideState::NextUp,
            None,
        ))
        .child(mock_slide_tile(
            "Verse 1",
            Some(SectionKind::Verse),
            "When peace, like a river, attendeth my way,\nWhen sorrows like sea billows roll;",
            FONT_LYRIC_BOLD,
            None,
            GallerySlideState::Live { progress: 0.72 },
            None,
        ))
        .child(mock_slide_tile(
            "Bridge",
            Some(SectionKind::Bridge),
            "Praise the Lord, praise the Lord,\nO my soul!",
            FONT_LYRIC_BOLD,
            Some("backgrounds/sanctuary-mountains.jpg"),
            GallerySlideState::Default,
            Some("Photo"),
        ))
        .child(mock_slide_tile(
            "Romans 8:28",
            None,
            "We know that all things work together for good for those who love God.",
            FONT_SCRIPTURE_SERIF,
            Some("backgrounds/golden-loop-02.jpg"),
            GallerySlideState::Default,
            Some("Motion"),
        ));

    section_frame(
        "16:9 Sanctuary Slide Cards",
        "Soft matte cards, 1.5px amber Next Up ring, crimson Live ring with warm-ivory progress hairline, plus Unsplash photo & motion previews",
    )
    .child(tiles)
}

fn mock_slide_tile(
    label: &str,
    kind: Option<SectionKind>,
    text: &str,
    body_font: &str,
    bg_uri: Option<&str>,
    state: GallerySlideState,
    right_meta: Option<&str>,
) -> Div {
    let (border_color, border_w) = match state {
        GallerySlideState::Default => (TOKENS.gridline, 1.0),
        GallerySlideState::NextUp => (TOKENS.signal_preview, 1.5),
        GallerySlideState::Live { .. } => (TOKENS.signal_live, 1.5),
    };

    let mut tile = div()
        .relative()
        .w(px(256.0))
        .h(px(144.0)) // exact 16:9
        .rounded(px(RADIUS_TILE))
        .overflow_hidden()
        .bg(TOKENS.slide_card)
        .border(px(border_w))
        .border_color(border_color)
        .flex()
        .flex_col();

    if let Some(uri) = bg_uri
        && let Some(render_img) = load_background_image(uri)
    {
        tile = tile
            .child(
                img(render_img)
                    .absolute()
                    .inset_0()
                    .size_full()
                    .object_fit(ObjectFit::Cover),
            )
            .child(div().absolute().inset_0().bg(rgba(0x0B0C_0E8C)));
    }

    let right_header = match state {
        GallerySlideState::Live { .. } => live_meta_indicator(),
        _ => match right_meta {
            Some(meta) => div()
                .font_family(FONT_BODY)
                .text_color(TOKENS.ink_secondary)
                .text_size(px(10.0))
                .font_weight(FontWeight::MEDIUM)
                .child(meta.to_owned()),
            None => div(),
        },
    };

    tile = tile
        .child(
            div()
                .relative()
                .w_full()
                .px(px(11.0))
                .pt(px(8.0))
                .pb(px(3.0))
                .flex()
                .items_center()
                .justify_between()
                .child(section_meta_label(label, kind))
                .child(right_header),
        )
        .child(
            div()
                .relative()
                .flex_1()
                .w_full()
                .min_w_0()
                .px(px(14.0))
                .py(px(6.0))
                .overflow_hidden()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .w_full()
                        .text_center()
                        .font_family(body_font.to_owned())
                        .text_color(TOKENS.ink_primary)
                        .text_size(px(12.5))
                        .line_height(px(17.5))
                        .child(text.to_owned()),
                ),
        );

    match state {
        GallerySlideState::NextUp => {
            tile = tile
                .child(
                    div()
                        .relative()
                        .w_full()
                        .pb(px(6.0))
                        .flex()
                        .justify_center()
                        .child(next_up_caption()),
                )
                .child(
                    div()
                        .relative()
                        .w_full()
                        .h(px(2.0))
                        .bg(rgba(0x0000_0040))
                        .child(div().w(px(148.0)).h_full().bg(TOKENS.signal_preview)),
                );
        }
        GallerySlideState::Live { progress } => {
            let fill_w = (254.0 * progress.clamp(0.0, 1.0)).round();
            tile = tile.child(
                div()
                    .relative()
                    .w_full()
                    .h(px(2.0))
                    .bg(rgba(0x0000_0040))
                    .child(div().w(px(fill_w)).h_full().bg(rgba(0xF5F3_EF8C))),
            );
        }
        GallerySlideState::Default => {}
    }

    tile
}
