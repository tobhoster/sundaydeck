use gpui::{
    AnyElement, Div, FontWeight, InteractiveElement, IntoElement, ParentElement, Rgba, Styled, div,
    px, rgba,
};
use sundaydeck_core::SectionKind;

use crate::design::icons::{DeckIcon, IconSize, deck_icon};
use crate::design::motion::{MotionMode, tally_pulse_dot};
use crate::design::tokens::{
    FONT_BODY, FONT_MONO, RADIUS_CONTROL, RADIUS_PILL, TOKENS, section_friendly_label,
    section_pill_colors,
};

/// Plain-English Auto-Follow operating mode surfaced in the Top Bar popover and Design Gallery.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VoiceSyncMode {
    #[default]
    Off,
    Suggest,
    Auto,
}

impl VoiceSyncMode {
    pub const ALL: [Self; 3] = [Self::Off, Self::Suggest, Self::Auto];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::Suggest => "Suggest next slide",
            Self::Auto => "Advance automatically",
        }
    }

    pub const fn pill_label(self) -> &'static str {
        match self {
            Self::Off => "Auto-Follow",
            Self::Suggest => "Auto-Follow: Suggest",
            Self::Auto => "Auto-Follow: On",
        }
    }
}

/// Subtle optional keyboard shortcut hint pill (used only in help/tooltips, never stamped on button faces).
pub fn keycap(label: &str, active_dark_ink: bool) -> Div {
    div()
        .px(px(6.0))
        .py(px(1.5))
        .rounded(px(4.0))
        .bg(if active_dark_ink {
            rgba(0x1314_1744)
        } else {
            TOKENS.elevated
        })
        .border_1()
        .border_color(if active_dark_ink {
            rgba(0x1314_1766)
        } else {
            TOKENS.gridline
        })
        .font_family(FONT_MONO)
        .text_color(if active_dark_ink {
            TOKENS.canvas
        } else {
            TOKENS.ink_secondary
        })
        .text_size(px(10.0))
        .font_weight(FontWeight::MEDIUM)
        .child(label.to_owned())
}

/// Soft rounded pill badge (`Live`, `Next Up`, `Listening`, `Verse 1`, `Chorus`, `Romans 8:28`).
pub fn soft_pill(label: &str, bg: Rgba, fg: Rgba) -> Div {
    div()
        .px(px(9.0))
        .py(px(2.5))
        .rounded(px(RADIUS_PILL))
        .bg(bg)
        .font_family(FONT_BODY)
        .text_color(fg)
        .text_size(px(10.5))
        .font_weight(FontWeight::SEMIBOLD)
        .child(label.to_owned())
}

/// Human-readable softly tinted section pill tag (`Verse 1`, `Chorus`, `Bridge`, `Romans 8:28`).
pub fn section_pill(label: &str, kind: Option<SectionKind>) -> Div {
    let (bg, fg) = section_pill_colors(label, kind);
    soft_pill(&section_friendly_label(label, kind), bg, fg)
}

/// Warm, approachable screen-control button (`Show on Screen`, `Hide Words`, `Black Screen`, `Show Logo`).
/// Clean Sentence-case label + Phosphor icon + comfortable hit target; shortcuts stay out of the face.
pub fn volunteer_action_button_body(
    icon: DeckIcon,
    label: &str,
    is_active: bool,
    active_signal: Rgba,
) -> Div {
    let bg = if is_active {
        active_signal
    } else {
        TOKENS.elevated
    };
    let fg = TOKENS.ink_primary;
    let icon_col = if is_active {
        TOKENS.ink_primary
    } else {
        TOKENS.ink_secondary
    };
    let border = if is_active {
        active_signal
    } else {
        TOKENS.gridline
    };

    div()
        .w_full()
        .px(px(8.0))
        .py(px(10.0))
        .rounded(px(RADIUS_CONTROL))
        .bg(bg)
        .hover(|s| {
            if is_active {
                s.bg(active_signal)
            } else {
                s.bg(TOKENS.elevated_hover)
            }
        })
        .border_1()
        .border_color(border)
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(5.0))
        .child(deck_icon(icon, IconSize::Md, icon_col))
        .child(
            div()
                .font_family(FONT_BODY)
                .text_color(fg)
                .text_size(px(11.0))
                .font_weight(FontWeight::SEMIBOLD)
                .child(label.to_owned()),
        )
}

/// Subtle 3-bar soft audio level indicator for the Auto-Follow pill.
pub fn voice_level_meter(active_bars: usize, listening: bool) -> Div {
    let heights = [5.0, 9.0, 12.0];
    let mut meter = div().h(px(13.0)).flex().items_end().gap(px(2.5));
    for (idx, h) in heights.into_iter().enumerate() {
        let lit = listening && idx < active_bars;
        let bar_color = if lit {
            TOKENS.signal_voice
        } else {
            TOKENS.gridline
        };
        meter = meter.child(
            div()
                .w(px(2.5))
                .h(px(h))
                .rounded(px(RADIUS_PILL))
                .bg(bar_color),
        );
    }
    meter
}

/// Clean friendly Auto-Follow status pill (used in Top Bar and Design Gallery).
pub fn render_auto_follow_pill(
    id_prefix: &'static str,
    mode: VoiceSyncMode,
    active_bars: usize,
    motion_mode: MotionMode,
) -> AnyElement {
    let listening = mode != VoiceSyncMode::Off;
    let signal_color = match mode {
        VoiceSyncMode::Off => TOKENS.ink_muted,
        VoiceSyncMode::Suggest => TOKENS.signal_preview,
        VoiceSyncMode::Auto => TOKENS.signal_voice,
    };

    let mut row = div()
        .h(px(30.0))
        .px(px(12.0))
        .rounded(px(RADIUS_PILL))
        .bg(if listening {
            signal_color.alpha(0.14)
        } else {
            TOKENS.elevated
        })
        .border_1()
        .border_color(if listening {
            signal_color.alpha(0.45)
        } else {
            TOKENS.gridline
        })
        .flex()
        .items_center()
        .gap(px(8.0))
        .child(deck_icon(
            if listening {
                DeckIcon::MicrophoneFill
            } else {
                DeckIcon::Microphone
            },
            IconSize::Sm,
            if listening {
                signal_color
            } else {
                TOKENS.ink_secondary
            },
        ))
        .child(
            div()
                .font_family(FONT_BODY)
                .text_color(if listening {
                    TOKENS.ink_primary
                } else {
                    TOKENS.ink_secondary
                })
                .text_size(px(12.0))
                .font_weight(FontWeight::MEDIUM)
                .child(mode.pill_label()),
        )
        .child(tally_pulse_dot(
            format!("{id_prefix}-dot"),
            signal_color,
            listening,
            motion_mode,
        ));

    if listening {
        row = row.child(voice_level_meter(active_bars, true));
    }

    row.into_any_element()
}

/// Friendly top-bar display status pill (`Projector Ready`, `Stage Screen`).
pub fn display_status_pill(icon: DeckIcon, label: &str, connected: bool, dot_color: Rgba) -> Div {
    div()
        .h(px(30.0))
        .px(px(11.0))
        .rounded(px(RADIUS_PILL))
        .bg(if connected {
            dot_color.alpha(0.14)
        } else {
            TOKENS.elevated
        })
        .border_1()
        .border_color(if connected {
            dot_color.alpha(0.45)
        } else {
            TOKENS.gridline
        })
        .flex()
        .items_center()
        .gap(px(7.0))
        .child(
            div()
                .w(px(7.0))
                .h(px(7.0))
                .rounded(px(RADIUS_PILL))
                .bg(if connected {
                    dot_color
                } else {
                    TOKENS.ink_muted
                }),
        )
        .child(deck_icon(
            icon,
            IconSize::Sm,
            if connected {
                TOKENS.ink_primary
            } else {
                TOKENS.ink_secondary
            },
        ))
        .child(
            div()
                .font_family(FONT_BODY)
                .text_color(if connected {
                    TOKENS.ink_primary
                } else {
                    TOKENS.ink_secondary
                })
                .text_size(px(12.0))
                .font_weight(FontWeight::MEDIUM)
                .child(label.to_owned()),
        )
}

/// Gentle bottom pill cue on a suggested next slide (`Next up · Space to advance`).
pub fn next_up_hint_pill() -> Div {
    div()
        .px(px(10.0))
        .py(px(3.5))
        .rounded(px(RADIUS_PILL))
        .bg(TOKENS.signal_preview_tint)
        .border_1()
        .border_color(TOKENS.signal_preview.alpha(0.5))
        .font_family(FONT_BODY)
        .text_color(TOKENS.signal_preview)
        .text_size(px(10.5))
        .font_weight(FontWeight::SEMIBOLD)
        .child("Next up · Space to advance")
}
