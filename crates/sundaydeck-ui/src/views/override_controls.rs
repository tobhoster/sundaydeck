use gpui::{
    Context, Div, FontWeight, InteractiveElement, ParentElement, StatefulInteractiveElement,
    Styled, div, px,
};
use sundaydeck_core::{OutputOverride, PresentationCommand};

use crate::design::icons::{DeckIcon, IconSize, deck_icon};
use crate::design::primitives::hardware_switcher_key_body;
use crate::design::tokens::{FONT_MONO, RADIUS_CONTROL, TOKENS};
use crate::views::operator_workspace::OperatorWorkspace;

/// Tactile Broadcast Switcher Override Strip (`GO LIVE`, `CLEAR`, `BLACK`, `LOGO`):
/// Physical ATEM-style keys with embossed top highlights, Phosphor icons, monospace `F1-F5`
/// keycaps, Tally Red `GO LIVE`, and high-vis Warning Orange (`#F97316`) for active overrides.
pub fn render_override_buttons(
    active: OutputOverride,
    has_live: bool,
    cx: &mut Context<OperatorWorkspace>,
) -> Div {
    let make_override_key = |id: &'static str,
                             icon: DeckIcon,
                             label: &'static str,
                             hotkey: &'static str,
                             target: OutputOverride,
                             cx: &mut Context<OperatorWorkspace>| {
        let is_on = active == target;
        div()
            .id(id)
            .flex_1()
            .cursor_pointer()
            .on_click(cx.listener(move |this, _, _, cx| {
                this.store.update(cx, |s, cx| {
                    let _ = s.dispatch(PresentationCommand::ToggleOverride(target), cx);
                });
            }))
            .child(hardware_switcher_key_body(
                icon,
                label,
                hotkey,
                is_on,
                TOKENS.signal_override,
            ))
    };

    div()
        .w_full()
        .flex()
        .items_center()
        .gap(px(6.0))
        .child(
            div()
                .id("deck-go-live")
                .flex_1()
                .cursor_pointer()
                .on_click(cx.listener(|this, _, _, cx| {
                    this.store.update(cx, |s, cx| {
                        let _ = s.dispatch(PresentationCommand::GoLive, cx);
                    });
                }))
                .child(hardware_switcher_key_body(
                    DeckIcon::PlayFill,
                    "LIVE",
                    "F5",
                    has_live && active == OutputOverride::None,
                    TOKENS.signal_live,
                )),
        )
        .child(make_override_key(
            "override-clear-text",
            DeckIcon::TextTSlash,
            "CLEAR",
            "F1",
            OutputOverride::ClearText,
            cx,
        ))
        .child(make_override_key(
            "override-black",
            DeckIcon::EyeSlash,
            "BLACK",
            "F2",
            OutputOverride::Black,
            cx,
        ))
        .child(make_override_key(
            "override-logo",
            DeckIcon::Image,
            "LOGO",
            "F3",
            OutputOverride::Logo,
            cx,
        ))
}

/// Compact private Stage Monitor cue toggles docked beneath the live slide queue.
pub fn render_stage_cue_strip(
    active_note: Option<&str>,
    cx: &mut Context<OperatorWorkspace>,
) -> Div {
    let presets = [
        ("stage-cue-repeat", "REPEAT CH"),
        ("stage-cue-2min", "2 MIN"),
    ];

    let mut row = div().flex().items_center().gap(px(4.0));
    for (id, label) in presets {
        let full_text = if label == "REPEAT CH" {
            "Repeat Chorus"
        } else {
            "2 Min Left"
        };
        let is_on = active_note == Some(full_text);
        row = row.child(
            div()
                .id(id)
                .px(px(6.0))
                .py(px(3.0))
                .rounded(px(RADIUS_CONTROL))
                .cursor_pointer()
                .bg(if is_on {
                    TOKENS.signal_preview_tint
                } else {
                    TOKENS.elevated
                })
                .hover(|s| s.bg(TOKENS.elevated_hover))
                .border_1()
                .border_color(if is_on {
                    TOKENS.signal_preview
                } else {
                    TOKENS.gridline
                })
                .font_family(FONT_MONO)
                .text_color(if is_on {
                    TOKENS.signal_preview
                } else {
                    TOKENS.ink_secondary
                })
                .text_size(px(9.5))
                .font_weight(FontWeight::BOLD)
                .on_click(cx.listener(move |this, _, _, cx| {
                    let next = if is_on {
                        None
                    } else {
                        Some(full_text.to_owned())
                    };
                    this.store.update(cx, |s, cx| {
                        let _ = s.dispatch(PresentationCommand::SetStageMessage(next), cx);
                    });
                }))
                .child(label),
        );
    }

    if active_note.is_some() {
        row = row.child(
            div()
                .id("stage-cue-clear")
                .px(px(5.0))
                .py(px(3.0))
                .rounded(px(RADIUS_CONTROL))
                .cursor_pointer()
                .bg(TOKENS.signal_live_tint)
                .border_1()
                .border_color(TOKENS.signal_live)
                .flex()
                .items_center()
                .on_click(cx.listener(|this, _, _, cx| {
                    this.store.update(cx, |s, cx| {
                        let _ = s.dispatch(PresentationCommand::SetStageMessage(None), cx);
                    });
                }))
                .child(deck_icon(DeckIcon::X, IconSize::Sm, TOKENS.signal_live)),
        );
    }

    div()
        .px(px(12.0))
        .py(px(8.0))
        .border_t_1()
        .border_color(TOKENS.gridline)
        .flex()
        .items_center()
        .justify_between()
        .gap(px(6.0))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(5.0))
                .child(deck_icon(
                    DeckIcon::ChatCenteredText,
                    IconSize::Sm,
                    TOKENS.ink_muted,
                ))
                .child(
                    div()
                        .font_family(FONT_MONO)
                        .text_color(TOKENS.ink_muted)
                        .text_size(px(9.5))
                        .font_weight(FontWeight::BOLD)
                        .child("STAGE CUE"),
                ),
        )
        .child(row)
}
