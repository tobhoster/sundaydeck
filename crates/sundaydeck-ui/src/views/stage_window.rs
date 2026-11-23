use chrono::Local;
use gpui::{
    Context, Div, Entity, FontWeight, IntoElement, ParentElement, Render, Styled, Subscription,
    Window, div, px, rgb,
};
use sundaydeck_core::{OutputOverride, StageRenderState, TimerUrgency};

use crate::design::icons::{DeckIcon, IconSize, deck_icon};
use crate::design::tokens::{
    FONT_BODY, FONT_DISPLAY, FONT_LYRIC_BOLD, FONT_MONO, RADIUS_CONTROL, RADIUS_PILL, TOKENS,
};
use crate::store::PresentationStore;

/// Stage Confidence Monitor (Output Window 2):
/// Pure `#000000` background, oversized warm-white current slide text, muted 40% opacity
/// next-slide preview below a thin horizontal rule, and top-right tabular mono clock + timer.
pub struct StageMonitorView {
    store: Entity<PresentationStore>,
    clock_override: Option<String>,
    _subscription: Subscription,
}

impl StageMonitorView {
    pub fn new(store: Entity<PresentationStore>, cx: &mut Context<Self>) -> Self {
        let subscription = cx.observe(&store, |_, _, cx| cx.notify());
        Self {
            store,
            clock_override: None,
            _subscription: subscription,
        }
    }

    pub fn with_fixed_clock(mut self, clock_text: impl Into<String>) -> Self {
        self.clock_override = Some(clock_text.into());
        self
    }

    fn formatted_clock(&self) -> String {
        self.clock_override
            .clone()
            .unwrap_or_else(|| Local::now().format("%I:%M %p").to_string())
    }
}

impl Render for StageMonitorView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let store_ref = self.store.read(cx);
        let snapshot = store_ref.snapshot();
        let now_ms = store_ref.now_epoch_ms();
        let stage = &snapshot.stage;
        let width_px = f32::from(window.viewport_size().width).max(640.0);
        let scale = (width_px / 1280.0).clamp(0.55, 2.0);

        let has_banner = stage.stage_message.is_some();
        let mut root = div()
            .size_full()
            .bg(rgb(0x000000))
            .font_family(FONT_BODY)
            .flex()
            .flex_col()
            .overflow_hidden();

        root = root.child(render_stage_top_bar(
            stage,
            &self.formatted_clock(),
            now_ms,
            scale,
        ));

        if let Some(note) = &stage.stage_message {
            root = root.child(render_private_stage_banner(note, scale));
        }

        root = root
            .child(render_current_slide_section(stage, has_banner, scale))
            .child(div().w_full().h(px(1.0)).bg(TOKENS.gridline))
            .child(render_next_slide_section(stage, scale));

        root
    }
}

fn render_stage_top_bar(stage: &StageRenderState, clock: &str, now_ms: i64, scale: f32) -> Div {
    let title = stage.live_title.as_deref().unwrap_or("Waiting to begin");
    let pos_text = stage
        .slide_position
        .map(|(cur, total)| format!("Slide {cur} of {total}"))
        .unwrap_or_default();

    let (status_label, status_dot) = match stage.active_override {
        OutputOverride::None => ("Live", TOKENS.signal_live_dot),
        OutputOverride::ClearText => ("Words Hidden", TOKENS.signal_override),
        OutputOverride::Black => ("Black Screen", TOKENS.signal_override),
        OutputOverride::Logo => ("Showing Logo", TOKENS.signal_override),
    };

    let mut right_cluster = div().flex().items_center().gap(px(18.0 * scale));
    if let Some(timer) = &stage.active_timer {
        let disp = timer.display(now_ms);
        let (t_dot, t_fg, t_border) = match disp.urgency {
            TimerUrgency::Normal => (TOKENS.signal_voice, TOKENS.ink_primary, TOKENS.gridline),
            TimerUrgency::Warning => (
                TOKENS.signal_preview,
                TOKENS.signal_preview,
                TOKENS.signal_preview,
            ),
            TimerUrgency::Overtime => (
                TOKENS.signal_live_dot,
                TOKENS.signal_live_dot,
                TOKENS.signal_live,
            ),
        };
        right_cluster = right_cluster.child(
            div()
                .px(px(12.0 * scale))
                .py(px(5.0 * scale))
                .rounded(px(RADIUS_CONTROL))
                .bg(TOKENS.recessed)
                .border_1()
                .border_color(t_border)
                .flex()
                .items_center()
                .gap(px(10.0 * scale))
                .child(
                    div()
                        .w(px(7.0 * scale))
                        .h(px(7.0 * scale))
                        .rounded(px(RADIUS_PILL))
                        .bg(t_dot),
                )
                .child(
                    div()
                        .font_family(FONT_BODY)
                        .text_color(TOKENS.ink_secondary)
                        .text_size(px(13.0 * scale))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(timer.label.clone()),
                )
                .child(
                    div()
                        .font_family(FONT_MONO)
                        .text_color(t_fg)
                        .text_size(px(26.0 * scale))
                        .font_weight(FontWeight::BOLD)
                        .child(disp.formatted),
                ),
        );
    }

    right_cluster = right_cluster.child(
        div()
            .flex()
            .items_center()
            .gap(px(8.0 * scale))
            .child(deck_icon(DeckIcon::Clock, IconSize::Lg, TOKENS.ink_muted))
            .child(
                div()
                    .font_family(FONT_MONO)
                    .text_color(TOKENS.ink_primary)
                    .text_size(px(26.0 * scale))
                    .font_weight(FontWeight::BOLD)
                    .child(clock.to_owned()),
            ),
    );

    div()
        .w_full()
        .px(px(28.0 * scale))
        .py(px(14.0 * scale))
        .border_b_1()
        .border_color(TOKENS.gridline)
        .flex()
        .items_center()
        .justify_between()
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(14.0 * scale))
                .child(
                    div()
                        .px(px(11.0 * scale))
                        .py(px(4.5 * scale))
                        .rounded(px(RADIUS_PILL))
                        .bg(TOKENS.status_surface)
                        .border_1()
                        .border_color(TOKENS.gridline)
                        .flex()
                        .items_center()
                        .gap(px(7.0 * scale))
                        .child(
                            div()
                                .w(px(7.0 * scale))
                                .h(px(7.0 * scale))
                                .rounded(px(RADIUS_PILL))
                                .bg(status_dot),
                        )
                        .child(
                            div()
                                .font_family(FONT_BODY)
                                .text_color(TOKENS.ink_primary)
                                .text_size(px(13.0 * scale))
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(status_label),
                        ),
                )
                .child(
                    div()
                        .font_family(FONT_DISPLAY)
                        .text_color(TOKENS.ink_primary)
                        .text_size(px(20.0 * scale))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(title.to_owned()),
                )
                .child(
                    div()
                        .font_family(FONT_BODY)
                        .text_color(TOKENS.ink_secondary)
                        .text_size(px(14.0 * scale))
                        .font_weight(FontWeight::MEDIUM)
                        .child(pos_text),
                ),
        )
        .child(right_cluster)
}

fn render_private_stage_banner(note: &str, scale: f32) -> Div {
    div()
        .w_full()
        .px(px(28.0 * scale))
        .py(px(10.0 * scale))
        .bg(TOKENS.recessed)
        .border_b_1()
        .border_color(TOKENS.hairline)
        .border_l_4()
        .border_color(TOKENS.signal_preview)
        .flex()
        .items_center()
        .justify_between()
        .gap(px(16.0 * scale))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(12.0 * scale))
                .child(
                    div()
                        .px(px(10.0 * scale))
                        .py(px(3.5 * scale))
                        .rounded(px(RADIUS_PILL))
                        .bg(TOKENS.status_surface)
                        .border_1()
                        .border_color(TOKENS.gridline)
                        .flex()
                        .items_center()
                        .gap(px(6.0 * scale))
                        .child(
                            div()
                                .w(px(6.0 * scale))
                                .h(px(6.0 * scale))
                                .rounded(px(RADIUS_PILL))
                                .bg(TOKENS.signal_preview),
                        )
                        .child(
                            div()
                                .font_family(FONT_BODY)
                                .text_color(TOKENS.ink_primary)
                                .text_size(px(12.5 * scale))
                                .font_weight(FontWeight::SEMIBOLD)
                                .child("Stage Note"),
                        ),
                )
                .child(
                    div()
                        .font_family(FONT_DISPLAY)
                        .text_color(TOKENS.ink_primary)
                        .text_size(px(21.0 * scale))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(note.to_owned()),
                ),
        )
        .child(
            div()
                .font_family(FONT_BODY)
                .text_color(TOKENS.ink_secondary)
                .text_size(px(12.5 * scale))
                .child("Visible on stage only"),
        )
}

fn render_current_slide_section(stage: &StageRenderState, has_banner: bool, scale: f32) -> Div {
    let (label, body) = match &stage.current_slide {
        Some(slide) => (slide.label.clone(), slide.content.primary_text().to_owned()),
        None => (
            "Waiting to begin".to_owned(),
            "No slide on screen yet.\nClick Show on Screen to begin.".to_owned(),
        ),
    };

    let line_count = body.lines().count();
    let base_font = if has_banner && line_count >= 4 {
        44.0
    } else if line_count >= 4 {
        50.0
    } else {
        56.0
    };
    let line_h = base_font * 1.24;

    let mut lines_box = div()
        .flex_1()
        .w_full()
        .overflow_hidden()
        .flex()
        .flex_col()
        .justify_center()
        .gap(px(8.0 * scale));

    for line in body.lines() {
        lines_box = lines_box.child(
            div()
                .font_family(FONT_LYRIC_BOLD)
                .text_color(TOKENS.ink_primary)
                .text_size(px(base_font * scale))
                .line_height(px(line_h * scale))
                .font_weight(FontWeight::EXTRA_BOLD)
                .child(line.to_owned()),
        );
    }

    div()
        .flex_1()
        .w_full()
        .overflow_hidden()
        .px(px(36.0 * scale))
        .py(px(22.0 * scale))
        .flex()
        .flex_col()
        .gap(px(10.0 * scale))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(10.0 * scale))
                .child(
                    div()
                        .w(px(6.0 * scale))
                        .h(px(6.0 * scale))
                        .rounded(px(99.0))
                        .bg(TOKENS.signal_live),
                )
                .child(
                    div()
                        .font_family(FONT_BODY)
                        .text_color(TOKENS.ink_secondary)
                        .text_size(px(14.0 * scale))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(format!("Now · {label}")),
                ),
        )
        .child(lines_box)
}

fn render_next_slide_section(stage: &StageRenderState, scale: f32) -> Div {
    let (next_label, next_body) = match &stage.next_slide {
        Some(slide) => (slide.label.clone(), slide.content.primary_text().to_owned()),
        None => ("End of item".to_owned(), "No next slide queued.".to_owned()),
    };

    let mut preview_lines = div()
        .flex_1()
        .overflow_hidden()
        .flex()
        .flex_col()
        .gap(px(4.0 * scale));

    for line in next_body.lines().take(3) {
        preview_lines = preview_lines.child(
            div()
                .font_family(FONT_BODY)
                .text_color(TOKENS.ink_primary)
                .opacity(0.40)
                .text_size(px(24.0 * scale))
                .line_height(px(31.0 * scale))
                .font_weight(FontWeight::MEDIUM)
                .child(line.to_owned()),
        );
    }

    div()
        .w_full()
        .h(px(172.0 * scale))
        .overflow_hidden()
        .px(px(36.0 * scale))
        .py(px(18.0 * scale))
        .flex()
        .flex_col()
        .gap(px(8.0 * scale))
        .child(
            div()
                .font_family(FONT_BODY)
                .text_color(TOKENS.signal_preview)
                .opacity(0.90)
                .text_size(px(14.0 * scale))
                .font_weight(FontWeight::SEMIBOLD)
                .child(format!("Next Up · {next_label}")),
        )
        .child(preview_lines)
}
