use gpui::{Div, FontWeight, ParentElement, Rgba, Styled, div, px, rgb};
use sundaydeck_core::SectionKind;

use crate::design::primitives::{keycap, next_up_hint_pill, section_pill, soft_pill};
use crate::design::tokens::{
    FONT_BODY, FONT_DISPLAY, FONT_LYRIC_BOLD, FONT_MONO, FONT_SCRIPTURE_SERIF, RADIUS_CONTROL,
    RADIUS_MODAL, RADIUS_TILE, TOKENS,
};

pub(crate) fn section_frame(title: &str, subtitle: &str) -> Div {
    div()
        .w_full()
        .rounded(px(RADIUS_MODAL))
        .bg(TOKENS.pane)
        .border_1()
        .border_color(TOKENS.gridline)
        .p(px(16.0))
        .flex()
        .flex_col()
        .gap(px(14.0))
        .child(
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
        ("Canvas", "#131417", "Warm obsidian", TOKENS.canvas),
        ("Sidebar", "#181A1F", "Schedule panel", TOKENS.pane),
        ("Slide Card", "#20232A", "Elevated surface", TOKENS.elevated),
        (
            "Hover Row",
            "#282C35",
            "Active highlight",
            TOKENS.elevated_hover,
        ),
        ("Soft Border", "#2B2F3A", "Subtle divider", TOKENS.gridline),
        (
            "Primary Ink",
            "#F5F3EF",
            "Warm paper white",
            TOKENS.ink_primary,
        ),
        (
            "Secondary Ink",
            "#A5AAB5",
            "Comfortable label",
            TOKENS.ink_secondary,
        ),
        ("Muted Ink", "#6E7482", "Helper copy", TOKENS.ink_muted),
        ("Live", "#E04F47", "On screen now", TOKENS.signal_live),
        ("Next Up", "#E5A93C", "Honey amber", TOKENS.signal_preview),
        (
            "Auto-Follow",
            "#34B37E",
            "Sage emerald",
            TOKENS.signal_voice,
        ),
        (
            "Paused Screen",
            "#E07A3A",
            "Warm terracotta",
            TOKENS.signal_override,
        ),
    ];

    let mut row = div().w_full().flex().flex_wrap().gap(px(8.0));
    for &(name, hex, role, swatch) in entries {
        row = row.child(
            div()
                .w(px(105.0))
                .bg(TOKENS.canvas)
                .border_1()
                .border_color(TOKENS.gridline)
                .rounded(px(RADIUS_TILE))
                .overflow_hidden()
                .flex()
                .flex_col()
                .child(
                    div()
                        .w_full()
                        .h(px(30.0))
                        .bg(swatch)
                        .border_b_1()
                        .border_color(TOKENS.gridline),
                )
                .child(
                    div()
                        .p(px(8.0))
                        .flex()
                        .flex_col()
                        .gap(px(2.0))
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
                                .text_color(TOKENS.ink_secondary)
                                .text_size(px(10.0))
                                .child(role),
                        )
                        .child(
                            div()
                                .font_family(FONT_MONO)
                                .text_color(TOKENS.ink_muted)
                                .text_size(px(9.5))
                                .child(hex),
                        ),
                ),
        );
    }

    section_frame(
        "Warm Studio Palette",
        "Layered dim-room charcoal surfaces with calm semantic accents",
    )
    .child(row)
}

pub(crate) fn render_operator_typography() -> Div {
    let type_box = |label: &'static str| {
        div()
            .flex_1()
            .rounded(px(RADIUS_TILE))
            .bg(TOKENS.canvas)
            .border_1()
            .border_color(TOKENS.gridline)
            .p(px(12.0))
            .flex()
            .flex_col()
            .gap(px(8.0))
            .child(
                div()
                    .font_family(FONT_BODY)
                    .text_color(TOKENS.ink_muted)
                    .text_size(px(11.0))
                    .font_weight(FontWeight::MEDIUM)
                    .child(label),
            )
    };

    let col_ui = type_box("Primary UI — Plus Jakarta Sans (Sentence case)")
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

    let col_pills = type_box("Song & Scripture Section Pills")
        .child(
            div()
                .flex()
                .flex_wrap()
                .gap(px(6.0))
                .child(section_pill("Verse 1", Some(SectionKind::Verse)))
                .child(section_pill("Chorus", Some(SectionKind::Chorus)))
                .child(section_pill("Bridge", Some(SectionKind::Bridge)))
                .child(section_pill("Romans 8:28", None))
                .child(soft_pill(
                    "Live",
                    TOKENS.signal_live_tint,
                    TOKENS.signal_live,
                ))
                .child(soft_pill(
                    "Next Up",
                    TOKENS.signal_preview_tint,
                    TOKENS.signal_preview,
                )),
        )
        .child(
            div()
                .font_family(FONT_BODY)
                .text_color(TOKENS.ink_secondary)
                .text_size(px(12.0))
                .child("Softly tinted pills replace cryptic short codes."),
        );

    let col_timers = type_box("Minimal Monospace — Clocks, Timers & Hints Only")
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(14.0))
                .font_family(FONT_MONO)
                .text_size(px(14.0))
                .font_weight(FontWeight::SEMIBOLD)
                .child(div().text_color(TOKENS.ink_primary).child("10:15 AM"))
                .child(div().text_color(TOKENS.signal_preview).child("04:32"))
                .child(div().text_color(TOKENS.signal_live).child("-00:18")),
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
                        .child("Tooltip hints:"),
                )
                .child(keycap("Space", false))
                .child(keycap("F1", false))
                .child(keycap("F2", false))
                .child(keycap("F5", false)),
        );

    section_frame(
        "Warm Editorial Typography",
        "Bundled SIL OFL fonts — approachable Sentence case everywhere",
    )
    .child(
        div()
            .w_full()
            .flex()
            .gap(px(12.0))
            .child(col_ui)
            .child(col_pills)
            .child(col_timers),
    )
}

pub(crate) fn render_slide_tile_showcase() -> Div {
    let tiles = div()
        .w_full()
        .flex()
        .flex_wrap()
        .gap(px(14.0))
        .child(mock_slide_tile(
            "Verse 1",
            Some(SectionKind::Verse),
            "Amazing grace! How sweet the sound\nThat saved a wretch like me!",
            FONT_LYRIC_BOLD,
            rgb(0x181B22),
            TOKENS.gridline,
            false,
            None,
            None,
            false,
        ))
        .child(mock_slide_tile(
            "Chorus",
            Some(SectionKind::Chorus),
            "It is well with my soul,\nIt is well, it is well with my soul.",
            FONT_LYRIC_BOLD,
            rgb(0x211D18),
            TOKENS.signal_preview,
            true,
            Some(("Next Up", TOKENS.signal_preview_tint, TOKENS.signal_preview)),
            None,
            false,
        ))
        .child(mock_slide_tile(
            "Verse 1",
            Some(SectionKind::Verse),
            "When peace, like a river, attendeth my way,\nWhen sorrows like sea billows roll;",
            FONT_LYRIC_BOLD,
            rgb(0x1D191B),
            TOKENS.signal_live,
            true,
            Some(("Live", TOKENS.signal_live, TOKENS.ink_primary)),
            Some(0.78),
            false,
        ))
        .child(mock_slide_tile(
            "Bridge",
            Some(SectionKind::Bridge),
            "Praise the Lord, praise the Lord,\nO my soul!",
            FONT_LYRIC_BOLD,
            rgb(0x1A1B23),
            TOKENS.signal_preview,
            true,
            None,
            None,
            true,
        ))
        .child(mock_slide_tile(
            "Romans 8:28",
            None,
            "We know that all things work together for good for those who love God.",
            FONT_SCRIPTURE_SERIF,
            rgb(0x161B1D),
            TOKENS.gridline,
            false,
            Some(("WEB", TOKENS.elevated, TOKENS.ink_secondary)),
            None,
            false,
        ));

    section_frame(
        "16:9 Slide Cards",
        "Miniature sanctuary previews with colored section pills, 2px Live ring, and gentle Auto-Follow cue",
    )
    .child(tiles)
}

#[allow(clippy::too_many_arguments)]
fn mock_slide_tile(
    label: &str,
    kind: Option<SectionKind>,
    text: &str,
    body_font: &str,
    sanctuary_bg: Rgba,
    border: Rgba,
    thick_ring: bool,
    state_pill: Option<(&str, Rgba, Rgba)>,
    voice_progress: Option<f32>,
    show_next_up_hint: bool,
) -> Div {
    let mut tile = div()
        .relative()
        .w(px(256.0))
        .h(px(144.0)) // exact 16:9
        .rounded(px(RADIUS_TILE))
        .overflow_hidden()
        .bg(sanctuary_bg)
        .border_color(border)
        .flex()
        .flex_col();

    tile = if thick_ring {
        tile.border_2()
    } else {
        tile.border_1()
    };

    tile = tile
        .child(
            div()
                .w_full()
                .px(px(10.0))
                .pt(px(8.0))
                .pb(px(4.0))
                .flex()
                .items_center()
                .justify_between()
                .child(section_pill(label, kind))
                .child(match state_pill {
                    Some((tag, bg, fg)) => soft_pill(tag, bg, fg),
                    None => div(),
                }),
        )
        .child(
            div()
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

    if show_next_up_hint {
        tile = tile.child(
            div()
                .w_full()
                .pb(px(7.0))
                .flex()
                .justify_center()
                .child(next_up_hint_pill()),
        );
    }

    if let Some(progress) = voice_progress {
        let fill_w = (252.0 * progress.clamp(0.0, 1.0)).round();
        tile = tile.child(
            div().w_full().h(px(3.0)).bg(TOKENS.canvas).child(
                div()
                    .w(px(fill_w))
                    .h_full()
                    .rounded(px(RADIUS_CONTROL))
                    .bg(TOKENS.signal_voice),
            ),
        );
    }

    tile
}
