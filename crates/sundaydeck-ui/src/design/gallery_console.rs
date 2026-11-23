use gpui::{Div, FontWeight, ParentElement, Rgba, Styled, div, px};
use sundaydeck_core::SectionKind;

use crate::design::primitives::{keycap, section_tag, suggest_corner_bracket};
use crate::design::tokens::{
    FONT_BODY, FONT_DISPLAY, FONT_LYRIC_BOLD, FONT_MONO, FONT_SCRIPTURE_SERIF, RADIUS_CONTROL,
    RADIUS_TILE, TOKENS, section_strip_color,
};

pub(crate) fn section_frame(index: &str, title: &str, subtitle: &str) -> Div {
    div()
        .w_full()
        .bg(TOKENS.pane)
        .border_1()
        .border_color(TOKENS.gridline)
        .p(px(14.0))
        .flex()
        .flex_col()
        .gap(px(12.0))
        .child(
            div()
                .pb(px(8.0))
                .border_b_1()
                .border_color(TOKENS.gridline)
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .child(
                            div()
                                .font_family(FONT_MONO)
                                .text_color(TOKENS.signal_preview)
                                .text_size(px(11.0))
                                .font_weight(FontWeight::BOLD)
                                .child(index.to_owned()),
                        )
                        .child(
                            div()
                                .font_family(FONT_DISPLAY)
                                .text_color(TOKENS.ink_primary)
                                .text_size(px(13.0))
                                .font_weight(FontWeight::BOLD)
                                .child(title.to_owned()),
                        ),
                )
                .child(
                    div()
                        .font_family(FONT_MONO)
                        .text_color(TOKENS.ink_muted)
                        .text_size(px(10.0))
                        .child(subtitle.to_owned()),
                ),
        )
}

pub(crate) fn render_color_swatches() -> Div {
    let entries: &[(&str, &str, &str, Rgba)] = &[
        ("CANVAS", "#0A0B0D", "Base Surface", TOKENS.canvas),
        ("PANE", "#111317", "Rail / Panel", TOKENS.pane),
        ("ELEVATED", "#181B21", "Tile / Key Cap", TOKENS.elevated),
        ("GRIDLINE", "#242832", "1px Rules", TOKENS.gridline),
        ("FOCUS", "#4B5565", "Active Frame", TOKENS.focus_border),
        ("INK PRI", "#F2F0EB", "Warm Primary", TOKENS.ink_primary),
        ("INK SEC", "#8E95A2", "Labels", TOKENS.ink_secondary),
        ("INK MUTE", "#4E5563", "Disabled", TOKENS.ink_muted),
        ("LIVE", "#E53935", "On-Air Tally", TOKENS.signal_live),
        ("PREVIEW", "#EAB308", "Staged Amber", TOKENS.signal_preview),
        ("VOICE", "#22C55E", "Audio Lock", TOKENS.signal_voice),
        (
            "OVERRIDE",
            "#F97316",
            "Blackout Warn",
            TOKENS.signal_override,
        ),
    ];

    let mut row = div().w_full().flex().flex_wrap().gap(px(8.0));
    for &(name, hex, role, swatch) in entries {
        row = row.child(
            div()
                .w(px(106.0))
                .bg(TOKENS.canvas)
                .border_1()
                .border_color(TOKENS.gridline)
                .rounded(px(RADIUS_CONTROL))
                .overflow_hidden()
                .flex()
                .flex_col()
                .child(
                    div()
                        .w_full()
                        .h(px(28.0))
                        .bg(swatch)
                        .border_b_1()
                        .border_color(TOKENS.gridline),
                )
                .child(
                    div()
                        .p(px(6.0))
                        .flex()
                        .flex_col()
                        .gap(px(1.0))
                        .child(
                            div()
                                .font_family(FONT_MONO)
                                .text_color(TOKENS.ink_primary)
                                .text_size(px(9.5))
                                .font_weight(FontWeight::BOLD)
                                .child(name),
                        )
                        .child(
                            div()
                                .font_family(FONT_MONO)
                                .text_color(TOKENS.ink_secondary)
                                .text_size(px(9.0))
                                .child(hex),
                        )
                        .child(
                            div()
                                .font_family(FONT_BODY)
                                .text_color(TOKENS.ink_muted)
                                .text_size(px(9.0))
                                .child(role),
                        ),
                ),
        );
    }

    section_frame(
        "01",
        "COLOR & SURFACE GRAMMAR",
        "DIM SOUND-BOOTH MATTE PALETTE // STRICT OPERATIONAL SIGNAL SEMANTICS",
    )
    .child(row)
}

pub(crate) fn render_operator_typography() -> Div {
    let type_box = |label: &'static str| {
        div()
            .flex_1()
            .bg(TOKENS.canvas)
            .border_1()
            .border_color(TOKENS.gridline)
            .p(px(10.0))
            .flex()
            .flex_col()
            .gap(px(6.0))
            .child(
                div()
                    .font_family(FONT_MONO)
                    .text_color(TOKENS.ink_muted)
                    .text_size(px(9.5))
                    .child(label),
            )
    };

    let col_display = type_box("DISPLAY // SPACE GROTESK SEMIBOLD & BOLD")
        .child(
            div()
                .font_family(FONT_DISPLAY)
                .text_color(TOKENS.ink_primary)
                .text_size(px(20.0))
                .font_weight(FontWeight::BOLD)
                .child("SUNDAYDECK CONSOLE"),
        )
        .child(
            div()
                .font_family(FONT_DISPLAY)
                .text_color(TOKENS.ink_secondary)
                .text_size(px(13.5))
                .font_weight(FontWeight::SEMIBOLD)
                .child("02  Amazing Grace  •  Arrangement: Default"),
        );

    let col_body = type_box("BODY // PLUS JAKARTA SANS REGULAR & MEDIUM")
        .child(
            div()
                .font_family(FONT_BODY)
                .text_color(TOKENS.ink_primary)
                .text_size(px(13.5))
                .font_weight(FontWeight::MEDIUM)
                .child("When peace, like a river, attendeth my way,"),
        )
        .child(
            div()
                .font_family(FONT_BODY)
                .text_color(TOKENS.ink_secondary)
                .text_size(px(12.0))
                .child("Dense 12-14px schedule lists, lyric bodies, and Bible scroll."),
        );

    let col_mono = type_box("TELEMETRY // JETBRAINS MONO TABULAR")
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(10.0))
                .font_family(FONT_MONO)
                .text_size(px(13.0))
                .font_weight(FontWeight::BOLD)
                .child(div().text_color(TOKENS.ink_primary).child("ROM 8:28"))
                .child(div().text_color(TOKENS.signal_voice).child("94%"))
                .child(div().text_color(TOKENS.signal_preview).child("04:32"))
                .child(div().text_color(TOKENS.signal_live).child("-00:18")),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(5.0))
                .child(keycap("SPACE", false))
                .child(keycap("F1", false))
                .child(keycap("F2", false))
                .child(keycap("F3", false))
                .child(keycap("F5", false))
                .child(keycap("CMD+K", false)),
        );

    section_frame(
        "02",
        "OPERATOR CONSOLE TYPOGRAPHY",
        "18 STATIC SIL OFL TTF WEIGHTS BUNDLED VIA include_bytes!",
    )
    .child(
        div()
            .w_full()
            .flex()
            .gap(px(10.0))
            .child(col_display)
            .child(col_body)
            .child(col_mono),
    )
}

pub(crate) fn render_slide_tile_showcase() -> Div {
    let tiles = div()
        .w_full()
        .flex()
        .flex_wrap()
        .gap(px(12.0))
        .child(mock_slide_tile(
            "V1",
            "01",
            SectionKind::Verse,
            "Amazing grace! How sweet the sound\nThat saved a wretch like me!",
            FONT_LYRIC_BOLD,
            TOKENS.gridline,
            TOKENS.elevated,
            None,
            None,
            false,
        ))
        .child(mock_slide_tile(
            "CH",
            "02",
            SectionKind::Chorus,
            "It is well with my soul,\nIt is well, it is well with my soul.",
            FONT_LYRIC_BOLD,
            TOKENS.signal_preview,
            TOKENS.signal_preview_tint,
            Some(("STAGED", TOKENS.signal_preview)),
            None,
            false,
        ))
        .child(mock_slide_tile(
            "V1",
            "01",
            SectionKind::Verse,
            "When peace, like a river, attendeth my way,\nWhen sorrows like sea billows roll;",
            FONT_LYRIC_BOLD,
            TOKENS.signal_live,
            TOKENS.signal_live_tint,
            Some(("ON AIR", TOKENS.signal_live)),
            Some(0.84),
            false,
        ))
        .child(mock_slide_tile(
            "BR",
            "04",
            SectionKind::Bridge,
            "Praise the Lord, praise the Lord,\nO my soul!",
            FONT_LYRIC_BOLD,
            TOKENS.gridline,
            TOKENS.elevated,
            Some(("NEXT 94%", TOKENS.signal_preview)),
            None,
            true,
        ))
        .child(mock_slide_tile(
            "ROM 8:28",
            "01",
            SectionKind::Other,
            "28 We know that all things work together for good for those who love God.",
            FONT_SCRIPTURE_SERIF,
            TOKENS.focus_border,
            TOKENS.canvas,
            Some(("WEB", TOKENS.ink_secondary)),
            None,
            false,
        ));

    section_frame(
        "03",
        "16:9 SLIDE TILES & VOICE-SYNC ALIGNMENT",
        "REAL BUNDLED TYPOGRAPHY // 3PX RADIUS // VOICE CONFIDENCE BAR + AMBER SUGGEST BRACKET",
    )
    .child(tiles)
}

#[allow(clippy::too_many_arguments)]
fn mock_slide_tile(
    code: &str,
    index: &str,
    kind: SectionKind,
    text: &str,
    body_font: &str,
    border: Rgba,
    bg: Rgba,
    state_tag: Option<(&str, Rgba)>,
    voice_progress: Option<f32>,
    suggest_bracket: bool,
) -> Div {
    let strip = section_strip_color(kind);
    let mut tile = div()
        .relative()
        .w(px(252.0))
        .h(px(142.0))
        .rounded(px(RADIUS_TILE))
        .overflow_hidden()
        .bg(bg)
        .border_1()
        .border_color(border)
        .flex()
        .flex_col()
        .child(
            div()
                .w_full()
                .px(px(8.0))
                .py(px(4.0))
                .bg(TOKENS.pane)
                .border_b_1()
                .border_color(TOKENS.gridline)
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.0))
                        .child(section_tag(code, strip))
                        .child(
                            div()
                                .font_family(FONT_MONO)
                                .text_color(TOKENS.ink_muted)
                                .text_size(px(9.5))
                                .child(index.to_owned()),
                        ),
                )
                .child(match state_tag {
                    Some((label, col)) => div()
                        .font_family(FONT_MONO)
                        .text_color(col)
                        .text_size(px(9.5))
                        .font_weight(FontWeight::BOLD)
                        .child(label.to_owned()),
                    None => div(),
                }),
        )
        .child(
            div()
                .flex_1()
                .w_full()
                .min_w_0()
                .px(px(10.0))
                .py(px(8.0))
                .overflow_hidden()
                .flex()
                .flex_col()
                .justify_center()
                .child(
                    div()
                        .w_full()
                        .font_family(body_font.to_owned())
                        .text_color(TOKENS.ink_primary)
                        .text_size(px(12.0))
                        .line_height(px(16.5))
                        .child(text.to_owned()),
                ),
        );

    if let Some(progress) = voice_progress {
        let fill_w = (250.0 * progress.clamp(0.0, 1.0)).round();
        tile = tile.child(
            div()
                .w_full()
                .h(px(3.0))
                .bg(TOKENS.canvas)
                .child(div().w(px(fill_w)).h_full().bg(TOKENS.signal_voice)),
        );
    }

    if suggest_bracket {
        tile = tile.child(suggest_corner_bracket());
    }

    tile
}
