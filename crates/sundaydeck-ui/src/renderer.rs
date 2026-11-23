use gpui::{
    AnyElement, App, Background, Div, FontWeight, IntoElement, ParentElement, Pixels, SharedString,
    Styled, div, linear_color_stop, linear_gradient, px, rgb, rgba,
};
use sundaydeck_core::{
    AlertPosition, AudienceAlert, AudienceRenderState, BackgroundSpec, HorizontalAlign,
    ServiceTimer, Slide, SlideContent, Theme, TimerUrgency, VerticalAlign,
};

use crate::palette::{rgba8_to_gpui, weight_from_u16};

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

/// Resolves a theme font safely against loaded platform fonts so missing system fonts
/// never panic during a live worship service.
pub fn resolve_font_family(requested: &str, cx: &App) -> SharedString {
    if requested.is_empty() || requested == ".SystemUIFont" {
        return SharedString::from(".SystemUIFont");
    }
    let available = cx.text_system().all_font_names();
    if available.iter().any(|name| name == requested) {
        SharedString::from(requested.to_owned())
    } else {
        SharedString::from(".SystemUIFont")
    }
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

    let bg = linear_gradient(
        155.0,
        linear_color_stop(rgb(0x0E172A), 0.0),
        linear_color_stop(rgb(0x050811), 1.0),
    );
    let badge_size = px((128.0 * scale).max(20.0));
    let badge_radius = px((28.0 * scale).max(5.0));
    let border_w = px((3.0 * scale).max(1.0));
    let title_px = px((64.0 * scale).max(11.0));
    let sub_px = px((22.0 * scale).max(7.0));
    let gap = px((24.0 * scale).max(5.0));

    let mut root = div()
        .size_full()
        .bg(bg)
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(gap)
        .font_family(".SystemUIFont")
        .child(
            div()
                .w(badge_size)
                .h(badge_size)
                .rounded(badge_radius)
                .border(border_w)
                .border_color(rgba(0xF59E_0BAA))
                .bg(rgba(0xF59E_0B1C))
                .flex()
                .items_center()
                .justify_center()
                .text_color(rgb(0xFCD34D))
                .text_size(px((56.0 * scale).max(10.0)))
                .font_weight(FontWeight::BOLD)
                .child("SD"),
        )
        .child(
            div()
                .text_color(rgb(0xF8FAFC))
                .text_size(title_px)
                .font_weight(FontWeight::BOLD)
                .child(title.to_owned()),
        )
        .child(
            div()
                .text_color(rgb(0x94A3B8))
                .text_size(sub_px)
                .font_weight(FontWeight::MEDIUM)
                .child("WELCOME  •  WORSHIP WITH US"),
        );

    if let Some(active_timer) = timer {
        let disp = active_timer.display(now_ms);
        root = root.child(
            div()
                .mt(px((18.0 * scale).max(4.0)))
                .px(px((34.0 * scale).max(8.0)))
                .py(px((14.0 * scale).max(4.0)))
                .rounded(px((14.0 * scale).max(4.0)))
                .bg(rgba(0x0F17_2AD8))
                .border(px((2.0 * scale).max(1.0)))
                .border_color(rgba(0xF59E_0B88))
                .flex()
                .items_center()
                .gap(px((18.0 * scale).max(5.0)))
                .child(
                    div()
                        .text_color(rgb(0x94A3B8))
                        .text_size(px((22.0 * scale).max(6.0)))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(active_timer.label.to_ascii_uppercase()),
                )
                .child(
                    div()
                        .text_color(rgb(0xFCD34D))
                        .text_size(px((48.0 * scale).max(10.0)))
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
    let bg = theme_background(&theme.background);

    let canvas_w = Theme::REFERENCE_WIDTH_PX * scale;
    let canvas_h = Theme::REFERENCE_HEIGHT_PX * scale;
    let pad_t = px(canvas_h * theme.margins.top_pct);
    let pad_r = px(canvas_w * theme.margins.right_pct);
    let pad_b = px(canvas_h * theme.margins.bottom_pct);
    let pad_l = px(canvas_w * theme.margins.left_pct);

    let mut root = div()
        .relative()
        .size_full()
        .overflow_hidden()
        .bg(bg)
        .font_family(font_family);

    if let BackgroundSpec::Image { scrim_opacity, .. } = &theme.background
        && *scrim_opacity > 0.0
    {
        root = root.child(
            div()
                .absolute()
                .inset_0()
                .bg(rgba(0x0000_00FF).alpha(scrim_opacity.clamp(0.0, 1.0))),
        );
    }

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
                        .text_color(text_color.alpha(0.68))
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
                    .py(px((14.0 * scale).max(4.0)))
                    .rounded(px((10.0 * scale).max(3.0)))
                    .bg(rgba(0x0000_00AA))
                    .text_color(rgb(0xE2E8F0))
                    .text_size(px((32.0 * scale).max(8.0)))
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
        TimerUrgency::Normal => rgb(0x6EE7B7),
        TimerUrgency::Warning => rgb(0xFCD34D),
        TimerUrgency::Overtime => rgb(0xFCA5A5),
    };
    let offset = px((24.0 * scale).max(4.0));

    div()
        .absolute()
        .top(offset)
        .left(offset)
        .px(px((22.0 * scale).max(6.0)))
        .py(px((10.0 * scale).max(3.0)))
        .rounded(px((10.0 * scale).max(3.0)))
        .bg(rgba(0x0B0F_17DD))
        .border(px((2.0 * scale).max(1.0)))
        .border_color(accent.alpha(0.75))
        .flex()
        .items_center()
        .gap(px((12.0 * scale).max(3.0)))
        .child(
            div()
                .text_color(rgb(0x94A3B8))
                .text_size(px((20.0 * scale).max(5.5)))
                .font_weight(FontWeight::SEMIBOLD)
                .child(timer.label.to_ascii_uppercase()),
        )
        .child(
            div()
                .text_color(accent)
                .text_size(px((32.0 * scale).max(8.0)))
                .font_weight(FontWeight::BOLD)
                .child(disp.formatted),
        )
}

fn render_audience_alert_banner(alert: &AudienceAlert, scale: f32) -> Div {
    let pad_x = px((28.0 * scale).max(6.0));
    let pad_y = px((12.0 * scale).max(3.0));
    let font_px = px((30.0 * scale).max(7.5));
    let offset = px((24.0 * scale).max(4.0));
    let border_w = px((2.0 * scale).max(1.0));

    let base = div()
        .absolute()
        .px(pad_x)
        .py(pad_y)
        .border_color(rgb(0xF59E0B))
        .text_color(rgb(0xFEF3C7))
        .text_size(font_px)
        .font_weight(FontWeight::SEMIBOLD)
        .child(alert.message.clone());

    match alert.position {
        AlertPosition::TopRight => base
            .bg(rgba(0x0B12_20F2))
            .border(border_w)
            .top(offset)
            .right(offset)
            .rounded(px((10.0 * scale).max(3.0))),
        AlertPosition::TopLeft => base
            .bg(rgba(0x0B12_20F2))
            .border(border_w)
            .top(offset)
            .left(offset)
            .rounded(px((10.0 * scale).max(3.0))),
        AlertPosition::TopBar => base
            .bg(rgb(0x0B1220))
            .border_b(border_w)
            .top(px(0.0))
            .left(px(0.0))
            .right(px(0.0))
            .flex()
            .justify_center(),
        AlertPosition::BottomBar => base
            .bg(rgb(0x0B1220))
            .border_t(border_w)
            .bottom(px(0.0))
            .left(px(0.0))
            .right(px(0.0))
            .flex()
            .justify_center(),
    }
}

fn theme_background(spec: &BackgroundSpec) -> Background {
    match spec {
        BackgroundSpec::Solid { color } => rgba8_to_gpui(*color).into(),
        BackgroundSpec::LinearGradient {
            start,
            end,
            angle_deg,
        } => linear_gradient(
            *angle_deg,
            linear_color_stop(rgba8_to_gpui(*start), 0.0),
            linear_color_stop(rgba8_to_gpui(*end), 1.0),
        ),
        BackgroundSpec::Image { .. } => linear_gradient(
            160.0,
            linear_color_stop(rgb(0x1E293B), 0.0),
            linear_color_stop(rgb(0x090D16), 1.0),
        ),
    }
}
