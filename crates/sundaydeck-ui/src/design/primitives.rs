use gpui::{
    AnyElement, Div, FontWeight, InteractiveElement, IntoElement, ParentElement, Rgba, Stateful,
    Styled, div, linear_color_stop, linear_gradient, px, rgb, rgba,
};
use sundaydeck_core::SectionKind;

use crate::design::icons::{DeckIcon, IconSize, deck_icon};
use crate::design::motion::{MotionMode, tally_pulse_dot};
use crate::design::tokens::{
    FONT_BODY, FONT_MONO, RADIUS_BUTTON, RADIUS_CONTROL, RADIUS_PILL, TOKENS, section_dot_color,
    section_friendly_label,
};

pub use sundaydeck_voice::VoiceSyncMode;

fn dot_6px(color: Rgba) -> Div {
    div()
        .w(px(6.0))
        .h(px(6.0))
        .flex_shrink_0()
        .rounded(px(RADIUS_PILL))
        .bg(color)
}

fn top_edge_highlight(color: Rgba) -> Div {
    div()
        .absolute()
        .top_0()
        .left_0()
        .right_0()
        .h(px(1.0))
        .bg(color)
}

/// Subtle optional keyboard shortcut keycap (used in help/tooltips, never stamped on main faces).
pub fn keycap(label: &str) -> Div {
    div()
        .px(px(6.0))
        .py(px(1.5))
        .rounded(px(4.0))
        .bg(TOKENS.recessed)
        .border_1()
        .border_color(TOKENS.gridline)
        .font_family(FONT_MONO)
        .text_color(TOKENS.ink_secondary)
        .text_size(px(10.0))
        .font_weight(FontWeight::MEDIUM)
        .child(label.to_owned())
}

/// Precision `6px` dot + clean Sentence-case typographic metadata (`Verse 1`, `Chorus`, `Romans 8:28`).
pub fn section_meta_label(label: &str, kind: Option<SectionKind>) -> Div {
    status_dot_label(
        &section_friendly_label(label, kind),
        section_dot_color(label, kind),
        TOKENS.ink_secondary,
    )
}

/// Understated Live metadata (`6px` solid `#E04F47` dot + plain warm-white `Live` text).
pub fn live_meta_indicator() -> Div {
    status_dot_label("Live", TOKENS.signal_live_dot, TOKENS.ink_primary)
}

/// Reusable `6px` solid precision dot + plain typographic label (zero translucent pill box).
pub fn status_dot_label(label: &str, dot_color: Rgba, text_color: Rgba) -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(6.0))
        .child(dot_6px(dot_color))
        .child(
            div()
                .font_family(FONT_BODY)
                .text_color(text_color)
                .text_size(px(11.0))
                .font_weight(FontWeight::SEMIBOLD)
                .child(label.to_owned()),
        )
}

/// Warm paper-ivory Primary Action (`Show on Screen`, `Add Plan to Service`, `Save Song`).
pub fn primary_ivory_button_body(icon: Option<DeckIcon>, label: &str, enabled: bool) -> Div {
    let ink = if enabled {
        TOKENS.ivory_ink
    } else {
        TOKENS.ink_muted
    };
    let mut btn = div()
        .relative()
        .h(px(32.0))
        .px(px(14.0))
        .rounded(px(RADIUS_BUTTON))
        .overflow_hidden()
        .bg(if enabled {
            TOKENS.ivory_bg
        } else {
            TOKENS.elevated
        })
        .hover(move |s| if enabled { s.bg(TOKENS.ivory_hover) } else { s })
        .border_1()
        .border_color(if enabled {
            rgba(0x0000_0030)
        } else {
            TOKENS.gridline
        })
        .flex()
        .items_center()
        .justify_center()
        .gap(px(6.5));

    if enabled {
        btn = btn.child(top_edge_highlight(rgba(0xFFFF_FFAA))).child(
            div()
                .absolute()
                .bottom_0()
                .left_0()
                .right_0()
                .h(px(1.0))
                .bg(rgba(0x0000_002E)),
        );
    }
    if let Some(ic) = icon {
        btn = btn.child(deck_icon(ic, IconSize::Sm, ink));
    }
    btn.child(
        div()
            .font_family(FONT_BODY)
            .text_color(ink)
            .text_size(px(12.0))
            .font_weight(FontWeight::SEMIBOLD)
            .child(label.to_owned()),
    )
}

/// Tall warm paper-ivory Primary Deck Action for the Screen Controls row (`Show on Screen`).
pub fn primary_ivory_deck_key_body(icon: DeckIcon, label: &str) -> Div {
    div()
        .relative()
        .w_full()
        .px(px(10.0))
        .py(px(11.0))
        .rounded(px(RADIUS_BUTTON))
        .overflow_hidden()
        .bg(TOKENS.ivory_bg)
        .hover(|s| s.bg(TOKENS.ivory_hover))
        .border_1()
        .border_color(rgba(0x0000_0036))
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(5.0))
        .child(top_edge_highlight(rgba(0xFFFF_FFAA)))
        .child(
            div()
                .absolute()
                .bottom_0()
                .left_0()
                .right_0()
                .h(px(1.5))
                .bg(rgba(0x0000_002E)),
        )
        .child(deck_icon(icon, IconSize::Md, TOKENS.ivory_ink))
        .child(
            div()
                .font_family(FONT_BODY)
                .text_color(TOKENS.ivory_ink)
                .text_size(px(11.5))
                .font_weight(FontWeight::SEMIBOLD)
                .child(label.to_owned()),
        )
}

/// Sculpted native screen-override key (`Hide Words`, `Black Screen`, `Show Logo`).
/// Idle: raised warm-graphite gradient + `1px` top highlight.
/// Engaged: recessed `#15171C` latched state, `#D97736` border, `6px` top-right dot, `Active` caption.
pub fn native_screen_key_body(icon: DeckIcon, label: &str, is_active: bool) -> Div {
    if is_active {
        return div()
            .relative()
            .w_full()
            .px(px(8.0))
            .py(px(9.0))
            .rounded(px(RADIUS_BUTTON))
            .overflow_hidden()
            .bg(TOKENS.recessed)
            .hover(|s| s.bg(TOKENS.status_surface))
            .border_1()
            .border_color(TOKENS.signal_override)
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(3.5))
            .child(
                div()
                    .absolute()
                    .top(px(7.0))
                    .right(px(7.0))
                    .child(dot_6px(TOKENS.signal_override)),
            )
            .child(deck_icon(icon, IconSize::Md, TOKENS.signal_override))
            .child(
                div()
                    .font_family(FONT_BODY)
                    .text_color(TOKENS.ink_primary)
                    .text_size(px(11.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(label.to_owned()),
            )
            .child(
                div()
                    .font_family(FONT_BODY)
                    .text_color(TOKENS.signal_override)
                    .text_size(px(9.5))
                    .font_weight(FontWeight::MEDIUM)
                    .child("Active"),
            );
    }

    div()
        .relative()
        .w_full()
        .px(px(8.0))
        .py(px(11.0))
        .rounded(px(RADIUS_BUTTON))
        .overflow_hidden()
        .bg(linear_gradient(
            180.0,
            linear_color_stop(TOKENS.key_top, 0.0),
            linear_color_stop(TOKENS.key_bottom, 1.0),
        ))
        .hover(|s| s.bg(TOKENS.elevated_hover))
        .border_1()
        .border_color(TOKENS.key_border)
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(5.0))
        .child(top_edge_highlight(rgba(0xFFFF_FF10)))
        .child(deck_icon(icon, IconSize::Md, TOKENS.ink_secondary))
        .child(
            div()
                .font_family(FONT_BODY)
                .text_color(rgb(0xEAE7E1))
                .text_size(px(11.0))
                .font_weight(FontWeight::MEDIUM)
                .child(label.to_owned()),
        )
}

/// Compact sculpted warm-graphite button for secondary toolbar & modal actions.
pub fn sculpted_secondary_button_body(icon: Option<DeckIcon>, label: &str, icon_tint: Rgba) -> Div {
    let mut btn = div()
        .relative()
        .h(px(32.0))
        .px(px(13.0))
        .rounded(px(RADIUS_CONTROL))
        .overflow_hidden()
        .bg(linear_gradient(
            180.0,
            linear_color_stop(TOKENS.key_top, 0.0),
            linear_color_stop(TOKENS.key_bottom, 1.0),
        ))
        .hover(|s| s.bg(TOKENS.elevated_hover))
        .border_1()
        .border_color(TOKENS.key_border)
        .flex()
        .items_center()
        .justify_center()
        .gap(px(6.0))
        .child(top_edge_highlight(rgba(0xFFFF_FF10)));

    if let Some(ic) = icon {
        btn = btn.child(deck_icon(ic, IconSize::Sm, icon_tint));
    }
    btn.child(
        div()
            .font_family(FONT_BODY)
            .text_color(rgb(0xEAE7E1))
            .text_size(px(12.0))
            .font_weight(FontWeight::MEDIUM)
            .child(label.to_owned()),
    )
}

/// Neutral `#1C1F26` option chip with a `6px` precision dot when selected (zero translucent fills).
pub fn neutral_option_chip(
    id: impl Into<gpui::ElementId>,
    label: impl Into<String>,
    selected: bool,
    dot_color: Rgba,
) -> Stateful<Div> {
    let mut el = div()
        .id(id)
        .px(px(11.0))
        .py(px(5.0))
        .rounded(px(RADIUS_PILL))
        .cursor_pointer()
        .bg(if selected {
            TOKENS.elevated
        } else {
            TOKENS.status_surface
        })
        .hover(|s| s.bg(TOKENS.elevated_hover))
        .border_1()
        .border_color(if selected {
            TOKENS.focus_border
        } else {
            TOKENS.gridline
        })
        .flex()
        .items_center()
        .gap(px(6.0));

    if selected {
        el = el.child(dot_6px(dot_color));
    }
    el.child(
        div()
            .font_family(FONT_BODY)
            .text_color(if selected {
                TOKENS.ink_primary
            } else {
                TOKENS.ink_secondary
            })
            .text_size(px(11.5))
            .font_weight(FontWeight::MEDIUM)
            .child(label.into()),
    )
}

/// Subtle 3-bar audio level indicator inside the neutral Auto-Follow status control.
pub fn voice_level_meter(active_bars: usize, listening: bool) -> Div {
    let mut meter = div().h(px(13.0)).flex().items_end().gap(px(2.0));
    for (idx, h) in [5.0, 9.0, 12.0].into_iter().enumerate() {
        meter = meter.child(div().w(px(2.0)).h(px(h)).rounded(px(RADIUS_PILL)).bg(
            if listening && idx < active_bars {
                TOKENS.signal_voice
            } else {
                TOKENS.gridline
            },
        ));
    }
    meter
}

/// Neutral-surface Auto-Follow control (`#1C1F26`, `1px` `#2B2F3A` border, `6px` precision dot).
pub fn render_auto_follow_pill(
    id_prefix: &'static str,
    mode: VoiceSyncMode,
    active_bars: usize,
    motion_mode: MotionMode,
) -> AnyElement {
    let listening = mode != VoiceSyncMode::Off;
    let dot_color = match mode {
        VoiceSyncMode::Off => rgb(0x565C6B),
        VoiceSyncMode::Suggest => TOKENS.signal_preview,
        VoiceSyncMode::Auto => TOKENS.signal_voice,
    };

    let mut row = div()
        .h(px(30.0))
        .px(px(11.0))
        .rounded(px(RADIUS_PILL))
        .bg(TOKENS.status_surface)
        .hover(|s| s.bg(TOKENS.elevated_hover))
        .border_1()
        .border_color(rgb(0x2B2F3A))
        .flex()
        .items_center()
        .gap(px(7.5))
        .child(tally_pulse_dot(
            format!("{id_prefix}-dot"),
            dot_color,
            listening,
            motion_mode,
        ))
        .child(deck_icon(
            if listening {
                DeckIcon::MicrophoneFill
            } else {
                DeckIcon::Microphone
            },
            IconSize::Sm,
            if listening {
                dot_color
            } else {
                TOKENS.ink_secondary
            },
        ))
        .child(
            div()
                .font_family(FONT_BODY)
                .text_color(if listening {
                    rgb(0xEAE7E1)
                } else {
                    TOKENS.ink_secondary
                })
                .text_size(px(12.0))
                .font_weight(FontWeight::MEDIUM)
                .child(mode.pill_label()),
        );

    if listening {
        row = row.child(voice_level_meter(active_bars, true));
    }
    row.into_any_element()
}

/// Neutral-surface top-bar display control (`Projector Ready`, `Stage Screen`, `Projector Off`).
pub fn display_status_pill(icon: DeckIcon, label: &str, connected: bool, dot_color: Rgba) -> Div {
    div()
        .h(px(30.0))
        .px(px(11.0))
        .rounded(px(RADIUS_PILL))
        .bg(TOKENS.status_surface)
        .hover(|s| s.bg(TOKENS.elevated_hover))
        .border_1()
        .border_color(rgb(0x2B2F3A))
        .flex()
        .items_center()
        .gap(px(7.0))
        .child(dot_6px(if connected { dot_color } else { rgb(0x565C6B) }))
        .child(deck_icon(
            icon,
            IconSize::Sm,
            if connected {
                rgb(0xEAE7E1)
            } else {
                TOKENS.ink_muted
            },
        ))
        .child(
            div()
                .font_family(FONT_BODY)
                .text_color(if connected {
                    rgb(0xEAE7E1)
                } else {
                    TOKENS.ink_secondary
                })
                .text_size(px(12.0))
                .font_weight(FontWeight::MEDIUM)
                .child(label.to_owned()),
        )
}

/// Clean bottom-center caption on a suggested next slide (`Next up · Space`) — zero pill border.
pub fn next_up_caption() -> Div {
    div()
        .font_family(FONT_BODY)
        .text_color(TOKENS.ink_secondary)
        .text_size(px(10.5))
        .font_weight(FontWeight::MEDIUM)
        .child("Next up · Space")
}
