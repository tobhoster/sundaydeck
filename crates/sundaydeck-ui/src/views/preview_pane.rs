use std::sync::Arc;

use gpui::{
    Context, Div, FontWeight, InteractiveElement, ParentElement, StatefulInteractiveElement,
    Styled, div, px, rgba,
};
use sundaydeck_core::{BackgroundSpec, PresentationCommand, ReflowMode, SlideContent};

use crate::design::fonts::resolve_font_family;
use crate::design::icons::{DeckIcon, IconSize, deck_icon};
use crate::design::primitives::{section_pill, soft_pill};
use crate::design::tokens::{
    FONT_BODY, FONT_DISPLAY, PANE_HEADER_HEIGHT, RADIUS_CONTROL, RADIUS_PILL, RADIUS_TILE, TOKENS,
    rgba8_to_gpui,
};
use crate::views::operator_workspace::OperatorWorkspace;

const TILE_W_PX: f32 = 256.0;
const TILE_H_PX: f32 = 144.0; // exact 16:9

/// Top half of the Center Pane:
/// Realistic 16:9 miniature slide previews styled like the sanctuary projector,
/// with human-readable colored section pills (`Verse 1`, `Chorus`, `Romans 8:28`),
/// clean Sentence-case reflow & theme controls, and an inviting `Show on Screen` action.
pub fn render_preview_section(
    workspace: &OperatorWorkspace,
    cx: &mut Context<OperatorWorkspace>,
) -> Div {
    let store = workspace.store.read(cx);
    let snapshot = store.snapshot();
    let preview = snapshot.preview.clone();
    let active_reflow = store.preview_reflow_mode;
    let themes = store.themes.clone();
    let active_theme = store.active_theme();

    let (title, slide_count, active_idx, slides, preview_font, tile_canvas_bg) = match &preview {
        Some(cursor) => {
            let bg = match &cursor.theme.background {
                BackgroundSpec::Solid { color } => rgba8_to_gpui(*color),
                BackgroundSpec::LinearGradient { start, .. } => rgba8_to_gpui(*start),
                BackgroundSpec::Image { .. } => TOKENS.elevated,
            };
            (
                cursor.title.clone(),
                cursor.len(),
                cursor.active_index,
                cursor.slides.to_vec(),
                resolve_font_family(&cursor.theme.font_family, cx),
                bg,
            )
        }
        None => (
            "Select an item from Service Order".to_owned(),
            0,
            0,
            Vec::new(),
            resolve_font_family(&active_theme.font_family, cx),
            TOKENS.elevated,
        ),
    };

    let mut reflow_bar = div()
        .flex()
        .items_center()
        .gap(px(2.0))
        .p(px(2.0))
        .bg(TOKENS.canvas)
        .border_1()
        .border_color(TOKENS.gridline)
        .rounded(px(RADIUS_CONTROL))
        .child(div().px(px(5.0)).flex().items_center().child(deck_icon(
            DeckIcon::Rows,
            IconSize::Sm,
            TOKENS.ink_muted,
        )));

    for mode in ReflowMode::ALL {
        let selected = mode == active_reflow;
        let friendly = match mode {
            ReflowMode::ByStanza => "All",
            ReflowMode::Lines1 => "1",
            ReflowMode::Lines2 => "2",
            ReflowMode::Lines4 => "4",
        };
        reflow_bar = reflow_bar.child(
            div()
                .id(format!("reflow-{}", mode.as_db_str()))
                .px(px(8.0))
                .py(px(3.5))
                .rounded(px(4.0))
                .cursor_pointer()
                .bg(if selected {
                    TOKENS.elevated
                } else {
                    rgba(0x0000_0000)
                })
                .font_family(FONT_BODY)
                .text_color(if selected {
                    TOKENS.ink_primary
                } else {
                    TOKENS.ink_muted
                })
                .text_size(px(11.0))
                .font_weight(FontWeight::MEDIUM)
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.store
                        .update(cx, |s, cx| s.set_preview_reflow_mode(mode, cx));
                }))
                .child(friendly),
        );
    }

    let mut theme_strip = div().flex().items_center().gap(px(6.0));
    for theme in themes {
        let is_active = theme.id == active_theme.id;
        let theme_click = Arc::clone(&theme);
        theme_strip = theme_strip.child(
            div()
                .id(format!("preview-theme-{}", theme.id))
                .px(px(10.0))
                .py(px(4.0))
                .rounded(px(RADIUS_PILL))
                .cursor_pointer()
                .bg(if is_active {
                    TOKENS.elevated
                } else {
                    rgba(0x0000_0000)
                })
                .border_1()
                .border_color(if is_active {
                    TOKENS.focus_border
                } else {
                    rgba(0x0000_0000)
                })
                .font_family(FONT_BODY)
                .text_color(if is_active {
                    TOKENS.ink_primary
                } else {
                    TOKENS.ink_secondary
                })
                .text_size(px(11.0))
                .font_weight(FontWeight::MEDIUM)
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.store.update(cx, |s, cx| {
                        let _ = s.dispatch(
                            PresentationCommand::ApplyLiveTheme(Arc::clone(&theme_click)),
                            cx,
                        );
                    });
                }))
                .child(theme.name.clone()),
        );
    }

    let mut grid = div()
        .id("preview-slide-grid")
        .flex_1()
        .bg(TOKENS.canvas)
        .overflow_y_scroll()
        .p(px(16.0))
        .flex()
        .flex_wrap()
        .content_start()
        .gap(px(14.0));

    for (idx, slide) in slides.iter().enumerate() {
        let is_selected = idx == active_idx;
        let sec_kind = match &slide.content {
            SlideContent::Text { section_kind, .. } => *section_kind,
            _ => None,
        };
        let raw_text = slide.content.primary_text();
        let line_count = raw_text.lines().count();
        let body_size = if raw_text.len() > 140 || line_count > 4 {
            11.0
        } else {
            12.5
        };
        let body_lh = body_size * 1.4;

        let mut tile = div()
            .id(format!("preview-slide-{idx}"))
            .w(px(TILE_W_PX))
            .h(px(TILE_H_PX))
            .rounded(px(RADIUS_TILE))
            .overflow_hidden()
            .cursor_pointer()
            .bg(tile_canvas_bg)
            .hover(|s| s.border_color(TOKENS.focus_border))
            .border_color(if is_selected {
                TOKENS.signal_preview
            } else {
                TOKENS.gridline
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
            }));

        tile = if is_selected {
            tile.border_2()
        } else {
            tile.border_1()
        };

        grid = grid.child(
            tile.child(
                div()
                    .w_full()
                    .px(px(10.0))
                    .pt(px(8.0))
                    .pb(px(3.0))
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(6.0))
                    .child(section_pill(&slide.label, sec_kind))
                    .child(if is_selected {
                        soft_pill("Next Up", TOKENS.signal_preview_tint, TOKENS.signal_preview)
                    } else {
                        div()
                            .font_family(FONT_BODY)
                            .text_color(TOKENS.ink_muted)
                            .text_size(px(10.5))
                            .child(format!("{}", idx + 1))
                    }),
            )
            .child(
                div()
                    .flex_1()
                    .w_full()
                    .min_w_0()
                    .px(px(14.0))
                    .py(px(6.0))
                    .overflow_hidden()
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .child(
                        div()
                            .w_full()
                            .text_center()
                            .font_family(preview_font.clone())
                            .text_color(TOKENS.ink_primary)
                            .text_size(px(body_size))
                            .line_height(px(body_lh))
                            .child(raw_text.to_owned()),
                    ),
            ),
        );
    }

    let count_copy = if slide_count == 1 {
        "1 slide".to_owned()
    } else {
        format!("{slide_count} slides")
    };

    div()
        .flex_1()
        .w_full()
        .border_b_1()
        .border_color(TOKENS.gridline)
        .flex()
        .flex_col()
        .overflow_hidden()
        .child(
            div()
                .w_full()
                .h(px(PANE_HEADER_HEIGHT))
                .px(px(14.0))
                .bg(TOKENS.pane)
                .border_b_1()
                .border_color(TOKENS.gridline)
                .flex()
                .items_center()
                .justify_between()
                .gap(px(10.0))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .overflow_hidden()
                        .flex()
                        .items_center()
                        .gap(px(9.0))
                        .child(div().flex_shrink_0().child(soft_pill(
                            "Next Up",
                            TOKENS.signal_preview_tint,
                            TOKENS.signal_preview,
                        )))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .overflow_hidden()
                                .whitespace_nowrap()
                                .font_family(FONT_DISPLAY)
                                .text_color(TOKENS.ink_primary)
                                .text_size(px(13.5))
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(title),
                        )
                        .child(
                            div()
                                .flex_shrink_0()
                                .font_family(FONT_BODY)
                                .text_color(TOKENS.ink_muted)
                                .text_size(px(11.5))
                                .child(count_copy),
                        ),
                )
                .child(
                    div()
                        .flex_shrink_0()
                        .flex()
                        .items_center()
                        .gap(px(10.0))
                        .child(reflow_bar)
                        .child(
                            div()
                                .id("btn-go-live")
                                .h(px(30.0))
                                .px(px(13.0))
                                .rounded(px(RADIUS_CONTROL))
                                .cursor_pointer()
                                .bg(TOKENS.signal_live)
                                .flex()
                                .items_center()
                                .gap(px(6.0))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.store.update(cx, |s, cx| {
                                        let _ = s.dispatch(PresentationCommand::GoLive, cx);
                                    });
                                }))
                                .child(deck_icon(
                                    DeckIcon::PlayFill,
                                    IconSize::Sm,
                                    TOKENS.ink_primary,
                                ))
                                .child(
                                    div()
                                        .font_family(FONT_BODY)
                                        .text_color(TOKENS.ink_primary)
                                        .text_size(px(12.0))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .child("Show on Screen"),
                                ),
                        ),
                ),
        )
        .child(
            div()
                .w_full()
                .px(px(14.0))
                .py(px(6.0))
                .bg(TOKENS.pane)
                .border_b_1()
                .border_color(TOKENS.gridline)
                .flex()
                .items_center()
                .gap(px(8.0))
                .child(deck_icon(DeckIcon::Palette, IconSize::Sm, TOKENS.ink_muted))
                .child(theme_strip),
        )
        .child(grid)
}
