use gpui::{
    Context, Div, FontWeight, InteractiveElement, ParentElement, StatefulInteractiveElement,
    Styled, div, px, rgb,
};
use sundaydeck_core::{OutputOverride, PresentationCommand, SlideContent};

use crate::palette::{
    BG_CARD, BG_CARD_HOVER, BG_ELEVATED, BG_SURFACE, BORDER_STRONG, BORDER_SUBTLE, LIVE_ACCENT,
    LIVE_SURFACE, OVERRIDE_BLACK, OVERRIDE_CLEAR, OVERRIDE_LOGO, TEXT_MUTED, TEXT_PRIMARY,
    TEXT_SECONDARY, color, section_kind_color,
};
use crate::renderer::render_audience_canvas;
use crate::views::operator_workspace::OperatorWorkspace;
use crate::views::override_controls::{render_override_buttons, render_stage_cue_strip};

const MONITOR_WIDTH_PX: f32 = 348.0;
const MONITOR_HEIGHT_PX: f32 = 196.0;

/// Right Pane: WYSIWYG Live Output Monitor, pinned F1-F3 Recovery Overrides,
/// active live slide navigation grid, Quick Edit trigger, and private Stage cues.
pub fn render_live_pane(workspace: &OperatorWorkspace, cx: &mut Context<OperatorWorkspace>) -> Div {
    let store_ref = workspace.store.read(cx);
    let snapshot = store_ref.snapshot();
    let now_ms = store_ref.now_epoch_ms();
    let live = snapshot.live.clone();
    let active_override = snapshot.output_override;
    let stage_note = snapshot.stage.stage_message.clone();

    let (live_title, pos_badge, active_idx, live_slides) = match &live {
        Some(cursor) => (
            cursor.title.clone(),
            format!("{} / {}", cursor.active_index + 1, cursor.len()),
            cursor.active_index,
            cursor.slides.to_vec(),
        ),
        None => ("Standby".to_owned(), "—".to_owned(), 0, Vec::new()),
    };

    let monitor_border = match active_override {
        OutputOverride::None => color(LIVE_ACCENT),
        OutputOverride::ClearText => color(OVERRIDE_CLEAR),
        OutputOverride::Black => color(OVERRIDE_BLACK),
        OutputOverride::Logo => color(OVERRIDE_LOGO),
    };

    let override_pill = match active_override {
        OutputOverride::None => None,
        OutputOverride::ClearText => Some(("TEXT CLEARED", color(OVERRIDE_CLEAR))),
        OutputOverride::Black => Some(("BLACK OVERRIDE", color(OVERRIDE_BLACK))),
        OutputOverride::Logo => Some(("LOGO SPLASH", color(OVERRIDE_LOGO))),
    };

    let mut monitor = div()
        .relative()
        .w(px(MONITOR_WIDTH_PX))
        .h(px(MONITOR_HEIGHT_PX))
        .rounded(px(10.0))
        .overflow_hidden()
        .border_2()
        .border_color(monitor_border)
        .bg(rgb(0x000000))
        .child(render_audience_canvas(
            &snapshot.audience,
            MONITOR_WIDTH_PX,
            now_ms,
            cx,
        ));

    if let Some((text, pill_bg)) = override_pill {
        monitor = monitor.child(
            div()
                .absolute()
                .top(px(8.0))
                .left(px(8.0))
                .px(px(8.0))
                .py(px(3.0))
                .rounded(px(5.0))
                .bg(pill_bg)
                .text_color(rgb(0x0B0F17))
                .text_size(px(10.0))
                .font_weight(FontWeight::BOLD)
                .child(text),
        );
    }

    let mut live_list = div()
        .id("live-slides-list")
        .flex_1()
        .overflow_y_scroll()
        .px(px(14.0))
        .py(px(10.0))
        .flex()
        .flex_col()
        .gap(px(6.0));

    for (idx, slide) in live_slides.iter().enumerate() {
        let is_current = idx == active_idx;
        let tag_color = match &slide.content {
            SlideContent::Text {
                section_kind: Some(kind),
                ..
            } => section_kind_color(*kind),
            _ => rgb(0x6366F1),
        };
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
                .px(px(10.0))
                .py(px(8.0))
                .rounded(px(8.0))
                .cursor_pointer()
                .bg(if is_current {
                    color(LIVE_SURFACE)
                } else {
                    color(BG_CARD)
                })
                .hover(|s| s.bg(color(BG_CARD_HOVER)))
                .border_2()
                .border_color(if is_current {
                    color(LIVE_ACCENT)
                } else {
                    color(BORDER_SUBTLE)
                })
                .flex()
                .flex_col()
                .gap(px(4.0))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.store.update(cx, |s, cx| {
                        let _ = s.dispatch(PresentationCommand::SelectLiveSlide(idx), cx);
                    });
                }))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(6.0))
                                .child(div().w(px(7.0)).h(px(7.0)).rounded(px(99.0)).bg(tag_color))
                                .child(
                                    div()
                                        .text_color(color(TEXT_PRIMARY))
                                        .text_size(px(11.5))
                                        .font_weight(FontWeight::BOLD)
                                        .child(slide.label.clone()),
                                ),
                        )
                        .child(
                            div()
                                .text_color(if is_current {
                                    rgb(0xFCA5A5)
                                } else {
                                    color(TEXT_MUTED)
                                })
                                .text_size(px(10.5))
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(if is_current {
                                    "ON AIR".to_owned()
                                } else {
                                    format!("#{}", idx + 1)
                                }),
                        ),
                )
                .child(
                    div()
                        .text_color(color(TEXT_SECONDARY))
                        .text_size(px(11.5))
                        .child(excerpt),
                ),
        );
    }

    div()
        .w(px(380.0))
        .h_full()
        .bg(color(BG_SURFACE))
        .border_l_1()
        .border_color(color(BORDER_SUBTLE))
        .flex()
        .flex_col()
        .child(
            div()
                .px(px(14.0))
                .py(px(10.0))
                .border_b_1()
                .border_color(color(BORDER_SUBTLE))
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(7.0))
                        .child(
                            div()
                                .px(px(7.0))
                                .py(px(3.0))
                                .rounded(px(5.0))
                                .bg(color(LIVE_ACCENT))
                                .text_color(rgb(0xFFFFFF))
                                .text_size(px(10.0))
                                .font_weight(FontWeight::BOLD)
                                .child("● LIVE"),
                        )
                        .child(
                            div()
                                .text_color(color(TEXT_PRIMARY))
                                .text_size(px(12.5))
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
                                .px(px(7.0))
                                .py(px(4.0))
                                .rounded(px(5.0))
                                .cursor_pointer()
                                .bg(color(BG_ELEVATED))
                                .hover(|s| s.bg(color(BG_CARD_HOVER)))
                                .border_1()
                                .border_color(color(BORDER_STRONG))
                                .text_color(color(TEXT_SECONDARY))
                                .text_size(px(10.5))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.open_quick_edit(cx);
                                }))
                                .child("Edit Live"),
                        )
                        .child(
                            div()
                                .text_color(color(TEXT_MUTED))
                                .text_size(px(10.5))
                                .child(pos_badge),
                        )
                        .child(
                            div()
                                .id("btn-prev-slide")
                                .px(px(7.0))
                                .py(px(4.0))
                                .rounded(px(5.0))
                                .cursor_pointer()
                                .bg(color(BG_ELEVATED))
                                .hover(|s| s.bg(color(BG_CARD_HOVER)))
                                .border_1()
                                .border_color(color(BORDER_STRONG))
                                .text_color(color(TEXT_PRIMARY))
                                .text_size(px(11.0))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.store.update(cx, |s, cx| {
                                        let _ = s.dispatch(PresentationCommand::PrevLiveSlide, cx);
                                    });
                                }))
                                .child("◀"),
                        )
                        .child(
                            div()
                                .id("btn-next-slide")
                                .px(px(7.0))
                                .py(px(4.0))
                                .rounded(px(5.0))
                                .cursor_pointer()
                                .bg(color(BG_ELEVATED))
                                .hover(|s| s.bg(color(BG_CARD_HOVER)))
                                .border_1()
                                .border_color(color(BORDER_STRONG))
                                .text_color(color(TEXT_PRIMARY))
                                .text_size(px(11.0))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.store.update(cx, |s, cx| {
                                        let _ = s.dispatch(PresentationCommand::NextLiveSlide, cx);
                                    });
                                }))
                                .child("▶"),
                        ),
                ),
        )
        .child(
            div()
                .p(px(14.0))
                .border_b_1()
                .border_color(color(BORDER_SUBTLE))
                .flex()
                .flex_col()
                .gap(px(10.0))
                .child(monitor)
                .child(render_override_buttons(active_override, cx)),
        )
        .child(live_list)
        .child(render_stage_cue_strip(stage_note.as_deref(), cx))
}
