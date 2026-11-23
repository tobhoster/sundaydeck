use gpui::{
    Context, Div, FontWeight, InteractiveElement, ParentElement, StatefulInteractiveElement,
    Styled, div, px, rgb, rgba,
};
use sundaydeck_core::{OutputOverride, PresentationCommand, SlideContent};

use crate::design::icons::{DeckIcon, IconSize, deck_icon};
use crate::design::motion::tally_pulse_dot;
use crate::design::primitives::section_tag;
use crate::design::tokens::{
    FONT_BODY, FONT_DISPLAY, FONT_MONO, LIVE_DECK_WIDTH, MONITOR_HEIGHT_PX, MONITOR_WIDTH_PX,
    PANE_HEADER_HEIGHT, RADIUS_CONTROL, TOKENS, section_short_code, section_strip_color,
};
use crate::renderer::render_audience_canvas;
use crate::views::operator_workspace::OperatorWorkspace;
use crate::views::override_controls::{render_override_buttons, render_stage_cue_strip};

/// Right Pane (`340px` Live Control Deck):
/// Top 16:9 Live Output mirror framed with `1px` Tally Red (`#E53935`, or Warning Orange `#F97316`
/// when an override is active), Tactile Broadcast Switcher Strip (`LIVE`, `CLEAR`, `BLACK`, `LOGO`),
/// active slide queue with `3px` tally bars, and private Stage Cue dock.
pub fn render_live_pane(workspace: &OperatorWorkspace, cx: &mut Context<OperatorWorkspace>) -> Div {
    let store_ref = workspace.store.read(cx);
    let snapshot = store_ref.snapshot();
    let now_ms = store_ref.now_epoch_ms();
    let motion_mode = store_ref.motion_mode;
    let live = snapshot.live.clone();
    let active_override = snapshot.output_override;
    let stage_note = snapshot.stage.stage_message.clone();
    let has_live = live.is_some();

    let (live_title, pos_badge, active_idx, live_slides) = match &live {
        Some(cursor) => (
            cursor.title.clone(),
            format!("{:02}/{:02}", cursor.active_index + 1, cursor.len()),
            cursor.active_index,
            cursor.slides.to_vec(),
        ),
        None => ("STANDBY".to_owned(), "--/--".to_owned(), 0, Vec::new()),
    };

    let monitor_border = if active_override != OutputOverride::None {
        TOKENS.signal_override
    } else if has_live {
        TOKENS.signal_live
    } else {
        TOKENS.gridline
    };

    let override_badge = match active_override {
        OutputOverride::None => None,
        OutputOverride::ClearText => Some("CLEAR TEXT [F1]"),
        OutputOverride::Black => Some("BLACKOUT [F2]"),
        OutputOverride::Logo => Some("LOGO SPLASH [F3]"),
    };

    let mut monitor = div()
        .relative()
        .w(px(MONITOR_WIDTH_PX))
        .h(px(MONITOR_HEIGHT_PX))
        .rounded(px(RADIUS_CONTROL))
        .overflow_hidden()
        .border_1()
        .border_color(monitor_border)
        .bg(rgb(0x000000))
        .child(render_audience_canvas(
            &snapshot.audience,
            MONITOR_WIDTH_PX,
            now_ms,
            cx,
        ));

    if let Some(text) = override_badge {
        monitor = monitor.child(
            div()
                .absolute()
                .top(px(6.0))
                .left(px(6.0))
                .px(px(6.0))
                .py(px(2.0))
                .rounded(px(RADIUS_CONTROL))
                .bg(TOKENS.signal_override)
                .font_family(FONT_MONO)
                .text_color(TOKENS.canvas)
                .text_size(px(9.5))
                .font_weight(FontWeight::BOLD)
                .child(text),
        );
    }

    let mut live_list = div()
        .id("live-slides-list")
        .flex_1()
        .overflow_y_scroll()
        .flex()
        .flex_col();

    for (idx, slide) in live_slides.iter().enumerate() {
        let is_current = idx == active_idx;
        let sec_kind = match &slide.content {
            SlideContent::Text { section_kind, .. } => *section_kind,
            _ => None,
        };
        let strip_color = sec_kind
            .map(section_strip_color)
            .unwrap_or(TOKENS.ink_muted);
        let short_code = section_short_code(&slide.label, sec_kind);
        let excerpt: String = slide
            .content
            .primary_text()
            .lines()
            .take(2)
            .collect::<Vec<_>>()
            .join(" / ");

        live_list = live_list.child(
            div()
                .id(format!("live-slide-{idx}"))
                .w_full()
                .cursor_pointer()
                .bg(if is_current {
                    TOKENS.signal_live_tint
                } else {
                    TOKENS.pane
                })
                .hover(|s| s.bg(TOKENS.elevated_hover))
                .border_b_1()
                .border_color(TOKENS.gridline)
                .flex()
                .items_stretch()
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.store.update(cx, |s, cx| {
                        let _ = s.dispatch(PresentationCommand::SelectLiveSlide(idx), cx);
                    });
                }))
                .child(div().w(px(3.0)).flex_shrink_0().bg(if is_current {
                    TOKENS.signal_live
                } else {
                    rgba(0x0000_0000)
                }))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .overflow_hidden()
                        .px(px(10.0))
                        .py(px(7.0))
                        .flex()
                        .flex_col()
                        .gap(px(4.0))
                        .child(
                            div()
                                .w_full()
                                .flex()
                                .items_center()
                                .justify_between()
                                .gap(px(6.0))
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(px(6.0))
                                        .child(section_tag(&short_code, strip_color))
                                        .child(
                                            div()
                                                .font_family(FONT_MONO)
                                                .text_color(TOKENS.ink_primary)
                                                .text_size(px(10.5))
                                                .font_weight(FontWeight::BOLD)
                                                .child(slide.label.clone()),
                                        ),
                                )
                                .child(
                                    div()
                                        .flex_shrink_0()
                                        .font_family(FONT_MONO)
                                        .text_color(if is_current {
                                            TOKENS.signal_live
                                        } else {
                                            TOKENS.ink_muted
                                        })
                                        .text_size(px(9.5))
                                        .font_weight(FontWeight::BOLD)
                                        .child(if is_current {
                                            "ON AIR".to_owned()
                                        } else {
                                            format!("{:02}", idx + 1)
                                        }),
                                ),
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
                                .text_size(px(11.0))
                                .line_height(px(15.0))
                                .child(excerpt),
                        ),
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
                .px(px(12.0))
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
                        .child(tally_pulse_dot(
                            "live-pane-tally",
                            TOKENS.signal_live,
                            has_live && active_override == OutputOverride::None,
                            motion_mode,
                        ))
                        .child(
                            div()
                                .font_family(FONT_MONO)
                                .text_color(TOKENS.signal_live)
                                .text_size(px(10.0))
                                .font_weight(FontWeight::BOLD)
                                .child("LIVE"),
                        )
                        .child(
                            div()
                                .font_family(FONT_DISPLAY)
                                .text_color(TOKENS.ink_primary)
                                .text_size(px(12.0))
                                .font_weight(FontWeight::BOLD)
                                .child(live_title),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(4.0))
                        .child(
                            div()
                                .id("btn-quick-edit-live")
                                .h(px(22.0))
                                .px(px(6.0))
                                .rounded(px(RADIUS_CONTROL))
                                .cursor_pointer()
                                .bg(TOKENS.elevated)
                                .hover(|s| s.bg(TOKENS.elevated_hover))
                                .border_1()
                                .border_color(TOKENS.gridline)
                                .flex()
                                .items_center()
                                .gap(px(4.0))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.open_quick_edit(cx);
                                }))
                                .child(deck_icon(
                                    DeckIcon::PencilSimple,
                                    IconSize::Sm,
                                    TOKENS.ink_secondary,
                                ))
                                .child(
                                    div()
                                        .font_family(FONT_MONO)
                                        .text_color(TOKENS.ink_secondary)
                                        .text_size(px(9.5))
                                        .child("EDIT"),
                                ),
                        )
                        .child(
                            div()
                                .font_family(FONT_MONO)
                                .text_color(TOKENS.ink_muted)
                                .text_size(px(10.0))
                                .child(pos_badge),
                        )
                        .child(
                            div()
                                .id("btn-prev-slide")
                                .w(px(22.0))
                                .h(px(22.0))
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
                                .w(px(22.0))
                                .h(px(22.0))
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
                .p(px(12.0))
                .border_b_1()
                .border_color(TOKENS.gridline)
                .flex()
                .flex_col()
                .gap(px(10.0))
                .child(monitor)
                .child(render_override_buttons(active_override, has_live, cx)),
        )
        .child(live_list)
        .child(render_stage_cue_strip(stage_note.as_deref(), cx))
}
