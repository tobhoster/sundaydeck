use gpui::{
    Context, Div, InteractiveElement, ParentElement, StatefulInteractiveElement, Styled, div, px,
};
use sundaydeck_core::{OutputOverride, PresentationCommand};

use crate::design::icons::{DeckIcon, IconSize, deck_icon};
use crate::design::primitives::{native_screen_key_body, neutral_option_chip};
use crate::design::tokens::{RADIUS_CONTROL, TOKENS};
use crate::views::operator_workspace::OperatorWorkspace;

/// Sculpted Native Screen Controls (`Hide Words`, `Black Screen`, `Show Logo`).
/// Idle keys use raised warm-graphite gradients; engaged overrides switch to a pressed-in
/// recessed latched state with crisp `#D97736` border, `6px` indicator dot, and `Active` label.
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
            .child(native_screen_key_body(icon, label, is_on))
    };

    div()
        .w_full()
        .flex()
        .items_stretch()
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

/// Private Stage Screen note presets docked beneath the live slide list.
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
            neutral_option_chip(id, label, is_on, TOKENS.signal_preview).on_click(cx.listener(
                move |this, _, _, cx| {
                    let next = if is_on { None } else { Some(label.to_owned()) };
                    this.store.update(cx, |s, cx| {
                        let _ = s.dispatch(PresentationCommand::SetStageMessage(next), cx);
                    });
                },
            )),
        );
    }

    if active_note.is_some() {
        row = row.child(
            div()
                .id("stage-cue-clear")
                .w(px(26.0))
                .h(px(26.0))
                .rounded(px(RADIUS_CONTROL))
                .cursor_pointer()
                .bg(TOKENS.status_surface)
                .hover(|s| s.bg(TOKENS.elevated_hover))
                .border_1()
                .border_color(TOKENS.gridline)
                .flex()
                .items_center()
                .justify_center()
                .on_click(cx.listener(|this, _, _, cx| {
                    this.store.update(cx, |s, cx| {
                        let _ = s.dispatch(PresentationCommand::SetStageMessage(None), cx);
                    });
                }))
                .child(deck_icon(DeckIcon::X, IconSize::Sm, TOKENS.signal_live_dot)),
        );
    }

    div()
        .px(px(14.0))
        .py(px(10.0))
        .border_t_1()
        .border_color(TOKENS.hairline)
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
