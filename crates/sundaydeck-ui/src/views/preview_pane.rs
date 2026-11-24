use std::sync::Arc;

use gpui::{
    Context, Div, FontWeight, InteractiveElement, ObjectFit, ParentElement,
    StatefulInteractiveElement, Styled, StyledImage, div, img, px, rgb, rgba,
};
use sundaydeck_core::{
    AudienceRenderState, BackgroundSpec, PresentationCommand, ReflowMode, SlideContent,
    SlideTransition, TransitionDirection,
};

use crate::design::fonts::resolve_font_family;
use crate::design::icons::{DeckIcon, IconSize, deck_icon};
use crate::design::primitives::{
    next_up_caption, primary_ivory_button_body, section_meta_label, status_dot_label,
};
use crate::design::tokens::{
    FONT_BODY, FONT_DISPLAY, PANE_HEADER_HEIGHT, RADIUS_CONTROL, RADIUS_PILL, RADIUS_TILE, TOKENS,
};
use crate::media::{apply_theme_background, load_background_image};
use crate::renderer::render_audience_canvas;
use crate::views::operator_workspace::OperatorWorkspace;

const TILE_W_PX: f32 = 256.0;
const TILE_H_PX: f32 = 144.0; // exact 16:9

/// Top half of the Center Pane:
/// Clean 16:9 sanctuary slide cards with typographic dot+label metadata (`Verse 1`, `Chorus`),
/// `1.5px` warm-amber Next Up ring + bottom `Next up · Space` caption, real Unsplash photo /
/// looping motion video backgrounds, and a warm paper-ivory `Show on Screen` primary button.
pub fn render_preview_section(
    workspace: &OperatorWorkspace,
    cx: &mut Context<OperatorWorkspace>,
) -> Div {
    let store = workspace.store.read(cx);
    let snapshot = store.snapshot();
    let now_ms = store.now_epoch_ms();
    let preview = snapshot.preview.clone();
    let active_reflow = store.preview_reflow_mode;
    let themes = store.themes.clone();
    let active_theme = store.active_theme();

    let (title, slide_count, active_idx, slides, preview_font, preview_bg_spec) = match &preview {
        Some(cursor) => (
            cursor.title.clone(),
            cursor.len(),
            cursor.active_index,
            cursor.slides.to_vec(),
            resolve_font_family(&cursor.theme.font_family, cx),
            cursor.theme.background.clone(),
        ),
        None => (
            "Select an item from Service Order".to_owned(),
            0,
            0,
            Vec::new(),
            resolve_font_family(&active_theme.font_family, cx),
            active_theme.background.clone(),
        ),
    };

    let preview_loop_secs = preview.as_ref().and_then(|c| c.loop_interval_secs);

    let mut reflow_bar = div()
        .flex()
        .items_center()
        .gap(px(2.0))
        .p(px(2.0))
        .bg(TOKENS.recessed)
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

    let mut theme_strip = div()
        .id("preview-theme-strip")
        .flex_1()
        .min_w_0()
        .overflow_hidden()
        .flex()
        .items_center()
        .gap(px(5.0));

    for theme in themes {
        let is_active = theme.id == active_theme.id;
        let theme_click = Arc::clone(&theme);
        let kind_dot = match &theme.background {
            BackgroundSpec::VideoLoop { .. } => Some(TOKENS.signal_preview),
            BackgroundSpec::Image { .. } => Some(rgb(0x6B9BF2)),
            BackgroundSpec::LiveCamera { .. } => Some(TOKENS.signal_voice),
            BackgroundSpec::Solid { color } if color.g() > 150 && color.r() < 40 => {
                Some(TOKENS.signal_voice)
            }
            BackgroundSpec::LinearGradient { .. } | BackgroundSpec::Solid { .. } => None,
        };

        let mut chip = div()
            .id(format!("preview-theme-{}", theme.id))
            .flex_shrink_0()
            .px(px(9.0))
            .py(px(3.5))
            .rounded(px(RADIUS_PILL))
            .cursor_pointer()
            .bg(if is_active {
                TOKENS.elevated
            } else {
                rgba(0x0000_0000)
            })
            .hover(|s| s.bg(TOKENS.elevated_hover))
            .border_1()
            .border_color(if is_active {
                TOKENS.focus_border
            } else {
                rgba(0x0000_0000)
            })
            .flex()
            .items_center()
            .gap(px(5.0))
            .on_click(cx.listener(move |this, _, _, cx| {
                this.store.update(cx, |s, cx| {
                    let _ = s.dispatch(
                        PresentationCommand::ApplyLiveTheme(Arc::clone(&theme_click)),
                        cx,
                    );
                });
            }));

        if let Some(dot) = kind_dot {
            chip = chip.child(div().w(px(5.0)).h(px(5.0)).rounded(px(RADIUS_PILL)).bg(dot));
        }

        theme_strip = theme_strip.child(
            chip.child(
                div()
                    .font_family(FONT_BODY)
                    .text_color(if is_active {
                        TOKENS.ink_primary
                    } else {
                        TOKENS.ink_secondary
                    })
                    .text_size(px(11.0))
                    .font_weight(FontWeight::MEDIUM)
                    .child(short_theme_label(&theme.name)),
            ),
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

    let has_media_bg = matches!(
        preview_bg_spec,
        BackgroundSpec::Image { .. } | BackgroundSpec::VideoLoop { .. }
    );

    for (idx, slide) in slides.iter().enumerate() {
        let is_selected = idx == active_idx;
        let sec_kind = match &slide.content {
            SlideContent::Text { section_kind, .. } => *section_kind,
            _ => None,
        };
        let raw_text = slide.content.primary_text();
        let line_count = raw_text.lines().count();
        let body_size = if raw_text.len() > 110 || line_count >= 4 {
            10.5
        } else {
            12.0
        };
        let body_lh = body_size * 1.35;

        let base_tile = div()
            .relative()
            .w(px(TILE_W_PX))
            .h(px(TILE_H_PX))
            .rounded(px(RADIUS_TILE))
            .overflow_hidden()
            .border(px(if is_selected { 1.5 } else { 1.0 }))
            .border_color(if is_selected {
                TOKENS.signal_preview
            } else {
                TOKENS.gridline
            })
            .flex()
            .flex_col();

        let slide_image = match &slide.content {
            SlideContent::Image { uri, .. } => uri.as_deref().and_then(load_background_image),
            _ => None,
        };
        let is_custom_layout = matches!(slide.content, SlideContent::Custom { .. });

        let mut styled_tile = if is_custom_layout {
            let custom_theme = preview
                .as_ref()
                .map(|c| Arc::clone(&c.theme))
                .unwrap_or_else(|| Arc::clone(&active_theme));
            let tr = slide
                .transition_override
                .unwrap_or(SlideTransition::DEFAULT_SOFT_RISE);
            let mini_state = AudienceRenderState::Slide {
                slide: slide.clone(),
                theme: custom_theme,
                show_text: true,
                alert: None,
                audience_timer: None,
                transition_ms: tr.duration_ms(),
                active_transition: tr,
                transition_direction: TransitionDirection::Forward,
            };
            base_tile.bg(rgb(0x0B0C0E)).child(
                div()
                    .absolute()
                    .inset_0()
                    .size_full()
                    .child(render_audience_canvas(&mini_state, TILE_W_PX, now_ms, cx)),
            )
        } else if let Some(render_img) = slide_image.clone() {
            base_tile
                .bg(rgb(0x0B0C0E))
                .child(
                    img(render_img)
                        .absolute()
                        .inset_0()
                        .size_full()
                        .object_fit(ObjectFit::Cover),
                )
                .child(div().absolute().inset_0().bg(rgba(0x0B0C_0E66)))
        } else if has_media_bg {
            apply_theme_background(base_tile, &preview_bg_spec, now_ms)
        } else {
            base_tile.bg(TOKENS.slide_card)
        };

        styled_tile = styled_tile.child(
            div()
                .relative()
                .w_full()
                .px(px(11.0))
                .pt(px(8.0))
                .pb(px(3.0))
                .flex()
                .items_center()
                .justify_between()
                .gap(px(6.0))
                .child(section_meta_label(&slide.label, sec_kind))
                .child(
                    div()
                        .font_family(FONT_BODY)
                        .text_color(if slide_image.is_some() {
                            TOKENS.ink_primary
                        } else {
                            TOKENS.ink_muted
                        })
                        .text_size(px(10.5))
                        .child(format!("{}", idx + 1)),
                ),
        );

        if slide_image.is_some() || is_custom_layout {
            styled_tile = styled_tile.child(div().flex_1());
        } else {
            styled_tile = styled_tile.child(
                div()
                    .relative()
                    .flex_1()
                    .w_full()
                    .min_w_0()
                    .px(px(14.0))
                    .py(px(5.0))
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
            );
        }

        if is_selected {
            styled_tile = styled_tile
                .child(
                    div()
                        .relative()
                        .w_full()
                        .pb(px(6.0))
                        .flex()
                        .justify_center()
                        .child(next_up_caption()),
                )
                .child(
                    div()
                        .relative()
                        .w_full()
                        .h(px(2.0))
                        .bg(TOKENS.signal_preview),
                );
        }

        grid = grid.child(
            div()
                .id(format!("preview-slide-{idx}"))
                .cursor_pointer()
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.store.update(cx, |s, cx| {
                        let _ = s.dispatch(PresentationCommand::SelectPreviewSlide(idx), cx);
                        if s.snapshot().direct_click_live {
                            let _ = s.dispatch(PresentationCommand::GoLive, cx);
                        }
                    });
                }))
                .child(styled_tile),
        );
    }

    let count_copy = match preview_loop_secs {
        Some(secs) => format!("{slide_count} slides · Loop {secs}s"),
        None if slide_count == 1 => "1 slide".to_owned(),
        None => format!("{slide_count} slides"),
    };

    div()
        .flex_1()
        .w_full()
        .border_b_1()
        .border_color(TOKENS.hairline)
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
                .border_color(TOKENS.hairline)
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
                        .gap(px(10.0))
                        .child(div().flex_shrink_0().child(status_dot_label(
                            "Next Up",
                            TOKENS.signal_preview,
                            TOKENS.ink_secondary,
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
                                .cursor_pointer()
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.store.update(cx, |s, cx| {
                                        let _ = s.dispatch(PresentationCommand::GoLive, cx);
                                    });
                                }))
                                .child(primary_ivory_button_body(
                                    Some(DeckIcon::PlayFill),
                                    "Show on Screen",
                                    preview.is_some(),
                                )),
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
                .border_color(TOKENS.hairline)
                .flex()
                .items_center()
                .gap(px(8.0))
                .child(deck_icon(DeckIcon::Palette, IconSize::Sm, TOKENS.ink_muted))
                .child(theme_strip),
        )
        .child(grid)
}

fn short_theme_label(name: &str) -> String {
    match name {
        "Classic Sanctuary Dark" => "Sanctuary Dark".to_owned(),
        "Warm Sanctuary Amber" => "Warm Amber".to_owned(),
        "Scripture Clean" => "Scripture".to_owned(),
        "Modern Display" => "Modern".to_owned(),
        "Sanctuary Mountains" => "Mountains".to_owned(),
        "Golden Light Motion" => "Golden Motion".to_owned(),
        "Still Waters Scripture" => "Still Waters".to_owned(),
        other => other.to_owned(),
    }
}
