use gpui::{
    Context, Div, FontWeight, InteractiveElement, ParentElement, StatefulInteractiveElement,
    Styled, div, px,
};
use sundaydeck_core::{OutputOverride, PresentationCommand};

use crate::design::icons::{DeckIcon, IconSize, deck_icon};
use crate::design::primitives::volunteer_action_button_body;
use crate::design::tokens::{FONT_BODY, RADIUS_CONTROL, RADIUS_PILL, TOKENS};
use crate::views::operator_workspace::OperatorWorkspace;

/// Calm, Plain-English Screen Controls (`Hide Words`, `Black Screen`, `Show Logo`).
/// Global hotkeys (F1-F3, F5) stay wired, without cluttering the button faces.
pub fn render_override_buttons(
    active: OutputOverride,
    _has_live: bool,
    cx: &mut Context<OperatorWorkspace>,
) -> Div {
    let make_override_btn = |id: &'static str,
                             icon: DeckIcon,
                             label: &'static str,
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
            .child(volunteer_action_button_body(
                icon,
                label,
                is_on,
                TOKENS.signal_override,
            ))
    };

    div()
        .w_full()
        .flex()
        .items_center()
        .gap(px(8.0))
        .child(make_override_btn(
            "override-clear-text",
            DeckIcon::TextTSlash,
            "Hide Words",
            OutputOverride::ClearText,
            cx,
        ))
        .child(make_override_btn(
            "override-black",
            DeckIcon::EyeSlash,
            "Black Screen",
            OutputOverride::Black,
            cx,
        ))
        .child(make_override_btn(
            "override-logo",
            DeckIcon::Image,
            "Show Logo",
            OutputOverride::Logo,
            cx,
        ))
}

/// Friendly private Stage Screen note presets docked beneath the live slide list.
pub fn render_stage_cue_strip(
    active_note: Option<&str>,
    cx: &mut Context<OperatorWorkspace>,
) -> Div {
    let presets = [
        ("stage-cue-repeat", "Repeat Chorus"),
        ("stage-cue-2min", "2 min left"),
    ];

    let mut row = div().flex().items_center().gap(px(6.0));
    for (id, label) in presets {
        let is_on = active_note == Some(label);
        row = row.child(
            div()
                .id(id)
                .px(px(10.0))
                .py(px(4.5))
                .rounded(px(RADIUS_PILL))
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
                .font_family(FONT_BODY)
                .text_color(if is_on {
                    TOKENS.signal_preview
                } else {
                    TOKENS.ink_secondary
                })
                .text_size(px(11.0))
                .font_weight(FontWeight::MEDIUM)
                .on_click(cx.listener(move |this, _, _, cx| {
                    let next = if is_on { None } else { Some(label.to_owned()) };
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
                .px(px(6.0))
                .py(px(4.5))
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
        .px(px(14.0))
        .py(px(10.0))
        .border_t_1()
        .border_color(TOKENS.gridline)
        .flex()
        .items_center()
        .justify_between()
        .gap(px(8.0))
        .child(div().flex().items_center().gap(px(6.0)).child(deck_icon(
            DeckIcon::ChatCenteredText,
            IconSize::Md,
            TOKENS.ink_secondary,
        )))
        .child(row)
}
