use gpui::{
    Context, Div, FontWeight, InteractiveElement, ObjectFit, ParentElement,
    StatefulInteractiveElement, Styled, StyledImage, div, img, px, rgb, rgba,
};
use sundaydeck_core::{OutputOverride, PresentationCommand, SlideContent};

use crate::design::icons::{DeckIcon, IconSize, deck_icon};
use crate::design::motion::tally_pulse_dot;
use crate::design::primitives::{
    VoiceSyncMode, live_meta_indicator, next_up_caption, section_meta_label,
};
use crate::design::tokens::{
    FONT_BODY, FONT_DISPLAY, LIVE_DECK_WIDTH, MONITOR_HEIGHT_PX, MONITOR_WIDTH_PX,
    PANE_HEADER_HEIGHT, RADIUS_CONTROL, RADIUS_PILL, RADIUS_TILE, TOKENS,
};
use crate::media::load_background_image;
use crate::renderer::render_audience_canvas;
use crate::views::operator_workspace::OperatorWorkspace;
use crate::views::override_controls::{render_override_buttons, render_stage_cue_strip};

/// Right Pane (On Screen Now):
/// Realistic 16:9 Live Preview framed with a crisp `1.5px` deep crimson ring (or amber-terracotta
/// ring when an override is latched), sculpted tactile screen keys, calm matte slide list,
/// and private Stage Screen cue strip.
pub fn render_live_pane(workspace: &OperatorWorkspace, cx: &mut Context<OperatorWorkspace>) -> Div {
    let store_ref = workspace.store.read(cx);
    let snapshot = store_ref.snapshot();
    let now_ms = store_ref.now_epoch_ms();
    let motion_mode = store_ref.motion_mode;
    let voice_mode = store_ref.voice_sync_mode;
    let voice_telemetry = store_ref.voice_telemetry.clone();
    let live = snapshot.live.clone();
    let active_override = snapshot.output_override;
    let stage_note = snapshot.stage.stage_message.clone();
    let has_live = live.is_some();

    let (live_title, pos_label, active_idx, live_slides) = match &live {
        Some(cursor) => {
            let pos = match cursor.loop_interval_secs {
                Some(secs) => format!(
                    "{} of {} · {}s",
                    cursor.active_index + 1,
                    cursor.len(),
                    secs
                ),
                None => format!("{} of {}", cursor.active_index + 1, cursor.len()),
            };
            (
                cursor.title.clone(),
                pos,
                cursor.active_index,
                cursor.slides.to_vec(),
            )
        }
        None => ("Nothing on screen".to_owned(), String::new(), 0, Vec::new()),
    };

    let monitor_border = if active_override != OutputOverride::None {
        TOKENS.signal_override
    } else if has_live {
        TOKENS.signal_live
    } else {
        TOKENS.gridline
    };

    let override_caption = match active_override {
        OutputOverride::None => None,
        OutputOverride::ClearText => Some("Words Hidden · Active"),
        OutputOverride::Black => Some("Black Screen · Active"),
        OutputOverride::Logo => Some("Showing Logo · Active"),
    };

    let mut monitor = div()
        .relative()
        .w(px(MONITOR_WIDTH_PX))
        .h(px(MONITOR_HEIGHT_PX))
        .rounded(px(RADIUS_TILE))
        .overflow_hidden()
        .border(px(1.5))
        .border_color(monitor_border)
        .bg(rgb(0x0E0F12))
        .child(render_audience_canvas(
            &snapshot.audience,
            MONITOR_WIDTH_PX,
            now_ms,
            cx,
        ));

    if let Some(text) = override_caption {
        monitor = monitor.child(
            div()
                .absolute()
                .top(px(8.0))
                .left(px(8.0))
                .px(px(10.0))
                .py(px(4.0))
                .rounded(px(RADIUS_PILL))
                .bg(TOKENS.recessed)
                .border_1()
                .border_color(TOKENS.signal_override)
                .flex()
                .items_center()
                .gap(px(6.0))
                .child(
                    div()
                        .w(px(6.0))
                        .h(px(6.0))
                        .rounded(px(RADIUS_PILL))
                        .bg(TOKENS.signal_override),
                )
                .child(
                    div()
                        .font_family(FONT_BODY)
                        .text_color(TOKENS.ink_primary)
                        .text_size(px(10.5))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(text),
                ),
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
        let is_suggested = !is_current && voice_telemetry.suggested_slide_index == Some(idx);
        let sec_kind = match &slide.content {
            SlideContent::Text { section_kind, .. } => *section_kind,
            _ => None,
        };
        let slide_thumb = match &slide.content {
            SlideContent::Image { uri, .. } => uri.as_deref().and_then(load_background_image),
            _ => None,
        };
        let excerpt: String = slide
            .content
            .primary_text()
            .lines()
            .take(2)
            .collect::<Vec<_>>()
            .join("\n");

        let body_row = if let Some(render_img) = slide_thumb {
            div()
                .w_full()
                .flex()
                .items_center()
                .gap(px(9.0))
                .child(
                    div()
                        .w(px(64.0))
                        .h(px(36.0))
                        .flex_shrink_0()
                        .rounded(px(4.0))
                        .overflow_hidden()
                        .border_1()
                        .border_color(TOKENS.gridline)
                        .bg(rgb(0x0B0C0E))
                        .child(img(render_img).size_full().object_fit(ObjectFit::Cover)),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .font_family(FONT_BODY)
                        .text_color(if is_current || is_suggested {
                            TOKENS.ink_primary
                        } else {
                            TOKENS.ink_secondary
                        })
                        .text_size(px(11.5))
                        .line_height(px(16.0))
                        .child(excerpt),
                )
        } else {
            div()
                .w_full()
                .min_w_0()
                .font_family(FONT_BODY)
                .text_color(if is_current || is_suggested {
                    TOKENS.ink_primary
                } else {
                    TOKENS.ink_secondary
                })
                .text_size(px(12.0))
                .line_height(px(17.0))
                .child(excerpt)
        };

        let mut card = div()
            .id(format!("live-slide-{idx}"))
            .w_full()
            .rounded(px(RADIUS_TILE))
            .overflow_hidden()
            .cursor_pointer()
            .bg(if is_current || is_suggested {
                TOKENS.elevated
            } else {
                TOKENS.slide_card
            })
            .hover(|s| s.bg(TOKENS.elevated_hover))
            .border(px(if is_current || is_suggested { 1.5 } else { 1.0 }))
            .border_color(if is_current {
                TOKENS.signal_live
            } else if is_suggested {
                TOKENS.signal_preview
            } else {
                TOKENS.gridline
            })
            .flex()
            .flex_col()
            .on_click(cx.listener(move |this, _, _, cx| {
                this.store.update(cx, |s, cx| {
                    let _ = s.dispatch(PresentationCommand::SelectLiveSlide(idx), cx);
                });
            }))
            .child(
                div()
                    .w_full()
                    .px(px(12.0))
                    .pt(px(9.0))
                    .pb(px(10.0))
                    .flex()
                    .flex_col()
                    .gap(px(6.0))
                    .child(
                        div()
                            .w_full()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap(px(6.0))
                            .child(section_meta_label(&slide.label, sec_kind))
                            .child(if is_current {
                                live_meta_indicator()
                            } else if is_suggested {
                                next_up_caption()
                            } else {
                                div()
                                    .font_family(FONT_BODY)
                                    .text_color(TOKENS.ink_muted)
                                    .text_size(px(11.0))
                                    .child(format!("{}", idx + 1))
                            }),
                    )
                    .child(body_row),
            );

        if is_current {
            let progress_pct = if voice_mode != VoiceSyncMode::Off
                && voice_telemetry.active_slide_progress > 0.02
            {
                voice_telemetry.active_slide_progress.clamp(0.05, 1.0)
            } else {
                1.0
            };
            let fill_px = (336.0 * progress_pct).round();
            card = card.child(
                div()
                    .w_full()
                    .h(px(2.0))
                    .bg(rgba(0x0000_0040))
                    .child(div().w(px(fill_px)).h_full().bg(rgba(0xF5F3_EF8C))),
            );
        } else if is_suggested {
            card = card.child(div().w_full().h(px(2.0)).bg(TOKENS.signal_preview));
        }

        live_list = live_list.child(card);
    }

    div()
        .w(px(LIVE_DECK_WIDTH))
        .h_full()
        .bg(TOKENS.pane)
        .border_l_1()
        .border_color(TOKENS.hairline)
        .flex()
        .flex_col()
        .child(
            div()
                .w_full()
                .h(px(PANE_HEADER_HEIGHT))
                .px(px(14.0))
                .border_b_1()
                .border_color(TOKENS.hairline)
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
                            TOKENS.signal_live_dot,
                            has_live && active_override == OutputOverride::None,
                            motion_mode,
                        )))
                        .child(
                            div()
                                .flex_shrink_0()
                                .font_family(FONT_BODY)
                                .text_color(TOKENS.ink_primary)
                                .text_size(px(11.5))
                                .font_weight(FontWeight::SEMIBOLD)
                                .child("Live"),
                        )
                        .child(div().w(px(1.0)).h(px(12.0)).bg(TOKENS.hairline))
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
                                .bg(TOKENS.status_surface)
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
                                .bg(TOKENS.status_surface)
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
                                .bg(TOKENS.status_surface)
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
        .child({
            let mut deck_top = div()
                .p(px(14.0))
                .border_b_1()
                .border_color(TOKENS.hairline)
                .flex()
                .flex_col()
                .gap(px(10.0))
                .child(monitor)
                .child(render_override_buttons(active_override, has_live, cx));

            if voice_mode != VoiceSyncMode::Off && !voice_telemetry.last_transcript.is_empty() {
                let dot_col = if voice_mode == VoiceSyncMode::Auto {
                    TOKENS.signal_voice
                } else {
                    TOKENS.signal_preview
                };
                deck_top = deck_top.child(
                    div()
                        .w_full()
                        .px(px(10.0))
                        .py(px(6.5))
                        .rounded(px(RADIUS_CONTROL))
                        .bg(TOKENS.recessed)
                        .border_1()
                        .border_color(TOKENS.hairline)
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(px(8.0))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .overflow_hidden()
                                .whitespace_nowrap()
                                .flex()
                                .items_center()
                                .gap(px(6.0))
                                .child(
                                    div()
                                        .w(px(6.0))
                                        .h(px(6.0))
                                        .flex_shrink_0()
                                        .rounded(px(RADIUS_PILL))
                                        .bg(dot_col),
                                )
                                .child(
                                    div()
                                        .font_family(FONT_BODY)
                                        .text_color(TOKENS.ink_secondary)
                                        .text_size(px(11.0))
                                        .child(format!("\"{}\"", voice_telemetry.last_transcript)),
                                ),
                        )
                        .child(
                            div()
                                .flex_shrink_0()
                                .font_family(FONT_BODY)
                                .text_color(TOKENS.ink_muted)
                                .text_size(px(10.5))
                                .font_weight(FontWeight::MEDIUM)
                                .child(format!("{:.0}%", voice_telemetry.confidence * 100.0)),
                        ),
                );
            }
            deck_top
        })
        .child(live_list)
        .child(render_stage_cue_strip(stage_note.as_deref(), cx))
}
