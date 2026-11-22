use gpui::{
    Context, Div, FontWeight, InteractiveElement, ParentElement, StatefulInteractiveElement,
    Styled, div, px, rgb, rgba,
};
use sundaydeck_core::{PresentationCommand, ReflowMode, SlideContent};

use crate::palette::{
    BG_CARD, BG_CARD_HOVER, BG_ELEVATED, BG_SURFACE, BORDER_SUBTLE, GO_LIVE_BG, GO_LIVE_HOVER,
    PREVIEW_ACCENT, PREVIEW_SURFACE, TEXT_MUTED, TEXT_PRIMARY, TEXT_SECONDARY, color,
    section_kind_color,
};
use crate::views::operator_workspace::OperatorWorkspace;

/// Top half of the Center Pane: Preview Slide Grid, Reflow mode toggles, and Go Live trigger.
pub fn render_preview_section(
    workspace: &OperatorWorkspace,
    cx: &mut Context<OperatorWorkspace>,
) -> Div {
    let store = workspace.store.read(cx);
    let snapshot = store.snapshot();
    let preview = snapshot.preview.clone();
    let active_reflow = store.preview_reflow_mode;

    let (title, slide_count, active_idx, slides) = match &preview {
        Some(cursor) => (
            cursor.title.clone(),
            cursor.len(),
            cursor.active_index,
            cursor.slides.to_vec(),
        ),
        None => ("Nothing Staged".to_owned(), 0, 0, Vec::new()),
    };

    let mut reflow_pills = div().flex().items_center().gap(px(4.0));
    for mode in ReflowMode::ALL {
        let is_selected = mode == active_reflow;
        reflow_pills = reflow_pills.child(
            div()
                .id(format!("reflow-{}", mode.as_db_str()))
                .px(px(8.0))
                .py(px(4.0))
                .rounded(px(5.0))
                .cursor_pointer()
                .bg(if is_selected {
                    rgba(0xF59E_0B30)
                } else {
                    color(BG_ELEVATED)
                })
                .border_1()
                .border_color(if is_selected {
                    color(PREVIEW_ACCENT)
                } else {
                    color(BORDER_SUBTLE)
                })
                .text_color(if is_selected {
                    rgb(0xFDE68A)
                } else {
                    color(TEXT_SECONDARY)
                })
                .text_size(px(11.0))
                .font_weight(FontWeight::MEDIUM)
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.store
                        .update(cx, |s, cx| s.set_preview_reflow_mode(mode, cx));
                }))
                .child(mode.display_label()),
        );
    }

    let mut grid = div()
        .id("preview-slide-grid")
        .flex_1()
        .overflow_y_scroll()
        .p(px(14.0))
        .flex()
        .flex_wrap()
        .content_start()
        .gap(px(10.0));

    for (idx, slide) in slides.iter().enumerate() {
        let is_selected = idx == active_idx;
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
            .take(4)
            .collect::<Vec<_>>()
            .join("\n");

        grid = grid.child(
            div()
                .id(format!("preview-slide-{idx}"))
                .w(px(236.0))
                .h(px(124.0))
                .rounded(px(9.0))
                .overflow_hidden()
                .cursor_pointer()
                .bg(if is_selected {
                    color(PREVIEW_SURFACE)
                } else {
                    color(BG_CARD)
                })
                .hover(|s| s.bg(color(BG_CARD_HOVER)))
                .border_2()
                .border_color(if is_selected {
                    color(PREVIEW_ACCENT)
                } else {
                    color(BORDER_SUBTLE)
                })
                .flex()
                .flex_col()
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.store.update(cx, |s, cx| {
                        let _ = s.dispatch(PresentationCommand::SelectPreviewSlide(idx), cx);
                        if s.snapshot().direct_click_live {
                            let _ = s.dispatch(PresentationCommand::GoLive, cx);
                        }
                    });
                }))
                .child(
                    div()
                        .w_full()
                        .px(px(9.0))
                        .py(px(4.0))
                        .bg(tag_color.alpha(0.24))
                        .border_b_1()
                        .border_color(tag_color.alpha(0.45))
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(
                            div()
                                .text_color(color(TEXT_PRIMARY))
                                .text_size(px(11.0))
                                .font_weight(FontWeight::BOLD)
                                .child(slide.label.clone()),
                        )
                        .child(
                            div()
                                .text_color(color(TEXT_SECONDARY))
                                .text_size(px(10.5))
                                .child(format!("#{}", idx + 1)),
                        ),
                )
                .child(
                    div()
                        .flex_1()
                        .p(px(9.0))
                        .overflow_hidden()
                        .text_color(color(TEXT_PRIMARY))
                        .text_size(px(12.0))
                        .line_height(px(17.0))
                        .child(excerpt),
                ),
        );
    }

    div()
        .flex_1()
        .w_full()
        .border_b_1()
        .border_color(color(BORDER_SUBTLE))
        .flex()
        .flex_col()
        .overflow_hidden()
        .child(
            div()
                .px(px(14.0))
                .py(px(10.0))
                .bg(color(BG_SURFACE))
                .border_b_1()
                .border_color(color(BORDER_SUBTLE))
                .flex()
                .items_center()
                .justify_between()
                .gap(px(10.0))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(9.0))
                        .child(
                            div()
                                .px(px(8.0))
                                .py(px(3.0))
                                .rounded(px(5.0))
                                .bg(rgba(0xF59E_0B28))
                                .border_1()
                                .border_color(color(PREVIEW_ACCENT))
                                .text_color(rgb(0xFCD34D))
                                .text_size(px(10.5))
                                .font_weight(FontWeight::BOLD)
                                .child("PREVIEW"),
                        )
                        .child(
                            div()
                                .text_color(color(TEXT_PRIMARY))
                                .text_size(px(14.0))
                                .font_weight(FontWeight::BOLD)
                                .child(title),
                        )
                        .child(
                            div()
                                .text_color(color(TEXT_MUTED))
                                .text_size(px(11.5))
                                .child(format!("{slide_count} slides")),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(10.0))
                        .child(reflow_pills)
                        .child(
                            div()
                                .id("btn-go-live")
                                .px(px(14.0))
                                .py(px(6.0))
                                .rounded(px(7.0))
                                .cursor_pointer()
                                .bg(color(GO_LIVE_BG))
                                .hover(|s| s.bg(color(GO_LIVE_HOVER)))
                                .text_color(rgb(0x051811))
                                .text_size(px(12.5))
                                .font_weight(FontWeight::BOLD)
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.store.update(cx, |s, cx| {
                                        let _ = s.dispatch(PresentationCommand::GoLive, cx);
                                    });
                                }))
                                .child("GO LIVE  [F5]"),
                        ),
                ),
        )
        .child(grid)
}
