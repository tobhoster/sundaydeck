use gpui::{Div, FontWeight, ObjectFit, ParentElement, Styled, StyledImage, div, img, px, rgba};

use sundaydeck_core::{SlideTransition, default_custom_slide_templates};

use crate::design::gallery_console::section_frame;
use crate::design::icons::{DeckIcon, IconSize, deck_icon};
use crate::design::motion::MotionMode;
use crate::design::primitives::{
    VoiceSyncMode, display_status_pill, native_screen_key_body, primary_ivory_deck_key_body,
    render_auto_follow_pill, status_dot_label,
};
use crate::design::tokens::{
    FONT_BODY, FONT_DISPLAY, FONT_DISPLAY_PROJ, FONT_LYRIC_BOLD, FONT_SCRIPTURE_SERIF,
    RADIUS_CONTROL, RADIUS_TILE, TOKENS,
};
use crate::media::load_background_image;

pub(crate) fn render_switcher_and_voice_showcase(motion_mode: MotionMode) -> Div {
    let controls_col = div()
        .flex_1()
        .flex()
        .flex_col()
        .gap(px(11.0))
        .child(
            div()
                .font_family(FONT_BODY)
                .text_color(TOKENS.ink_muted)
                .text_size(px(11.5))
                .font_weight(FontWeight::MEDIUM)
                .child("Tactile Native Screen Controls (Ivory Primary · Sculpted Graphite · Latched Override)"),
        )
        .child(
            div()
                .w_full()
                .flex()
                .items_stretch()
                .gap(px(9.0))
                .child(div().flex_1().child(primary_ivory_deck_key_body(
                    DeckIcon::PlayFill,
                    "Show on Screen",
                )))
                .child(div().flex_1().child(native_screen_key_body(
                    DeckIcon::TextTSlash,
                    "Hide Words",
                    false,
                )))
                .child(div().flex_1().child(native_screen_key_body(
                    DeckIcon::EyeSlash,
                    "Black Screen",
                    true,
                )))
                .child(div().flex_1().child(native_screen_key_body(
                    DeckIcon::Image,
                    "Show Logo",
                    false,
                ))),
        );

    let status_col = div()
        .flex_1()
        .flex()
        .flex_col()
        .gap(px(11.0))
        .child(
            div()
                .font_family(FONT_BODY)
                .text_color(TOKENS.ink_muted)
                .text_size(px(11.5))
                .font_weight(FontWeight::MEDIUM)
                .child("Neutral Status Surface + Precision 6px Dots"),
        )
        .child(
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap(px(8.0))
                .child(render_auto_follow_pill(
                    "gal-voice-off",
                    VoiceSyncMode::Off,
                    0,
                    motion_mode,
                ))
                .child(render_auto_follow_pill(
                    "gal-voice-suggest",
                    VoiceSyncMode::Suggest,
                    2,
                    motion_mode,
                ))
                .child(render_auto_follow_pill(
                    "gal-voice-auto",
                    VoiceSyncMode::Auto,
                    3,
                    motion_mode,
                )),
        )
        .child(
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap(px(8.0))
                .child(display_status_pill(
                    DeckIcon::MonitorFill,
                    "Projector Ready",
                    true,
                    TOKENS.signal_voice,
                ))
                .child(display_status_pill(
                    DeckIcon::ChalkboardSimple,
                    "Stage Screen",
                    true,
                    TOKENS.signal_preview,
                ))
                .child(display_status_pill(
                    DeckIcon::Monitor,
                    "Projector Off",
                    false,
                    TOKENS.ink_muted,
                )),
        );

    section_frame(
        "Native Controls & Status Bar",
        "Zero flat pastel bricks or translucent pills — tactile physical contrast with calm semantics",
    )
    .child(
        div()
            .w_full()
            .flex()
            .gap(px(24.0))
            .child(controls_col)
            .child(div().w(px(1.0)).bg(TOKENS.hairline))
            .child(status_col),
    )
}

struct ProjectionCard<'a> {
    family: &'a str,
    role: &'a str,
    sample: &'a str,
    weight: FontWeight,
    footer: &'a str,
    bg_uri: Option<&'a str>,
}

pub(crate) fn render_projection_font_pack() -> Div {
    let cards: &[ProjectionCard<'_>] = &[
        ProjectionCard {
            family: FONT_LYRIC_BOLD,
            role: "Sanctuary Mountains · Photo + Manrope",
            sample: "Amazing grace! How sweet the sound\nThat saved a wretch like me!",
            weight: FontWeight::EXTRA_BOLD,
            footer: "John Newton · CCLI #22025",
            bg_uri: Some("backgrounds/sanctuary-mountains.jpg"),
        },
        ProjectionCard {
            family: FONT_SCRIPTURE_SERIF,
            role: "Still Waters · Photo + Newsreader",
            sample: "He makes me lie down in green pastures.\nHe leads me beside still waters.",
            weight: FontWeight::NORMAL,
            footer: "Psalm 23:2 · WEB",
            bg_uri: Some("backgrounds/still-waters.jpg"),
        },
        ProjectionCard {
            family: FONT_DISPLAY,
            role: "Golden Light Motion · Looping Video",
            sample: "When peace, like a river, attendeth my way,\nIt is well with my soul.",
            weight: FontWeight::SEMIBOLD,
            footer: "Horatio G. Spafford",
            bg_uri: Some("backgrounds/golden-loop-03.jpg"),
        },
        ProjectionCard {
            family: FONT_DISPLAY_PROJ,
            role: "Modern Display · Matte Obsidian + Syne",
            sample: "Sunday Worship\nGathering at 10:00 AM",
            weight: FontWeight::BOLD,
            footer: "Grace Fellowship",
            bg_uri: None,
        },
    ];

    let mut grid = div().w_full().flex().flex_wrap().gap(px(16.0));
    for card_spec in cards {
        let mut card = div()
            .relative()
            .w(px(330.0))
            .h(px(186.0)) // exact 16:9
            .bg(TOKENS.slide_card)
            .border_1()
            .border_color(TOKENS.gridline)
            .rounded(px(RADIUS_TILE))
            .overflow_hidden()
            .p(px(16.0))
            .flex()
            .flex_col()
            .justify_between();

        if let Some(uri) = card_spec.bg_uri
            && let Some(render_img) = load_background_image(uri)
        {
            card = card
                .child(
                    img(render_img)
                        .absolute()
                        .inset_0()
                        .size_full()
                        .object_fit(ObjectFit::Cover),
                )
                .child(div().absolute().inset_0().bg(rgba(0x0B0C_0E88)));
        }

        grid = grid.child(
            card.child(
                div()
                    .relative()
                    .font_family(FONT_BODY)
                    .text_color(TOKENS.ink_secondary)
                    .text_size(px(11.0))
                    .font_weight(FontWeight::MEDIUM)
                    .child(card_spec.role),
            )
            .child(
                div()
                    .relative()
                    .font_family(card_spec.family)
                    .font_weight(card_spec.weight)
                    .text_color(TOKENS.ink_primary)
                    .text_size(px(18.5))
                    .line_height(px(25.0))
                    .child(card_spec.sample),
            )
            .child(
                div()
                    .relative()
                    .font_family(FONT_BODY)
                    .text_color(TOKENS.ink_secondary)
                    .text_size(px(11.0))
                    .child(card_spec.footer),
            ),
        );
    }

    section_frame(
        "Sanctuary Projection Themes & Font Pack",
        "Bundled SIL OFL typefaces over Public Domain sanctuary photography and looping motion video backgrounds",
    )
    .child(grid)
}

pub(crate) fn render_custom_layouts_and_motions() -> Div {
    let mut motion_row = div().w_full().flex().flex_wrap().gap(px(8.0));
    for tr in SlideTransition::ALL_PRESETS {
        let timing = if tr.duration_ms() == 0 {
            "0ms".to_owned()
        } else {
            format!("{}ms", tr.duration_ms())
        };
        motion_row = motion_row.child(
            div()
                .px(px(12.0))
                .py(px(7.0))
                .rounded(px(RADIUS_CONTROL))
                .bg(TOKENS.pane)
                .border_1()
                .border_color(TOKENS.gridline)
                .flex()
                .items_center()
                .gap(px(8.0))
                .child(status_dot_label(
                    tr.label(),
                    if tr == SlideTransition::Instant {
                        TOKENS.ink_muted
                    } else {
                        TOKENS.signal_preview
                    },
                    TOKENS.ink_primary,
                ))
                .child(
                    div()
                        .font_family(crate::design::tokens::FONT_MONO)
                        .text_color(TOKENS.ink_muted)
                        .text_size(px(10.5))
                        .child(timing),
                ),
        );
    }

    let mut tpl_row = div().w_full().flex().flex_wrap().gap(px(10.0));
    for spec in default_custom_slide_templates() {
        tpl_row = tpl_row.child(
            div()
                .w(px(214.0))
                .p(px(10.0))
                .rounded(px(RADIUS_TILE))
                .bg(TOKENS.pane)
                .border_1()
                .border_color(TOKENS.hairline)
                .flex()
                .flex_col()
                .gap(px(5.0))
                .child(status_dot_label(
                    spec.layout.template_label(),
                    TOKENS.signal_voice,
                    TOKENS.ink_secondary,
                ))
                .child(
                    div()
                        .font_family(FONT_DISPLAY)
                        .text_color(TOKENS.ink_primary)
                        .text_size(px(12.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(spec.label),
                )
                .child(
                    div()
                        .font_family(FONT_BODY)
                        .text_color(TOKENS.ink_muted)
                        .text_size(px(10.5))
                        .child(
                            spec.transition_override
                                .unwrap_or(SlideTransition::DEFAULT_CROSSFADE)
                                .label(),
                        ),
                ),
        );
    }

    section_frame(
        "Structured Custom Slide Layouts & Layer-Aware Motions",
        "6 curated zero-clutter templates paired with 5 GPU-accelerated slide transitions",
    )
    .child(motion_row)
    .child(tpl_row)
}

pub(crate) fn render_icon_catalog() -> Div {
    let mut size_row = div()
        .flex()
        .items_center()
        .gap(px(20.0))
        .pb(px(12.0))
        .border_b_1()
        .border_color(TOKENS.hairline);

    for (label, sz) in [
        ("Small 14px (Inline)", IconSize::Sm),
        ("Medium 16px (Toolbar)", IconSize::Md),
        ("Large 18px (Actions)", IconSize::Lg),
    ] {
        size_row = size_row.child(
            div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .child(deck_icon(DeckIcon::Waveform, sz, TOKENS.signal_voice))
                .child(deck_icon(
                    DeckIcon::BroadcastFill,
                    sz,
                    TOKENS.signal_live_dot,
                ))
                .child(deck_icon(DeckIcon::BookOpenText, sz, TOKENS.signal_preview))
                .child(
                    div()
                        .font_family(FONT_BODY)
                        .text_color(TOKENS.ink_secondary)
                        .text_size(px(11.5))
                        .child(label),
                ),
        );
    }

    let mut swatches = div().w_full().flex().flex_wrap().gap(px(8.0));
    for &icon in DeckIcon::ALL {
        let tint = if icon.short_name().ends_with('*') {
            TOKENS.signal_preview
        } else {
            TOKENS.ink_primary
        };
        swatches = swatches.child(
            div()
                .w(px(114.0))
                .px(px(9.0))
                .py(px(8.0))
                .bg(TOKENS.pane)
                .border_1()
                .border_color(TOKENS.hairline)
                .rounded(px(RADIUS_CONTROL))
                .flex()
                .items_center()
                .gap(px(8.0))
                .child(deck_icon(icon, IconSize::Md, tint))
                .child(
                    div()
                        .font_family(FONT_BODY)
                        .text_color(TOKENS.ink_secondary)
                        .text_size(px(10.5))
                        .child(icon.short_name()),
                ),
        );
    }

    section_frame(
        "Embedded Phosphor Icon Catalog",
        "49 offline-safe SVG icons bundled directly in the binary — zero emojis",
    )
    .child(size_row)
    .child(swatches)
}
