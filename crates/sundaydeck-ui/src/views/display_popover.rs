use gpui::{
    Context, Div, FontWeight, InteractiveElement, ParentElement, StatefulInteractiveElement,
    Styled, div, px, rgb, rgba,
};
use sundaydeck_core::PresentationCommand;

use crate::design::icons::{DeckIcon, IconSize, deck_icon};
use crate::design::primitives::{
    neutral_option_chip, primary_ivory_button_body, sculpted_secondary_button_body,
};
use crate::design::tokens::{
    FONT_BODY, FONT_DISPLAY, RADIUS_CONTROL, RADIUS_PILL, RADIUS_TILE, TOKENS,
};
use crate::views::operator_workspace::OperatorWorkspace;
use crate::windows::{ConnectedDisplayInfo, DisplayTarget, WindowManager};

/// Floating multi-display target picker for Audience Projector and Stage Confidence Monitor.
pub(crate) fn render_projector_popover(
    workspace: &OperatorWorkspace,
    cx: &mut Context<OperatorWorkspace>,
) -> Div {
    let direct_click = workspace.store.read(cx).snapshot().direct_click_live;
    let wm = workspace.windows.read(cx);
    let audience_open = wm.is_audience_open();
    let stage_open = wm.is_stage_open();
    let aud_target = wm.audience_target;
    let stg_target = wm.stage_target;
    let displays = WindowManager::connected_displays(cx);

    let aud_chips = build_display_target_chips(
        "pop-aud",
        &displays,
        aud_target,
        TOKENS.signal_voice,
        true,
        cx,
    );
    let stg_chips = build_display_target_chips(
        "pop-stg",
        &displays,
        stg_target,
        TOKENS.signal_preview,
        false,
        cx,
    );

    div()
        .w(px(340.0))
        .p(px(14.0))
        .rounded(px(RADIUS_TILE))
        .bg(TOKENS.pane)
        .border_1()
        .border_color(TOKENS.focus_border)
        .flex()
        .flex_col()
        .gap(px(10.0))
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .font_family(FONT_DISPLAY)
                        .text_color(TOKENS.ink_primary)
                        .text_size(px(13.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child("Sanctuary Displays"),
                )
                .child(
                    div()
                        .id("close-proj-popover")
                        .cursor_pointer()
                        .on_click(cx.listener(|this, _, _, cx| this.close_popover(cx)))
                        .child(deck_icon(DeckIcon::X, IconSize::Sm, TOKENS.ink_muted)),
                ),
        )
        .child(
            div()
                .font_family(FONT_BODY)
                .text_color(TOKENS.ink_secondary)
                .text_size(px(11.5))
                .child(format!(
                    "{} display{} detected · Choose target screen per output",
                    displays.len(),
                    if displays.len() == 1 { "" } else { "s" }
                )),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(5.0))
                .child(
                    div()
                        .font_family(FONT_BODY)
                        .text_color(TOKENS.ink_muted)
                        .text_size(px(10.5))
                        .font_weight(FontWeight::MEDIUM)
                        .child("Audience Projector Target"),
                )
                .child(aud_chips),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(5.0))
                .child(
                    div()
                        .font_family(FONT_BODY)
                        .text_color(TOKENS.ink_muted)
                        .text_size(px(10.5))
                        .font_weight(FontWeight::MEDIUM)
                        .child("Stage Screen Target"),
                )
                .child(stg_chips),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(8.0))
                .child(
                    div()
                        .id("pop-toggle-aud-btn")
                        .flex_1()
                        .cursor_pointer()
                        .on_click(cx.listener(|this, _, _, cx| {
                            let store = this.store.clone();
                            this.windows
                                .update(cx, |wm, cx| wm.toggle_audience(store, cx));
                        }))
                        .child(if audience_open {
                            sculpted_secondary_button_body(
                                Some(DeckIcon::Monitor),
                                "Close Projector",
                                TOKENS.signal_live_dot,
                            )
                        } else {
                            primary_ivory_button_body(
                                Some(DeckIcon::MonitorFill),
                                "Open Projector",
                                true,
                            )
                        }),
                )
                .child(
                    div()
                        .id("pop-toggle-stg-btn")
                        .flex_1()
                        .cursor_pointer()
                        .on_click(cx.listener(|this, _, _, cx| {
                            let store = this.store.clone();
                            this.windows.update(cx, |wm, cx| wm.toggle_stage(store, cx));
                        }))
                        .child(sculpted_secondary_button_body(
                            Some(DeckIcon::ChalkboardSimple),
                            if stage_open {
                                "Close Stage"
                            } else {
                                "Open Stage"
                            },
                            if stage_open {
                                TOKENS.signal_preview
                            } else {
                                TOKENS.ink_secondary
                            },
                        )),
                ),
        )
        .child(
            div()
                .id("pop-toggle-direct-click")
                .w_full()
                .px(px(10.0))
                .py(px(8.0))
                .rounded(px(RADIUS_CONTROL))
                .cursor_pointer()
                .bg(if direct_click {
                    TOKENS.elevated
                } else {
                    rgba(0x0000_0000)
                })
                .hover(|s| s.bg(TOKENS.elevated_hover))
                .border_1()
                .border_color(if direct_click {
                    TOKENS.focus_border
                } else {
                    TOKENS.gridline
                })
                .flex()
                .items_center()
                .justify_between()
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.store.update(cx, |s, cx| {
                        let _ =
                            s.dispatch(PresentationCommand::SetDirectClickLive(!direct_click), cx);
                    });
                }))
                .child(
                    div()
                        .font_family(FONT_BODY)
                        .text_color(TOKENS.ink_secondary)
                        .text_size(px(11.5))
                        .child("Click slide to show immediately"),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(5.0))
                        .child(div().w(px(6.0)).h(px(6.0)).rounded(px(RADIUS_PILL)).bg(
                            if direct_click {
                                TOKENS.signal_preview
                            } else {
                                rgb(0x565C6B)
                            },
                        ))
                        .child(
                            div()
                                .font_family(FONT_BODY)
                                .text_color(if direct_click {
                                    TOKENS.ink_primary
                                } else {
                                    TOKENS.ink_muted
                                })
                                .text_size(px(11.0))
                                .font_weight(FontWeight::MEDIUM)
                                .child(if direct_click { "On" } else { "Off" }),
                        ),
                ),
        )
}

fn build_display_target_chips(
    prefix: &'static str,
    displays: &[ConnectedDisplayInfo],
    current: DisplayTarget,
    dot_color: gpui::Rgba,
    is_audience: bool,
    cx: &mut Context<OperatorWorkspace>,
) -> Div {
    let mut targets: Vec<(String, String, DisplayTarget)> = vec![(
        format!("{prefix}-auto"),
        "Auto".to_owned(),
        DisplayTarget::Auto,
    )];
    for d in displays {
        targets.push((
            format!("{prefix}-disp-{}", d.index),
            format!("Display {} ({})", d.index + 1, d.resolution),
            DisplayTarget::Display(d.index),
        ));
    }
    targets.push((
        format!("{prefix}-win"),
        "Windowed".to_owned(),
        DisplayTarget::Windowed,
    ));

    let mut row = div().flex().flex_wrap().gap(px(5.0));
    for (id, label, target) in targets {
        row = row.child(
            neutral_option_chip(id, label, current == target, dot_color).on_click(cx.listener(
                move |this, _, _, cx| {
                    let store = this.store.clone();
                    this.windows.update(cx, |w, cx| {
                        if is_audience {
                            w.set_audience_target(target, store, cx);
                        } else {
                            w.set_stage_target(target, store, cx);
                        }
                    });
                },
            )),
        );
    }
    row
}
