use gpui::{Div, FontWeight, ParentElement, Styled, div, px, rgb};

use crate::design::gallery_console::section_frame;
use crate::design::icons::{DeckIcon, IconSize, deck_icon};
use crate::design::motion::MotionMode;
use crate::design::primitives::{
    VoiceSyncMode, display_status_badge, hardware_switcher_key_body, render_voice_sync_pill_display,
};
use crate::design::tokens::{
    FONT_DISPLAY, FONT_DISPLAY_PROJ, FONT_LYRIC_BOLD, FONT_MONO, FONT_SCRIPTURE_SERIF,
    RADIUS_CONTROL, RADIUS_TILE, TOKENS,
};

pub(crate) fn render_switcher_and_voice_showcase(motion_mode: MotionMode) -> Div {
    let switcher_col = div()
        .flex_1()
        .bg(TOKENS.canvas)
        .border_1()
        .border_color(TOKENS.gridline)
        .p(px(10.0))
        .flex()
        .flex_col()
        .gap(px(8.0))
        .child(
            div()
                .font_family(FONT_MONO)
                .text_color(TOKENS.ink_muted)
                .text_size(px(9.5))
                .child("TACTILE OVERRIDE STRIP // TALLY RED & ACTIVE WARNING ORANGE"),
        )
        .child(
            div()
                .w_full()
                .flex()
                .gap(px(6.0))
                .child(div().flex_1().child(hardware_switcher_key_body(
                    DeckIcon::PlayFill,
                    "GO LIVE",
                    "F5",
                    true,
                    TOKENS.signal_live,
                )))
                .child(div().flex_1().child(hardware_switcher_key_body(
                    DeckIcon::TextTSlash,
                    "CLEAR",
                    "F1",
                    false,
                    TOKENS.signal_override,
                )))
                .child(div().flex_1().child(hardware_switcher_key_body(
                    DeckIcon::EyeSlash,
                    "BLACK",
                    "F2",
                    true,
                    TOKENS.signal_override,
                )))
                .child(div().flex_1().child(hardware_switcher_key_body(
                    DeckIcon::Image,
                    "LOGO",
                    "F3",
                    false,
                    TOKENS.signal_override,
                ))),
        );

    let voice_col = div()
        .flex_1()
        .bg(TOKENS.canvas)
        .border_1()
        .border_color(TOKENS.gridline)
        .p(px(10.0))
        .flex()
        .flex_col()
        .gap(px(8.0))
        .child(
            div()
                .font_family(FONT_MONO)
                .text_color(TOKENS.ink_muted)
                .text_size(px(9.5))
                .child("VOICE-SYNC HARDWARE PILL & DISPLAY CONNECTION BADGES"),
        )
        .child(render_voice_sync_pill_display(
            "gal-voice-off",
            VoiceSyncMode::Off,
            0,
            None,
            "BOOTH-IN 1/2",
            motion_mode,
        ))
        .child(render_voice_sync_pill_display(
            "gal-voice-suggest",
            VoiceSyncMode::Suggest,
            4,
            Some(82),
            "X32-USB CH1",
            motion_mode,
        ))
        .child(render_voice_sync_pill_display(
            "gal-voice-auto",
            VoiceSyncMode::Auto,
            6,
            Some(94),
            "X32-USB CH1",
            motion_mode,
        ))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .child(display_status_badge(
                    DeckIcon::MonitorFill,
                    "AUD",
                    "1920x1080",
                    true,
                    TOKENS.signal_live,
                ))
                .child(display_status_badge(
                    DeckIcon::ChalkboardSimple,
                    "STG",
                    "1920x1080",
                    true,
                    TOKENS.signal_preview,
                ))
                .child(display_status_badge(
                    DeckIcon::Monitor,
                    "AUD",
                    "STANDBY",
                    false,
                    TOKENS.ink_muted,
                )),
        );

    section_frame(
        "04",
        "HARDWARE OVERRIDE KEYS & TELEMETRY",
        "EMBOSSED TOP BEVEL // F1-F5 MONO KEYCAPS // 6-TICK AUDIO METER",
    )
    .child(
        div()
            .w_full()
            .flex()
            .gap(px(12.0))
            .child(switcher_col)
            .child(voice_col),
    )
}

pub(crate) fn render_projection_font_pack() -> Div {
    let cards: &[(&str, &str, &str, FontWeight, &str, bool)] = &[
        (
            FONT_LYRIC_BOLD,
            "MODERN BOLD SANS // LYRICS",
            "Amazing grace! How sweet the sound\nThat saved a wretch like me!",
            FontWeight::EXTRA_BOLD,
            "JOHN NEWTON • CCLI #22025",
            false,
        ),
        (
            FONT_SCRIPTURE_SERIF,
            "EDITORIAL SCRIPTURE SERIF",
            "4 Love is patient and is kind.\nLove doesn't envy. Love doesn't brag.",
            FontWeight::NORMAL,
            "1 CORINTHIANS 13:4 (WEB)",
            false,
        ),
        (
            FONT_DISPLAY,
            "MINIMAL GROTESK // LOWER & CENTER",
            "When peace, like a river, attendeth my way,\nIt is well with my soul.",
            FontWeight::SEMIBOLD,
            "HORATIO G. SPAFFORD",
            true,
        ),
        (
            FONT_DISPLAY_PROJ,
            "CONTEMPORARY DISPLAY // TITLES",
            "SUNDAY WORSHIP\nGATHERING AT 10:00 AM",
            FontWeight::BOLD,
            "GRACE FELLOWSHIP",
            false,
        ),
    ];

    let mut grid = div().w_full().flex().flex_wrap().gap(px(12.0));
    for &(family, role, sample, weight, footer, warm_bg) in cards {
        grid = grid.child(
            div()
                .w(px(332.0))
                .h(px(187.0)) // exact 16:9
                .bg(if warm_bg {
                    rgb(0x19120D)
                } else {
                    TOKENS.canvas
                })
                .border_1()
                .border_color(TOKENS.gridline)
                .rounded(px(RADIUS_TILE))
                .p(px(14.0))
                .flex()
                .flex_col()
                .justify_between()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(
                            div()
                                .font_family(FONT_MONO)
                                .text_color(TOKENS.signal_preview)
                                .text_size(px(9.5))
                                .font_weight(FontWeight::BOLD)
                                .child(family.to_ascii_uppercase()),
                        )
                        .child(
                            div()
                                .font_family(FONT_MONO)
                                .text_color(TOKENS.ink_muted)
                                .text_size(px(9.0))
                                .child(role),
                        ),
                )
                .child(
                    div()
                        .font_family(family)
                        .font_weight(weight)
                        .text_color(TOKENS.ink_primary)
                        .text_size(px(19.0))
                        .line_height(px(25.0))
                        .child(sample),
                )
                .child(
                    div()
                        .font_family(FONT_MONO)
                        .text_color(TOKENS.ink_secondary)
                        .text_size(px(9.5))
                        .child(footer),
                ),
        );
    }

    section_frame(
        "05",
        "SANCTUARY PROJECTION FONT PACK (4 OFL TYPEFACES)",
        "ZERO FALLBACK TO SYSTEM ARIAL // 16:9 CONGREGATION-TESTED SAMPLES",
    )
    .child(grid)
}

pub(crate) fn render_icon_catalog() -> Div {
    let mut size_row = div()
        .flex()
        .items_center()
        .gap(px(16.0))
        .pb(px(10.0))
        .border_b_1()
        .border_color(TOKENS.gridline);

    for (label, sz) in [
        ("SM 14PX (INLINE)", IconSize::Sm),
        ("MD 16PX (TOOLBAR)", IconSize::Md),
        ("LG 18PX (TRANSPORT)", IconSize::Lg),
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
                        .font_family(FONT_MONO)
                        .text_color(TOKENS.ink_secondary)
                        .text_size(px(10.0))
                        .child(label),
                ),
        );
    }

    let mut swatches = div().w_full().flex().flex_wrap().gap(px(6.0));
    for &icon in DeckIcon::ALL {
        let tint = if icon.short_name().ends_with('*') {
            TOKENS.signal_preview
        } else {
            TOKENS.ink_primary
        };
        swatches = swatches.child(
            div()
                .w(px(104.0))
                .px(px(8.0))
                .py(px(7.0))
                .bg(TOKENS.canvas)
                .border_1()
                .border_color(TOKENS.gridline)
                .rounded(px(RADIUS_CONTROL))
                .flex()
                .items_center()
                .gap(px(7.0))
                .child(deck_icon(icon, IconSize::Md, tint))
                .child(
                    div()
                        .font_family(FONT_MONO)
                        .text_color(TOKENS.ink_secondary)
                        .text_size(px(9.5))
                        .child(icon.short_name()),
                ),
        );
    }

    section_frame(
        "06",
        "VENDORED PHOSPHOR SVG ICON CATALOG (49 EMBEDDED)",
        "RUST-EMBED ASSET SOURCE // OFFLINE-SAFE // REGULAR + FILL WEIGHTS // ZERO EMOJIS",
    )
    .child(size_row)
    .child(swatches)
}
