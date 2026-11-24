use gpui::{Div, FontWeight, ParentElement, Styled, div, px};
use sundaydeck_core::{AlertPosition, AudienceAlert, ServiceTimer, TimerUrgency};

use crate::design::tokens::{FONT_BODY, FONT_MONO, RADIUS_CONTROL, TOKENS};

pub fn render_audience_timer_badge(timer: &ServiceTimer, now_ms: i64, scale: f32) -> Div {
    let disp = timer.display(now_ms);
    let accent = match disp.urgency {
        TimerUrgency::Normal => TOKENS.signal_voice,
        TimerUrgency::Warning => TOKENS.signal_preview,
        TimerUrgency::Overtime => TOKENS.signal_live,
    };
    let offset = px((24.0 * scale).max(4.0));

    div()
        .absolute()
        .top(offset)
        .left(offset)
        .px(px((20.0 * scale).max(5.0)))
        .py(px((9.0 * scale).max(2.5)))
        .rounded(px((RADIUS_CONTROL * 1.5 * scale).max(2.0)))
        .bg(TOKENS.pane)
        .border(px((2.0 * scale).max(1.0)))
        .border_color(TOKENS.gridline)
        .flex()
        .items_center()
        .gap(px((12.0 * scale).max(3.0)))
        .child(
            div()
                .font_family(FONT_MONO)
                .text_color(TOKENS.ink_secondary)
                .text_size(px((18.0 * scale).max(5.0)))
                .font_weight(FontWeight::MEDIUM)
                .child(timer.label.to_ascii_uppercase()),
        )
        .child(
            div()
                .font_family(FONT_MONO)
                .text_color(accent)
                .text_size(px((30.0 * scale).max(7.5)))
                .font_weight(FontWeight::BOLD)
                .child(disp.formatted),
        )
}

pub fn render_audience_alert_banner(alert: &AudienceAlert, scale: f32) -> Div {
    let pad_x = px((28.0 * scale).max(6.0));
    let pad_y = px((12.0 * scale).max(3.0));
    let font_px = px((28.0 * scale).max(7.0));
    let offset = px((24.0 * scale).max(4.0));
    let border_w = px((2.0 * scale).max(1.0));

    let base = div()
        .absolute()
        .px(pad_x)
        .py(pad_y)
        .border_color(TOKENS.signal_preview)
        .font_family(FONT_BODY)
        .text_color(TOKENS.ink_primary)
        .text_size(font_px)
        .font_weight(FontWeight::SEMIBOLD)
        .child(alert.message.clone());

    match alert.position {
        AlertPosition::TopRight => base
            .bg(TOKENS.pane)
            .border(border_w)
            .top(offset)
            .right(offset)
            .rounded(px((RADIUS_CONTROL * 1.5 * scale).max(2.0))),
        AlertPosition::TopLeft => base
            .bg(TOKENS.pane)
            .border(border_w)
            .top(offset)
            .left(offset)
            .rounded(px((RADIUS_CONTROL * 1.5 * scale).max(2.0))),
        AlertPosition::TopBar => base
            .bg(TOKENS.canvas)
            .border_b(border_w)
            .top(px(0.0))
            .left(px(0.0))
            .right(px(0.0))
            .flex()
            .justify_center(),
        AlertPosition::BottomBar => base
            .bg(TOKENS.canvas)
            .border_t(border_w)
            .bottom(px(0.0))
            .left(px(0.0))
            .right(px(0.0))
            .flex()
            .justify_center(),
    }
}
