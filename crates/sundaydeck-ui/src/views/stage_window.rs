use chrono::Local;
use gpui::{
    Context, Div, Entity, FontWeight, IntoElement, ParentElement, Render, Styled, Subscription,
    Window, div, px, rgb, rgba,
};
use sundaydeck_core::{OutputOverride, StageRenderState};

use crate::palette::{
    BORDER_SUBTLE, LIVE_ACCENT, OVERRIDE_BLACK, OVERRIDE_CLEAR, OVERRIDE_LOGO, STAGE_BG,
    STAGE_NEXT_ACCENT, STAGE_PANEL, TEXT_MUTED, TEXT_PRIMARY, TEXT_SECONDARY, color,
};
use crate::store::PresentationStore;

/// High-contrast dark confidence monitor for worship leaders and speakers.
/// Displays current slide large, dimmed next-slide preview, clock, audience override
/// state, and private stage-only operator messages.
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

    /// Allows deterministic clock text during visual regression / review captures.
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
        let snapshot = self.store.read(cx).snapshot();
        let stage = &snapshot.stage;
        let width_px = f32::from(window.viewport_size().width).max(640.0);
        let scale = (width_px / 1280.0).clamp(0.55, 2.0);

        let mut root = div()
            .size_full()
            .bg(color(STAGE_BG))
            .font_family(".SystemUIFont")
            .flex()
            .flex_col()
            .overflow_hidden()
            .p(px(18.0 * scale))
            .gap(px(12.0 * scale));

        let has_banner = stage.stage_message.is_some();
        root = root.child(render_stage_top_bar(stage, &self.formatted_clock(), scale));

        if let Some(note) = &stage.stage_message {
            root = root.child(render_private_stage_banner(note, scale));
        }

        root = root
            .child(render_current_slide_panel(stage, has_banner, scale))
            .child(render_next_slide_panel(stage, scale));

        root
    }
}

fn render_stage_top_bar(stage: &StageRenderState, clock: &str, scale: f32) -> Div {
    let title = stage
        .live_title
        .as_deref()
        .unwrap_or("Standby — Waiting for Live Slide");
    let pos_text = stage
        .slide_position
        .map(|(cur, total)| format!("Slide {cur} of {total}"))
        .unwrap_or_else(|| "No Active Deck".to_owned());

    let (status_label, status_bg, status_fg) = match stage.active_override {
        OutputOverride::None => ("AUDIENCE: LIVE", rgb(0x064E3B), rgb(0x6EE7B7)),
        OutputOverride::ClearText => (
            "AUDIENCE: TEXT CLEARED",
            color(OVERRIDE_CLEAR).alpha(0.24),
            rgb(0x7DD3FC),
        ),
        OutputOverride::Black => (
            "AUDIENCE: BLACKED OUT",
            color(OVERRIDE_BLACK).alpha(0.28),
            rgb(0xFDBA74),
        ),
        OutputOverride::Logo => (
            "AUDIENCE: LOGO SPLASH",
            color(OVERRIDE_LOGO).alpha(0.28),
            rgb(0xE9D5FF),
        ),
    };

    div()
        .w_full()
        .px(px(20.0 * scale))
        .py(px(14.0 * scale))
        .rounded(px(12.0 * scale))
        .bg(color(STAGE_PANEL))
        .border_1()
        .border_color(color(BORDER_SUBTLE))
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
                        .text_color(color(TEXT_PRIMARY))
                        .text_size(px(20.0 * scale))
                        .font_weight(FontWeight::BOLD)
                        .child(title.to_owned()),
                )
                .child(
                    div()
                        .px(px(10.0 * scale))
                        .py(px(4.0 * scale))
                        .rounded(px(6.0 * scale))
                        .bg(rgba(0xFFFFFF12))
                        .text_color(color(TEXT_SECONDARY))
                        .text_size(px(14.0 * scale))
                        .font_weight(FontWeight::MEDIUM)
                        .child(pos_text),
                ),
        )
        .child(
            div()
                .px(px(14.0 * scale))
                .py(px(6.0 * scale))
                .rounded(px(999.0))
                .bg(status_bg)
                .text_color(status_fg)
                .text_size(px(14.0 * scale))
                .font_weight(FontWeight::BOLD)
                .child(status_label),
        )
        .child(
            div()
                .text_color(color(TEXT_PRIMARY))
                .text_size(px(28.0 * scale))
                .font_weight(FontWeight::BOLD)
                .child(clock.to_owned()),
        )
}

fn render_private_stage_banner(note: &str, scale: f32) -> Div {
    div()
        .w_full()
        .px(px(20.0 * scale))
        .py(px(12.0 * scale))
        .rounded(px(10.0 * scale))
        .bg(rgba(0xF59E_0B26))
        .border_2()
        .border_color(color(STAGE_NEXT_ACCENT))
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
                        .py(px(4.0 * scale))
                        .rounded(px(6.0 * scale))
                        .bg(color(STAGE_NEXT_ACCENT))
                        .text_color(rgb(0x0B0F17))
                        .text_size(px(13.0 * scale))
                        .font_weight(FontWeight::BOLD)
                        .child("STAGE ONLY"),
                )
                .child(
                    div()
                        .text_color(rgb(0xFEF3C7))
                        .text_size(px(22.0 * scale))
                        .font_weight(FontWeight::BOLD)
                        .child(note.to_owned()),
                ),
        )
        .child(
            div()
                .text_color(rgb(0xFDE68A))
                .text_size(px(13.0 * scale))
                .font_weight(FontWeight::MEDIUM)
                .child("Hidden from Audience"),
        )
}

fn render_current_slide_panel(stage: &StageRenderState, has_banner: bool, scale: f32) -> Div {
    let (label, body) = match &stage.current_slide {
        Some(slide) => (slide.label.clone(), slide.content.primary_text().to_owned()),
        None => (
            "STANDBY".to_owned(),
            "No slide is live yet.\nSelect an item and press F5 (Go Live).".to_owned(),
        ),
    };

    let line_count = body.lines().count();
    let base_font = if has_banner && line_count >= 4 {
        40.0
    } else if line_count >= 4 {
        44.0
    } else {
        48.0
    };
    let line_h = base_font * 1.24;

    let mut lines_box = div()
        .flex_1()
        .w_full()
        .overflow_hidden()
        .flex()
        .flex_col()
        .justify_center()
        .gap(px(6.0 * scale));

    for line in body.lines() {
        lines_box = lines_box.child(
            div()
                .text_color(color(TEXT_PRIMARY))
                .text_size(px(base_font * scale))
                .line_height(px(line_h * scale))
                .font_weight(FontWeight::BOLD)
                .child(line.to_owned()),
        );
    }

    div()
        .flex_1()
        .w_full()
        .overflow_hidden()
        .p(px(22.0 * scale))
        .rounded(px(14.0 * scale))
        .bg(color(STAGE_PANEL))
        .border_l_4()
        .border_color(color(LIVE_ACCENT))
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
                        .px(px(10.0 * scale))
                        .py(px(3.0 * scale))
                        .rounded(px(6.0 * scale))
                        .bg(rgba(0xEF44_4433))
                        .text_color(rgb(0xFCA5A5))
                        .text_size(px(13.0 * scale))
                        .font_weight(FontWeight::BOLD)
                        .child("CURRENT SLIDE"),
                )
                .child(
                    div()
                        .text_color(color(TEXT_SECONDARY))
                        .text_size(px(15.0 * scale))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(label),
                ),
        )
        .child(lines_box)
}

fn render_next_slide_panel(stage: &StageRenderState, scale: f32) -> Div {
    let (next_label, next_body) = match &stage.next_slide {
        Some(slide) => (slide.label.clone(), slide.content.primary_text().to_owned()),
        None => (
            "END OF DECK".to_owned(),
            "No upcoming slide queued.".to_owned(),
        ),
    };

    let mut preview_lines = div()
        .flex_1()
        .overflow_hidden()
        .flex()
        .flex_col()
        .gap(px(3.0 * scale));

    for line in next_body.lines().take(3) {
        preview_lines = preview_lines.child(
            div()
                .text_color(if stage.next_slide.is_some() {
                    rgb(0xCBD5E1)
                } else {
                    color(TEXT_MUTED)
                })
                .text_size(px(22.0 * scale))
                .line_height(px(28.0 * scale))
                .font_weight(FontWeight::MEDIUM)
                .child(line.to_owned()),
        );
    }

    div()
        .w_full()
        .h(px(164.0 * scale))
        .overflow_hidden()
        .p(px(18.0 * scale))
        .rounded(px(14.0 * scale))
        .bg(rgba(0x0F14_1FCC))
        .border_l_4()
        .border_color(color(STAGE_NEXT_ACCENT))
        .flex()
        .flex_col()
        .gap(px(8.0 * scale))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(10.0 * scale))
                .child(
                    div()
                        .px(px(10.0 * scale))
                        .py(px(3.0 * scale))
                        .rounded(px(6.0 * scale))
                        .bg(rgba(0xF59E_0B2B))
                        .text_color(rgb(0xFDE68A))
                        .text_size(px(13.0 * scale))
                        .font_weight(FontWeight::BOLD)
                        .child("NEXT SLIDE"),
                )
                .child(
                    div()
                        .text_color(color(TEXT_SECONDARY))
                        .text_size(px(14.0 * scale))
                        .font_weight(FontWeight::MEDIUM)
                        .child(next_label),
                ),
        )
        .child(preview_lines)
}
