use gpui::{
    Context, Div, FontWeight, InteractiveElement, ParentElement, StatefulInteractiveElement,
    Styled, div, px, rgb, rgba,
};
use sundaydeck_core::{OutputOverride, PresentationCommand};

use crate::palette::{
    BG_CARD_HOVER, BG_ELEVATED, BORDER_STRONG, BORDER_SUBTLE, OVERRIDE_BLACK, OVERRIDE_CLEAR,
    OVERRIDE_LOGO, STAGE_NEXT_ACCENT, TEXT_MUTED, TEXT_PRIMARY, TEXT_SECONDARY, color,
};
use crate::views::operator_workspace::OperatorWorkspace;

/// Pinned instant recovery override controls (`Clear Text`, `Black`, `Logo`).
pub fn render_override_buttons(active: OutputOverride, cx: &mut Context<OperatorWorkspace>) -> Div {
    let make_btn = |id: &'static str,
                    label: &'static str,
                    hotkey: &'static str,
                    target: OutputOverride,
                    active_hex: u32,
                    cx: &mut Context<OperatorWorkspace>| {
        let is_on = active == target;
        div()
            .id(id)
            .flex_1()
            .py(px(8.0))
            .px(px(8.0))
            .rounded(px(7.0))
            .cursor_pointer()
            .bg(if is_on {
                color(active_hex)
            } else {
                color(BG_ELEVATED)
            })
            .hover(|s| if is_on { s } else { s.bg(color(BG_CARD_HOVER)) })
            .border_1()
            .border_color(if is_on {
                color(active_hex)
            } else {
                color(BORDER_STRONG)
            })
            .flex()
            .flex_col()
            .items_center()
            .gap(px(2.0))
            .on_click(cx.listener(move |this, _, _, cx| {
                this.store.update(cx, |s, cx| {
                    let _ = s.dispatch(PresentationCommand::ToggleOverride(target), cx);
                });
            }))
            .child(
                div()
                    .text_color(if is_on {
                        rgb(0x0B0F17)
                    } else {
                        color(TEXT_PRIMARY)
                    })
                    .text_size(px(12.0))
                    .font_weight(FontWeight::BOLD)
                    .child(label),
            )
            .child(
                div()
                    .text_color(if is_on {
                        rgba(0x0B0F_17BB)
                    } else {
                        color(TEXT_MUTED)
                    })
                    .text_size(px(10.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(hotkey),
            )
    };

    div()
        .w_full()
        .flex()
        .items_center()
        .gap(px(8.0))
        .child(make_btn(
            "override-clear-text",
            "Clear Text",
            "F1",
            OutputOverride::ClearText,
            OVERRIDE_CLEAR,
            cx,
        ))
        .child(make_btn(
            "override-black",
            "Black",
            "F2",
            OutputOverride::Black,
            OVERRIDE_BLACK,
            cx,
        ))
        .child(make_btn(
            "override-logo",
            "Logo",
            "F3",
            OutputOverride::Logo,
            OVERRIDE_LOGO,
            cx,
        ))
}

/// Quick private Stage Monitor cue toggles (never shown on Audience output).
pub fn render_stage_cue_strip(
    active_note: Option<&str>,
    cx: &mut Context<OperatorWorkspace>,
) -> Div {
    let presets = [
        ("stage-cue-repeat", "Repeat Chorus"),
        ("stage-cue-2min", "2 Min Left"),
    ];

    let mut row = div().flex().items_center().gap(px(6.0));
    for (id, text) in presets {
        let is_on = active_note == Some(text);
        row = row.child(
            div()
                .id(id)
                .px(px(8.0))
                .py(px(4.0))
                .rounded(px(5.0))
                .cursor_pointer()
                .bg(if is_on {
                    rgba(0xF59E_0B30)
                } else {
                    color(BG_ELEVATED)
                })
                .border_1()
                .border_color(if is_on {
                    color(STAGE_NEXT_ACCENT)
                } else {
                    color(BORDER_STRONG)
                })
                .text_color(if is_on {
                    rgb(0xFEF3C7)
                } else {
                    color(TEXT_SECONDARY)
                })
                .text_size(px(11.0))
                .on_click(cx.listener(move |this, _, _, cx| {
                    let next = if is_on { None } else { Some(text.to_owned()) };
                    this.store.update(cx, |s, cx| {
                        let _ = s.dispatch(PresentationCommand::SetStageMessage(next), cx);
                    });
                }))
                .child(text),
        );
    }

    if active_note.is_some() {
        row = row.child(
            div()
                .id("stage-cue-clear")
                .px(px(8.0))
                .py(px(4.0))
                .rounded(px(5.0))
                .cursor_pointer()
                .bg(rgba(0xEF44_4422))
                .text_color(rgb(0xFCA5A5))
                .text_size(px(11.0))
                .on_click(cx.listener(|this, _, _, cx| {
                    this.store.update(cx, |s, cx| {
                        let _ = s.dispatch(PresentationCommand::SetStageMessage(None), cx);
                    });
                }))
                .child("Clear"),
        );
    }

    div()
        .px(px(14.0))
        .py(px(10.0))
        .border_t_1()
        .border_color(color(BORDER_SUBTLE))
        .flex()
        .items_center()
        .justify_between()
        .gap(px(8.0))
        .child(
            div()
                .text_color(color(TEXT_MUTED))
                .text_size(px(10.5))
                .font_weight(FontWeight::BOLD)
                .child("STAGE NOTE"),
        )
        .child(row)
}
