use std::sync::Arc;

use gpui::{
    Context, Div, FontWeight, InteractiveElement, ParentElement, StatefulInteractiveElement,
    Styled, div, px, rgba,
};
use sundaydeck_core::{PresentationCommand, ReflowMode, SlideContent};

use crate::design::fonts::resolve_font_family;
use crate::design::icons::{DeckIcon, IconSize, deck_icon};
use crate::design::primitives::{keycap, section_tag};
use crate::design::tokens::{
    FONT_DISPLAY, FONT_MONO, PANE_HEADER_HEIGHT, RADIUS_CONTROL, RADIUS_TILE, TOKENS,
    section_short_code, section_strip_color,
};
use crate::views::operator_workspace::OperatorWorkspace;

const TILE_W_PX: f32 = 248.0;
const TILE_H_PX: f32 = 140.0; // exact 16:9

/// Top half of the Center Pane:
/// Fluid 16:9 miniature slide grid rendered with real bundled projection typography,
/// monospace section strip tags (`V1`, `CH`, `BR`, `SCR`), Reflow toggle, and 1-click Theme bar.
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

    let (title, slide_count, active_idx, slides, preview_font) = match &preview {
        Some(cursor) => (
            cursor.title.clone(),
            cursor.len(),
            cursor.active_index,
            cursor.slides.to_vec(),
            resolve_font_family(&cursor.theme.font_family, cx),
        ),
        None => (
            "NO ITEM STAGED".to_owned(),
            0,
            0,
            Vec::new(),
            resolve_font_family(&active_theme.font_family, cx),
        ),
    };

    let mut reflow_bar = div()
        .flex()
        .items_center()
        .bg(TOKENS.canvas)
        .border_1()
        .border_color(TOKENS.gridline)
        .rounded(px(RADIUS_CONTROL));

    for mode in ReflowMode::ALL {
        let selected = mode == active_reflow;
        let short = match mode {
            ReflowMode::ByStanza => "STZ",
            ReflowMode::Lines1 => "1L",
            ReflowMode::Lines2 => "2L",
            ReflowMode::Lines4 => "4L",
        };
        reflow_bar = reflow_bar.child(
            div()
                .id(format!("reflow-{}", mode.as_db_str()))
                .px(px(7.0))
                .py(px(3.0))
                .cursor_pointer()
                .bg(if selected {
                    TOKENS.signal_preview_tint
                } else {
                    rgba(0x0000_0000)
                })
                .font_family(FONT_MONO)
                .text_color(if selected {
                    TOKENS.signal_preview
                } else {
                    TOKENS.ink_muted
                })
                .text_size(px(9.5))
                .font_weight(FontWeight::BOLD)
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.store
                        .update(cx, |s, cx| s.set_preview_reflow_mode(mode, cx));
                }))
                .child(short),
        );
    }

    let mut theme_strip = div().flex().items_center().gap(px(4.0));
    for theme in themes {
        let is_active = theme.id == active_theme.id;
        let theme_click = Arc::clone(&theme);
        theme_strip = theme_strip.child(
            div()
                .id(format!("preview-theme-{}", theme.id))
                .px(px(7.0))
                .py(px(2.5))
                .rounded(px(RADIUS_CONTROL))
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
                    TOKENS.gridline
                })
                .font_family(FONT_MONO)
                .text_color(if is_active {
                    TOKENS.ink_primary
                } else {
                    TOKENS.ink_muted
                })
                .text_size(px(9.5))
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
        .p(px(12.0))
        .flex()
        .flex_wrap()
        .content_start()
        .gap(px(10.0));

    for (idx, slide) in slides.iter().enumerate() {
        let is_selected = idx == active_idx;
        let sec_kind = match &slide.content {
            SlideContent::Text { section_kind, .. } => *section_kind,
            _ => None,
        };
        let strip_color = sec_kind
            .map(section_strip_color)
            .unwrap_or(TOKENS.ink_muted);
        let short_code = section_short_code(&slide.label, sec_kind);
        let raw_text = slide.content.primary_text();
        let line_count = raw_text.lines().count();
        let body_size = if raw_text.len() > 140 || line_count > 4 {
            10.5
        } else {
            12.0
        };
        let body_lh = body_size * 1.35;

        grid = grid.child(
            div()
                .id(format!("preview-slide-{idx}"))
                .w(px(TILE_W_PX))
                .h(px(TILE_H_PX))
                .rounded(px(RADIUS_TILE))
                .overflow_hidden()
                .cursor_pointer()
                .bg(if is_selected {
                    TOKENS.signal_preview_tint
                } else {
                    TOKENS.elevated
                })
                .hover(|s| s.bg(TOKENS.elevated_hover))
                .border_1()
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
                }))
                .child(
                    div()
                        .w_full()
                        .px(px(8.0))
                        .py(px(4.0))
                        .bg(TOKENS.pane)
                        .border_b_1()
                        .border_color(TOKENS.gridline)
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(px(6.0))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .overflow_hidden()
                                .flex()
                                .items_center()
                                .gap(px(6.0))
                                .child(
                                    div()
                                        .flex_shrink_0()
                                        .child(section_tag(&short_code, strip_color)),
                                )
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .overflow_hidden()
                                        .whitespace_nowrap()
                                        .font_family(FONT_MONO)
                                        .text_color(TOKENS.ink_secondary)
                                        .text_size(px(9.5))
                                        .child(slide.label.clone()),
                                ),
                        )
                        .child(
                            div()
                                .flex_shrink_0()
                                .font_family(FONT_MONO)
                                .text_color(if is_selected {
                                    TOKENS.signal_preview
                                } else {
                                    TOKENS.ink_muted
                                })
                                .text_size(px(9.5))
                                .font_weight(FontWeight::BOLD)
                                .child(format!("{:02}", idx + 1)),
                        ),
                )
                .child(
                    div()
                        .flex_1()
                        .w_full()
                        .min_w_0()
                        .px(px(10.0))
                        .py(px(7.0))
                        .overflow_hidden()
                        .flex()
                        .flex_col()
                        .justify_center()
                        .child(
                            div()
                                .w_full()
                                .font_family(preview_font.clone())
                                .text_color(TOKENS.ink_primary)
                                .text_size(px(body_size))
                                .line_height(px(body_lh))
                                .child(raw_text.to_owned()),
                        ),
                ),
        );
    }

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
                .px(px(12.0))
                .bg(TOKENS.pane)
                .border_b_1()
                .border_color(TOKENS.gridline)
                .flex()
                .items_center()
                .justify_between()
                .gap(px(10.0))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .child(
                            div()
                                .px(px(6.0))
                                .py(px(2.0))
                                .rounded(px(RADIUS_CONTROL))
                                .bg(TOKENS.signal_preview_tint)
                                .border_1()
                                .border_color(TOKENS.signal_preview)
                                .font_family(FONT_MONO)
                                .text_color(TOKENS.signal_preview)
                                .text_size(px(9.5))
                                .font_weight(FontWeight::BOLD)
                                .child("PREVIEW"),
                        )
                        .child(
                            div()
                                .font_family(FONT_DISPLAY)
                                .text_color(TOKENS.ink_primary)
                                .text_size(px(13.0))
                                .font_weight(FontWeight::BOLD)
                                .child(title),
                        )
                        .child(
                            div()
                                .font_family(FONT_MONO)
                                .text_color(TOKENS.ink_muted)
                                .text_size(px(10.0))
                                .child(format!("{slide_count:02} SLIDES")),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(4.0))
                                .child(deck_icon(DeckIcon::Rows, IconSize::Sm, TOKENS.ink_muted))
                                .child(reflow_bar),
                        )
                        .child(
                            div()
                                .id("btn-go-live")
                                .h(px(24.0))
                                .px(px(9.0))
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
                                .child(deck_icon(DeckIcon::PlayFill, IconSize::Sm, TOKENS.canvas))
                                .child(
                                    div()
                                        .font_family(FONT_DISPLAY)
                                        .text_color(TOKENS.canvas)
                                        .text_size(px(11.0))
                                        .font_weight(FontWeight::BOLD)
                                        .child("GO LIVE"),
                                )
                                .child(keycap("F5", true)),
                        ),
                ),
        )
        .child(
            div()
                .w_full()
                .px(px(12.0))
                .py(px(5.0))
                .bg(TOKENS.pane)
                .border_b_1()
                .border_color(TOKENS.gridline)
                .flex()
                .items_center()
                .gap(px(8.0))
                .child(deck_icon(DeckIcon::Palette, IconSize::Sm, TOKENS.ink_muted))
                .child(
                    div()
                        .font_family(FONT_MONO)
                        .text_color(TOKENS.ink_muted)
                        .text_size(px(9.5))
                        .child("THEME"),
                )
                .child(theme_strip),
        )
        .child(grid)
}
