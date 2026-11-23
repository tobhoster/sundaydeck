use gpui::{Div, FontWeight, ParentElement, Styled, div, px, rgb};

use crate::design::gallery_console::section_frame;
use crate::design::icons::{DeckIcon, IconSize, deck_icon};
use crate::design::motion::MotionMode;
use crate::design::primitives::{
    VoiceSyncMode, display_status_pill, render_auto_follow_pill, volunteer_action_button_body,
};
use crate::design::tokens::{
    FONT_BODY, FONT_DISPLAY, FONT_DISPLAY_PROJ, FONT_LYRIC_BOLD, FONT_SCRIPTURE_SERIF,
    RADIUS_CONTROL, RADIUS_TILE, TOKENS,
};

pub(crate) fn render_switcher_and_voice_showcase(motion_mode: MotionMode) -> Div {
    let controls_col = div()
        .flex_1()
        .rounded(px(RADIUS_TILE))
        .bg(TOKENS.canvas)
        .border_1()
        .border_color(TOKENS.gridline)
        .p(px(12.0))
        .flex()
        .flex_col()
        .gap(px(10.0))
        .child(
            div()
                .font_family(FONT_BODY)
                .text_color(TOKENS.ink_muted)
                .text_size(px(11.5))
                .font_weight(FontWeight::MEDIUM)
                .child("Plain-English Screen Controls (shortcuts in hover tooltips)"),
        )
        .child(
            div()
                .w_full()
                .flex()
                .gap(px(8.0))
                .child(div().flex_1().child(volunteer_action_button_body(
                    DeckIcon::PlayFill,
                    "Show on Screen",
                    true,
                    TOKENS.signal_live,
                )))
                .child(div().flex_1().child(volunteer_action_button_body(
                    DeckIcon::TextTSlash,
                    "Hide Words",
                    false,
                    TOKENS.signal_override,
                )))
                .child(div().flex_1().child(volunteer_action_button_body(
                    DeckIcon::EyeSlash,
                    "Black Screen",
                    true,
                    TOKENS.signal_override,
                )))
                .child(div().flex_1().child(volunteer_action_button_body(
                    DeckIcon::Image,
                    "Show Logo",
                    false,
                    TOKENS.signal_override,
                ))),
        );

    let status_col = div()
        .flex_1()
        .rounded(px(RADIUS_TILE))
        .bg(TOKENS.canvas)
        .border_1()
        .border_color(TOKENS.gridline)
        .p(px(12.0))
        .flex()
        .flex_col()
        .gap(px(10.0))
        .child(
            div()
                .font_family(FONT_BODY)
                .text_color(TOKENS.ink_muted)
                .text_size(px(11.5))
                .font_weight(FontWeight::MEDIUM)
                .child("Calm Auto-Follow & Display Status Pills"),
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
        "Volunteer Controls & Status Pills",
        "Zero broadcast jargon — every button reads in clear, calm English",
    )
    .child(
        div()
            .w_full()
            .flex()
            .gap(px(14.0))
            .child(controls_col)
            .child(status_col),
    )
}

pub(crate) fn render_projection_font_pack() -> Div {
    let cards: &[(&str, &str, &str, FontWeight, &str, bool)] = &[
        (
            FONT_LYRIC_BOLD,
            "Sanctuary Lyrics — Manrope",
            "Amazing grace! How sweet the sound\nThat saved a wretch like me!",
            FontWeight::EXTRA_BOLD,
            "John Newton · CCLI #22025",
            false,
        ),
        (
            FONT_SCRIPTURE_SERIF,
            "Editorial Scripture — Newsreader",
            "Love is patient and is kind.\nLove doesn't envy. Love doesn't brag.",
            FontWeight::NORMAL,
            "1 Corinthians 13:4 · WEB",
            false,
        ),
        (
            FONT_DISPLAY,
            "Clean Studio — Plus Jakarta Sans",
            "When peace, like a river, attendeth my way,\nIt is well with my soul.",
            FontWeight::SEMIBOLD,
            "Horatio G. Spafford",
            true,
        ),
        (
            FONT_DISPLAY_PROJ,
            "Announcement Titles — Syne",
            "Sunday Worship\nGathering at 10:00 AM",
            FontWeight::BOLD,
            "Grace Fellowship",
            false,
        ),
    ];

    let mut grid = div().w_full().flex().flex_wrap().gap(px(14.0));
    for &(family, role, sample, weight, footer, warm_bg) in cards {
        grid = grid.child(
            div()
                .w(px(332.0))
                .h(px(187.0)) // exact 16:9
                .bg(if warm_bg {
                    rgb(0x1E1813)
                } else {
                    TOKENS.canvas
                })
                .border_1()
                .border_color(TOKENS.gridline)
                .rounded(px(RADIUS_TILE))
                .p(px(16.0))
                .flex()
                .flex_col()
                .justify_between()
                .child(
                    div()
                        .font_family(FONT_BODY)
                        .text_color(TOKENS.signal_preview)
                        .text_size(px(11.0))
                        .font_weight(FontWeight::MEDIUM)
                        .child(role),
                )
                .child(
                    div()
                        .font_family(family)
                        .font_weight(weight)
                        .text_color(TOKENS.ink_primary)
                        .text_size(px(19.0))
                        .line_height(px(26.0))
                        .child(sample),
                )
                .child(
                    div()
                        .font_family(FONT_BODY)
                        .text_color(TOKENS.ink_secondary)
                        .text_size(px(11.0))
                        .child(footer),
                ),
        );
    }

    section_frame(
        "Sanctuary Projection Font Pack",
        "4 bundled SIL OFL typefaces ready offline out of the box",
    )
    .child(grid)
}

pub(crate) fn render_icon_catalog() -> Div {
    let mut size_row = div()
        .flex()
        .items_center()
        .gap(px(18.0))
        .pb(px(10.0))
        .border_b_1()
        .border_color(TOKENS.gridline);

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
                .child(deck_icon(DeckIcon::BroadcastFill, sz, TOKENS.signal_live))
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
                .w(px(112.0))
                .px(px(9.0))
                .py(px(8.0))
                .bg(TOKENS.canvas)
                .border_1()
                .border_color(TOKENS.gridline)
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
