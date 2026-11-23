use gpui::{
    AnyElement, Div, FontWeight, IntoElement, ParentElement, Rgba, Styled, div, px, rgb, rgba,
};

use crate::design::icons::{DeckIcon, IconSize, deck_icon};
use crate::design::motion::{MotionMode, tally_pulse_dot};
use crate::design::tokens::{FONT_DISPLAY, FONT_MONO, RADIUS_CONTROL, RADIUS_TILE, TOKENS};

/// Voice-Sync operating mode surfaced in the Top Hardware Status Bar and Design Gallery.
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
            Self::Off => "OFF",
            Self::Suggest => "SUGGEST",
            Self::Auto => "AUTO",
        }
    }
}

/// Crisp monospace keyboard keycap (`SPACE`, `F1`, `F5`, `CMD+K`).
pub fn keycap(label: &str, active_dark_ink: bool) -> Div {
    div()
        .px(px(5.0))
        .py(px(1.0))
        .rounded(px(RADIUS_CONTROL))
        .bg(if active_dark_ink {
            rgba(0x0A0B_0D44)
        } else {
            TOKENS.canvas
        })
        .border_1()
        .border_color(if active_dark_ink {
            rgba(0x0A0B_0D66)
        } else {
            TOKENS.gridline
        })
        .font_family(FONT_MONO)
        .text_color(if active_dark_ink {
            TOKENS.canvas
        } else {
            TOKENS.ink_secondary
        })
        .text_size(px(9.5))
        .font_weight(FontWeight::MEDIUM)
        .child(label.to_owned())
}

/// Compact monospace section header tag (`V1`, `CH`, `BR`, `SCR`).
pub fn section_tag(code: &str, strip_color: Rgba) -> Div {
    div()
        .px(px(5.0))
        .py(px(1.0))
        .rounded(px(RADIUS_CONTROL))
        .bg(strip_color.alpha(0.32))
        .border_1()
        .border_color(strip_color.alpha(0.65))
        .font_family(FONT_MONO)
        .text_color(TOKENS.ink_primary)
        .text_size(px(9.5))
        .font_weight(FontWeight::BOLD)
        .child(code.to_owned())
}

/// Hardware broadcast switcher key (Blackmagic ATEM style):
/// matte elevated cap with 1px embossed top bevel, crisp Phosphor icon, uppercase display label,
/// and tabular monospace hotkey keycap.
pub fn hardware_switcher_key_body(
    icon: DeckIcon,
    label: &str,
    hotkey: &str,
    is_active: bool,
    active_signal: Rgba,
) -> Div {
    let fg = if is_active {
        TOKENS.canvas
    } else {
        TOKENS.ink_primary
    };
    let bg = if is_active {
        active_signal
    } else {
        TOKENS.elevated
    };
    let border = if is_active {
        active_signal
    } else {
        TOKENS.gridline
    };

    div()
        .w_full()
        .rounded(px(RADIUS_CONTROL))
        .bg(bg)
        .border_1()
        .border_color(border)
        .overflow_hidden()
        .flex()
        .flex_col()
        // Embossed top bezel highlight line
        .child(div().w_full().h(px(1.0)).bg(if is_active {
            rgba(0xFFFF_FF40)
        } else {
            TOKENS.emboss_top
        }))
        .child(
            div()
                .w_full()
                .px(px(6.0))
                .py(px(7.0))
                .flex()
                .flex_col()
                .items_center()
                .gap(px(4.0))
                .child(deck_icon(icon, IconSize::Lg, fg))
                .child(
                    div()
                        .font_family(FONT_DISPLAY)
                        .text_color(fg)
                        .text_size(px(10.5))
                        .font_weight(FontWeight::BOLD)
                        .child(label.to_owned()),
                )
                .child(keycap(hotkey, is_active)),
        )
}

/// 6-tick vertical audio input level bar meter (2px wide ticks, strictly operational signal color).
pub fn voice_level_meter(active_ticks: usize, listening: bool) -> Div {
    let heights = [4.0, 6.0, 9.0, 11.0, 13.0, 10.0];
    let mut meter = div().h(px(14.0)).flex().items_end().gap(px(2.0));
    for (idx, h) in heights.into_iter().enumerate() {
        let lit = listening && idx < active_ticks;
        let tick_color = if !lit {
            TOKENS.gridline
        } else if idx >= 5 {
            TOKENS.signal_preview
        } else {
            TOKENS.signal_voice
        };
        meter = meter.child(div().w(px(2.0)).h(px(h)).rounded(px(1.0)).bg(tick_color));
    }
    meter
}

/// Static display version of the Voice-Sync Hardware Pill (used in Design Gallery and telemetry).
pub fn render_voice_sync_pill_display(
    id_prefix: &'static str,
    mode: VoiceSyncMode,
    active_ticks: usize,
    confidence_pct: Option<u8>,
    device_name: &str,
    motion_mode: MotionMode,
) -> AnyElement {
    let listening = mode != VoiceSyncMode::Off;
    let signal_color = match mode {
        VoiceSyncMode::Off => TOKENS.ink_muted,
        VoiceSyncMode::Suggest => TOKENS.signal_preview,
        VoiceSyncMode::Auto => TOKENS.signal_voice,
    };

    let mut segs = div()
        .flex()
        .items_center()
        .bg(TOKENS.canvas)
        .border_1()
        .border_color(TOKENS.gridline)
        .rounded(px(RADIUS_CONTROL));

    for option in VoiceSyncMode::ALL {
        let active = option == mode;
        let seg_bg = if !active {
            rgba(0x0000_0000)
        } else {
            match option {
                VoiceSyncMode::Off => TOKENS.elevated,
                VoiceSyncMode::Suggest => TOKENS.signal_preview_tint,
                VoiceSyncMode::Auto => TOKENS.signal_voice_tint,
            }
        };
        let seg_fg = if !active {
            TOKENS.ink_muted
        } else {
            match option {
                VoiceSyncMode::Off => TOKENS.ink_primary,
                VoiceSyncMode::Suggest => TOKENS.signal_preview,
                VoiceSyncMode::Auto => TOKENS.signal_voice,
            }
        };
        segs = segs.child(
            div()
                .px(px(6.0))
                .py(px(2.0))
                .bg(seg_bg)
                .font_family(FONT_MONO)
                .text_color(seg_fg)
                .text_size(px(9.5))
                .font_weight(FontWeight::BOLD)
                .child(option.label()),
        );
    }

    let mut row = div()
        .h(px(24.0))
        .px(px(8.0))
        .rounded(px(RADIUS_TILE))
        .bg(TOKENS.elevated)
        .border_1()
        .border_color(if listening {
            signal_color.alpha(0.55)
        } else {
            TOKENS.gridline
        })
        .flex()
        .items_center()
        .gap(px(8.0))
        .child(tally_pulse_dot(
            format!("{id_prefix}-dot"),
            signal_color,
            listening,
            motion_mode,
        ))
        .child(deck_icon(
            if listening {
                DeckIcon::MicrophoneFill
            } else {
                DeckIcon::MicrophoneSlash
            },
            IconSize::Sm,
            signal_color,
        ))
        .child(voice_level_meter(active_ticks, listening))
        .child(segs)
        .child(
            div()
                .font_family(FONT_MONO)
                .text_color(if listening {
                    TOKENS.ink_secondary
                } else {
                    TOKENS.ink_muted
                })
                .text_size(px(10.0))
                .child(device_name.to_owned()),
        );

    if let Some(pct) = confidence_pct {
        row = row.child(
            div()
                .px(px(5.0))
                .py(px(1.0))
                .rounded(px(RADIUS_CONTROL))
                .bg(signal_color.alpha(0.18))
                .font_family(FONT_MONO)
                .text_color(signal_color)
                .text_size(px(10.0))
                .font_weight(FontWeight::BOLD)
                .child(format!("{pct}%")),
        );
    }

    row.into_any_element()
}

/// Monospace display connection badge for the 36px hardware status bar (`AUD: 1920x1080`).
pub fn display_status_badge(
    icon: DeckIcon,
    code: &str,
    detail: &str,
    connected: bool,
    signal: Rgba,
) -> Div {
    div()
        .h(px(24.0))
        .px(px(7.0))
        .rounded(px(RADIUS_CONTROL))
        .bg(if connected {
            signal.alpha(0.16)
        } else {
            TOKENS.elevated
        })
        .border_1()
        .border_color(if connected {
            signal.alpha(0.75)
        } else {
            TOKENS.gridline
        })
        .flex()
        .items_center()
        .gap(px(5.0))
        .child(deck_icon(
            icon,
            IconSize::Sm,
            if connected { signal } else { TOKENS.ink_muted },
        ))
        .child(
            div()
                .font_family(FONT_MONO)
                .text_color(if connected {
                    TOKENS.ink_primary
                } else {
                    TOKENS.ink_secondary
                })
                .text_size(px(10.0))
                .font_weight(FontWeight::BOLD)
                .child(format!("{code}:")),
        )
        .child(
            div()
                .font_family(FONT_MONO)
                .text_color(if connected { signal } else { TOKENS.ink_muted })
                .text_size(px(10.0))
                .child(detail.to_owned()),
        )
}

/// Amber corner bracket marker indicating Voice-Sync Suggest Mode's predicted next slide.
pub fn suggest_corner_bracket() -> Div {
    div()
        .absolute()
        .top(px(0.0))
        .right(px(0.0))
        .w(px(14.0))
        .h(px(14.0))
        .border_t_2()
        .border_r_2()
        .border_color(TOKENS.signal_preview)
        .bg(rgb(0x000000).alpha(0.0))
}
