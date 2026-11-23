use gpui::{
    Context, Div, FontWeight, InteractiveElement, ParentElement, StatefulInteractiveElement,
    Styled, div, px, rgb,
};
use sundaydeck_core::{OutputOverride, PresentationCommand, SlideContent};

use crate::design::icons::{DeckIcon, IconSize, deck_icon};
use crate::design::motion::tally_pulse_dot;
use crate::design::primitives::{section_pill, soft_pill};
use crate::design::tokens::{
    FONT_BODY, FONT_DISPLAY, LIVE_DECK_WIDTH, MONITOR_HEIGHT_PX, MONITOR_WIDTH_PX,
    PANE_HEADER_HEIGHT, RADIUS_CONTROL, RADIUS_PILL, RADIUS_TILE, TOKENS,
};
use crate::renderer::render_audience_canvas;
use crate::views::operator_workspace::OperatorWorkspace;
use crate::views::override_controls::{render_override_buttons, render_stage_cue_strip};

/// Right Pane (On Screen Now):
/// Realistic 16:9 Live Preview framed with a clean 2px warm coral-crimson ring (or warm terracotta
/// ring + unmistakable status pill when screen is paused), plain-English recovery buttons,
/// calm active slide list, and private Note for Stage strip.
pub fn render_live_pane(workspace: &OperatorWorkspace, cx: &mut Context<OperatorWorkspace>) -> Div {
    let store_ref = workspace.store.read(cx);
    let snapshot = store_ref.snapshot();
    let now_ms = store_ref.now_epoch_ms();
    let motion_mode = store_ref.motion_mode;
    let live = snapshot.live.clone();
    let active_override = snapshot.output_override;
    let stage_note = snapshot.stage.stage_message.clone();
    let has_live = live.is_some();

    let (live_title, pos_label, active_idx, live_slides) = match &live {
        Some(cursor) => (
            cursor.title.clone(),
            format!("{} of {}", cursor.active_index + 1, cursor.len()),
            cursor.active_index,
            cursor.slides.to_vec(),
        ),
        None => ("Nothing on screen".to_owned(), String::new(), 0, Vec::new()),
    };

    let monitor_border = if active_override != OutputOverride::None {
        TOKENS.signal_override
    } else if has_live {
        TOKENS.signal_live
    } else {
        TOKENS.gridline
    };

    let override_pill_copy = match active_override {
        OutputOverride::None => None,
        OutputOverride::ClearText => Some("Words Hidden"),
        OutputOverride::Black => Some("Screen Paused · Black Screen"),
        OutputOverride::Logo => Some("Showing Church Logo"),
    };

    let mut monitor = div()
        .relative()
        .w(px(MONITOR_WIDTH_PX))
        .h(px(MONITOR_HEIGHT_PX))
        .rounded(px(RADIUS_TILE))
        .overflow_hidden()
        .border_2()
        .border_color(monitor_border)
        .bg(rgb(0x0E0F12))
        .child(render_audience_canvas(
            &snapshot.audience,
            MONITOR_WIDTH_PX,
            now_ms,
            cx,
        ));

    if let Some(text) = override_pill_copy {
        monitor = monitor.child(
            div()
                .absolute()
                .top(px(8.0))
                .left(px(8.0))
                .px(px(10.0))
                .py(px(4.0))
                .rounded(px(RADIUS_PILL))
                .bg(TOKENS.signal_override)
                .font_family(FONT_BODY)
                .text_color(TOKENS.ink_primary)
                .text_size(px(11.0))
                .font_weight(FontWeight::SEMIBOLD)
                .child(text),
        );
    }

    let mut live_list = div()
        .id("live-slides-list")
        .flex_1()
        .overflow_y_scroll()
        .p(px(10.0))
        .flex()
        .flex_col()
        .gap(px(7.0));

    for (idx, slide) in live_slides.iter().enumerate() {
        let is_current = idx == active_idx;
        let sec_kind = match &slide.content {
            SlideContent::Text { section_kind, .. } => *section_kind,
            _ => None,
        };
        let excerpt: String = slide
            .content
            .primary_text()
            .lines()
            .take(2)
            .collect::<Vec<_>>()
            .join("\n");

        let mut card = div()
            .id(format!("live-slide-{idx}"))
            .w_full()
            .px(px(12.0))
            .py(px(10.0))
            .rounded(px(RADIUS_TILE))
            .cursor_pointer()
            .bg(if is_current {
                TOKENS.signal_live_tint
            } else {
                TOKENS.elevated
            })
            .hover(|s| s.bg(TOKENS.elevated_hover))
            .border_color(if is_current {
                TOKENS.signal_live
            } else {
                TOKENS.gridline
            })
            .flex()
            .flex_col()
            .gap(px(6.0))
            .on_click(cx.listener(move |this, _, _, cx| {
                this.store.update(cx, |s, cx| {
                    let _ = s.dispatch(PresentationCommand::SelectLiveSlide(idx), cx);
                });
            }));

        card = if is_current {
            card.border_2()
        } else {
            card.border_1()
        };

        live_list = live_list.child(
            card.child(
                div()
                    .w_full()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(6.0))
                    .child(section_pill(&slide.label, sec_kind))
                    .child(if is_current {
                        soft_pill("Live", TOKENS.signal_live, TOKENS.ink_primary)
                    } else {
                        div()
                            .font_family(FONT_BODY)
                            .text_color(TOKENS.ink_muted)
                            .text_size(px(11.0))
                            .child(format!("{}", idx + 1))
                    }),
            )
            .child(
                div()
                    .w_full()
                    .min_w_0()
                    .font_family(FONT_BODY)
                    .text_color(if is_current {
                        TOKENS.ink_primary
                    } else {
                        TOKENS.ink_secondary
                    })
                    .text_size(px(12.0))
                    .line_height(px(17.0))
                    .child(excerpt),
            ),
        );
    }

    div()
        .w(px(LIVE_DECK_WIDTH))
        .h_full()
        .bg(TOKENS.pane)
        .border_l_1()
        .border_color(TOKENS.gridline)
        .flex()
        .flex_col()
        .child(
            div()
                .w_full()
                .h(px(PANE_HEADER_HEIGHT))
                .px(px(14.0))
                .border_b_1()
                .border_color(TOKENS.gridline)
                .flex()
                .items_center()
                .justify_between()
                .gap(px(8.0))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .overflow_hidden()
                        .flex()
                        .items_center()
                        .gap(px(7.0))
                        .child(div().flex_shrink_0().child(tally_pulse_dot(
                            "live-pane-tally",
                            TOKENS.signal_live,
                            has_live && active_override == OutputOverride::None,
                            motion_mode,
                        )))
                        .child(div().flex_shrink_0().child(soft_pill(
                            "Live",
                            TOKENS.signal_live_tint,
                            TOKENS.signal_live,
                        )))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .overflow_hidden()
                                .whitespace_nowrap()
                                .font_family(FONT_DISPLAY)
                                .text_color(TOKENS.ink_primary)
                                .text_size(px(13.0))
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(live_title),
                        ),
                )
                .child(
                    div()
                        .flex_shrink_0()
                        .flex()
                        .items_center()
                        .gap(px(5.0))
                        .child(
                            div()
                                .id("btn-quick-edit-live")
                                .w(px(26.0))
                                .h(px(26.0))
                                .rounded(px(RADIUS_CONTROL))
                                .cursor_pointer()
                                .bg(TOKENS.elevated)
                                .hover(|s| s.bg(TOKENS.elevated_hover))
                                .border_1()
                                .border_color(TOKENS.gridline)
                                .flex()
                                .items_center()
                                .justify_center()
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.open_quick_edit(cx);
                                }))
                                .child(deck_icon(
                                    DeckIcon::PencilSimple,
                                    IconSize::Sm,
                                    TOKENS.ink_secondary,
                                )),
                        )
                        .child(
                            div()
                                .font_family(FONT_BODY)
                                .text_color(TOKENS.ink_muted)
                                .text_size(px(11.0))
                                .child(pos_label),
                        )
                        .child(
                            div()
                                .id("btn-prev-slide")
                                .w(px(26.0))
                                .h(px(26.0))
                                .rounded(px(RADIUS_CONTROL))
                                .cursor_pointer()
                                .bg(TOKENS.elevated)
                                .hover(|s| s.bg(TOKENS.elevated_hover))
                                .border_1()
                                .border_color(TOKENS.gridline)
                                .flex()
                                .items_center()
                                .justify_center()
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.store.update(cx, |s, cx| {
                                        let _ = s.dispatch(PresentationCommand::PrevLiveSlide, cx);
                                    });
                                }))
                                .child(deck_icon(
                                    DeckIcon::CaretLeft,
                                    IconSize::Sm,
                                    TOKENS.ink_primary,
                                )),
                        )
                        .child(
                            div()
                                .id("btn-next-slide")
                                .w(px(26.0))
                                .h(px(26.0))
                                .rounded(px(RADIUS_CONTROL))
                                .cursor_pointer()
                                .bg(TOKENS.elevated)
                                .hover(|s| s.bg(TOKENS.elevated_hover))
                                .border_1()
                                .border_color(TOKENS.gridline)
                                .flex()
                                .items_center()
                                .justify_center()
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.store.update(cx, |s, cx| {
                                        let _ = s.dispatch(PresentationCommand::NextLiveSlide, cx);
                                    });
                                }))
                                .child(deck_icon(
                                    DeckIcon::CaretRight,
                                    IconSize::Sm,
                                    TOKENS.ink_primary,
                                )),
                        ),
                ),
        )
        .child(
            div()
                .p(px(14.0))
                .border_b_1()
                .border_color(TOKENS.gridline)
                .flex()
                .flex_col()
                .gap(px(12.0))
                .child(monitor)
                .child(render_override_buttons(active_override, has_live, cx)),
        )
        .child(live_list)
        .child(render_stage_cue_strip(stage_note.as_deref(), cx))
}
