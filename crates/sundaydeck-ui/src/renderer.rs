use gpui::{
    AnyElement, App, Div, FontWeight, IntoElement, ParentElement, Pixels, SharedString, Styled,
    div, px, rgb,
};
use sundaydeck_core::{
    AlertPosition, AudienceAlert, AudienceRenderState, HorizontalAlign, ServiceTimer, Slide,
    SlideContent, Theme, TimerUrgency, VerticalAlign,
};

use crate::design::fonts::resolve_font_family as resolve_bundled_font;
use crate::design::tokens::{
    FONT_BODY, FONT_DISPLAY, FONT_MONO, RADIUS_CONTROL, TOKENS, rgba8_to_gpui, weight_from_u16,
};
use crate::media::apply_theme_background;

/// Renders the exact Audience Output canvas at any scale (both full-resolution projector
/// and miniature 16:9 Operator Monitor preview) for pixel-faithful WYSIWYG.
pub fn render_audience_canvas(
    state: &AudienceRenderState,
    canvas_width_px: f32,
    now_ms: i64,
    cx: &App,
) -> AnyElement {
    let scale = (canvas_width_px / Theme::REFERENCE_WIDTH_PX).clamp(0.12, 4.0);
    match state {
        AudienceRenderState::Black { .. } => div().size_full().bg(rgb(0x000000)).into_any_element(),
        AudienceRenderState::Logo {
            fallback_title,
            audience_timer,
            ..
        } => render_logo_splash(fallback_title, audience_timer.as_ref(), now_ms, scale)
            .into_any_element(),
        AudienceRenderState::Slide {
            slide,
            theme,
            show_text,
            alert,
            audience_timer,
            ..
        } => render_slide_canvas(
            slide,
            theme,
            *show_text,
            alert.as_ref(),
            audience_timer.as_ref(),
            now_ms,
            scale,
            cx,
        )
        .into_any_element(),
    }
}

/// Resolves a theme font safely against bundled and loaded fonts so missing fonts never panic.
pub fn resolve_font_family(requested: &str, cx: &App) -> SharedString {
    resolve_bundled_font(requested, cx)
}

fn render_logo_splash(
    church_name: &str,
    timer: Option<&ServiceTimer>,
    now_ms: i64,
    scale: f32,
) -> Div {
    let title = if church_name.trim().is_empty() {
        "SundayDeck"
    } else {
        church_name.trim()
    };

    let mark_size = px((112.0 * scale).max(18.0));
    let border_w = px((2.0 * scale).max(1.0));
    let title_px = px((62.0 * scale).max(10.0));
    let sub_px = px((20.0 * scale).max(6.0));
    let gap = px((22.0 * scale).max(4.0));

    let mut root = div()
        .size_full()
        .bg(TOKENS.canvas)
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(gap)
        .child(
            div()
                .w(mark_size)
                .h(mark_size)
                .rounded(px((4.0 * scale).max(2.0)))
                .border(border_w)
                .border_color(TOKENS.gridline)
                .bg(TOKENS.pane)
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap(px((6.0 * scale).max(1.0)))
                .child(
                    div()
                        .w(px((40.0 * scale).max(6.0)))
                        .h(px((3.0 * scale).max(1.0)))
                        .bg(TOKENS.signal_preview),
                )
                .child(
                    div()
                        .font_family(FONT_DISPLAY)
                        .text_color(TOKENS.ink_primary)
                        .text_size(px((44.0 * scale).max(9.0)))
                        .font_weight(FontWeight::BOLD)
                        .child("SD"),
                ),
        )
        .child(
            div()
                .font_family(FONT_DISPLAY)
                .text_color(TOKENS.ink_primary)
                .text_size(title_px)
                .font_weight(FontWeight::BOLD)
                .child(title.to_owned()),
        )
        .child(
            div()
                .font_family(FONT_MONO)
                .text_color(TOKENS.ink_secondary)
                .text_size(sub_px)
                .font_weight(FontWeight::MEDIUM)
                .child("SUNDAY GATHERING"),
        );

    if let Some(active_timer) = timer {
        let disp = active_timer.display(now_ms);
        root = root.child(
            div()
                .mt(px((18.0 * scale).max(4.0)))
                .px(px((32.0 * scale).max(8.0)))
                .py(px((12.0 * scale).max(3.0)))
                .rounded(px((RADIUS_CONTROL * 2.0 * scale).max(2.0)))
                .bg(TOKENS.pane)
                .border(px((2.0 * scale).max(1.0)))
                .border_color(TOKENS.gridline)
                .flex()
                .items_center()
                .gap(px((18.0 * scale).max(5.0)))
                .child(
                    div()
                        .font_family(FONT_MONO)
                        .text_color(TOKENS.ink_secondary)
                        .text_size(px((20.0 * scale).max(5.5)))
                        .font_weight(FontWeight::MEDIUM)
                        .child(active_timer.label.to_ascii_uppercase()),
                )
                .child(
                    div()
                        .font_family(FONT_MONO)
                        .text_color(TOKENS.signal_preview)
                        .text_size(px((46.0 * scale).max(9.5)))
                        .font_weight(FontWeight::BOLD)
                        .child(disp.formatted),
                ),
        );
    }

    root
}

#[allow(clippy::too_many_arguments)] // Canvas render parameters mirror AudienceRenderState::Slide fields directly.
fn render_slide_canvas(
    slide: &Slide,
    theme: &Theme,
    show_text: bool,
    alert: Option<&AudienceAlert>,
    timer: Option<&ServiceTimer>,
    now_ms: i64,
    scale: f32,
    cx: &App,
) -> Div {
    let font_family = resolve_font_family(&theme.font_family, cx);

    let canvas_w = Theme::REFERENCE_WIDTH_PX * scale;
    let canvas_h = Theme::REFERENCE_HEIGHT_PX * scale;
    let pad_t = px(canvas_h * theme.margins.top_pct);
    let pad_r = px(canvas_w * theme.margins.right_pct);
    let pad_b = px(canvas_h * theme.margins.bottom_pct);
    let pad_l = px(canvas_w * theme.margins.left_pct);

    let mut root = apply_theme_background(
        div()
            .relative()
            .size_full()
            .overflow_hidden()
            .font_family(font_family),
        &theme.background,
        now_ms,
    );

    if show_text {
        root = root.child(render_slide_foreground(
            slide, theme, scale, pad_t, pad_r, pad_b, pad_l,
        ));
        if let Some(active_timer) = timer {
            root = root.child(render_audience_timer_badge(active_timer, now_ms, scale));
        }
        if let Some(active_alert) = alert {
            root = root.child(render_audience_alert_banner(active_alert, scale));
        }
    }

    root
}

fn render_slide_foreground(
    slide: &Slide,
    theme: &Theme,
    scale: f32,
    pad_t: Pixels,
    pad_r: Pixels,
    pad_b: Pixels,
    pad_l: Pixels,
) -> Div {
    let mut container = div()
        .relative()
        .size_full()
        .pt(pad_t)
        .pr(pad_r)
        .pb(pad_b)
        .pl(pad_l)
        .flex()
        .flex_col();

    container = match theme.v_align {
        VerticalAlign::Top => container.justify_start(),
        VerticalAlign::Middle => container.justify_center(),
        VerticalAlign::Bottom => container.justify_end(),
    };

    container = match theme.h_align {
        HorizontalAlign::Left => container.items_start(),
        HorizontalAlign::Center => container.items_center(),
        HorizontalAlign::Right => container.items_end(),
    };

    match &slide.content {
        SlideContent::Text { body, footer, .. } => {
            let font_px = px((theme.font_size_pt * scale).max(9.0));
            let line_h = px((theme.font_size_pt * theme.line_height * scale).max(11.0));
            let text_color = rgba8_to_gpui(theme.text_color);
            let weight = weight_from_u16(theme.font_weight);

            let mut lines_col = div()
                .w_full()
                .flex()
                .flex_col()
                .gap(px((8.0 * scale).max(1.0)));

            lines_col = match theme.h_align {
                HorizontalAlign::Left => lines_col.items_start(),
                HorizontalAlign::Center => lines_col.items_center(),
                HorizontalAlign::Right => lines_col.items_end(),
            };

            for line in body.lines() {
                let mut line_el = div()
                    .text_color(text_color)
                    .text_size(font_px)
                    .line_height(line_h)
                    .font_weight(weight)
                    .child(line.to_owned());

                line_el = match theme.h_align {
                    HorizontalAlign::Left => line_el.text_left(),
                    HorizontalAlign::Center => line_el.text_center(),
                    HorizontalAlign::Right => line_el.text_right(),
                };
                lines_col = lines_col.child(line_el);
            }

            container = container.child(lines_col);

            if let Some(footer_text) = footer {
                let footer_px = px((theme.footer_font_size_pt * scale).max(6.5));
                let footer_pad_b = px((24.0 * scale).max(4.0));
                container = container.child(
                    div()
                        .absolute()
                        .bottom(footer_pad_b)
                        .left(pad_l)
                        .right(pad_r)
                        .flex()
                        .justify_center()
                        .font_family(FONT_MONO)
                        .text_color(text_color.alpha(0.62))
                        .text_size(footer_px)
                        .font_weight(FontWeight::MEDIUM)
                        .child(footer_text.clone()),
                );
            }
        }
        SlideContent::Image { caption, .. } => {
            container = container.child(
                div()
                    .px(px((24.0 * scale).max(6.0)))
                    .py(px((12.0 * scale).max(4.0)))
                    .rounded(px((RADIUS_CONTROL * scale).max(2.0)))
                    .bg(TOKENS.pane)
                    .border_1()
                    .border_color(TOKENS.gridline)
                    .font_family(FONT_DISPLAY)
                    .text_color(TOKENS.ink_primary)
                    .text_size(px((30.0 * scale).max(8.0)))
                    .child(caption.clone().unwrap_or_else(|| slide.label.clone())),
            );
        }
        SlideContent::Blank => {}
    }

    container
}

fn render_audience_timer_badge(timer: &ServiceTimer, now_ms: i64, scale: f32) -> Div {
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

fn render_audience_alert_banner(alert: &AudienceAlert, scale: f32) -> Div {
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
