use gpui::{
    AnyElement, App, Div, FontWeight, IntoElement, ObjectFit, ParentElement, Pixels, SharedString,
    Styled, StyledImage, div, img, px, rgb,
};
use sundaydeck_core::{
    AudienceAlert, AudienceRenderState, HorizontalAlign, ServiceTimer, Slide, SlideContent, Theme,
    VerticalAlign,
};

use crate::custom_slide_renderer::{
    render_audience_alert_banner, render_audience_timer_badge, render_custom_slide_layout,
};
use crate::design::fonts::resolve_font_family as resolve_bundled_font;
use crate::design::tokens::{
    FONT_DISPLAY, FONT_MONO, RADIUS_CONTROL, TOKENS, rgba8_to_gpui, weight_from_u16,
};
use crate::media::{apply_theme_background, load_background_image};

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
            logo_uri,
            fallback_title,
            audience_timer,
            ..
        } => render_logo_splash(
            logo_uri.as_deref(),
            fallback_title,
            audience_timer.as_ref(),
            now_ms,
            scale,
        )
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

/// Renders ONLY the theme background layer (used by layer-aware Audience transitions so backgrounds
/// stay steady while foreground lyrics/verses animate between slides of the same item).
pub fn render_audience_background_only(theme: &Theme, now_ms: i64) -> AnyElement {
    apply_theme_background(
        div().relative().size_full().overflow_hidden(),
        &theme.background,
        now_ms,
    )
    .into_any_element()
}

/// Renders ONLY the foreground slide content + alerts/timers over a transparent container
/// for layer-aware slide transitions (`SoftRise`, `GentleSlide`, `ZoomDissolve`, `Crossfade`).
pub fn render_audience_foreground_only(
    state: &AudienceRenderState,
    canvas_width_px: f32,
    now_ms: i64,
    cx: &App,
) -> AnyElement {
    let scale = (canvas_width_px / Theme::REFERENCE_WIDTH_PX).clamp(0.12, 4.0);
    let AudienceRenderState::Slide {
        slide,
        theme,
        show_text,
        alert,
        audience_timer,
        ..
    } = state
    else {
        return div().size_full().into_any_element();
    };
    let font_family = resolve_font_family(&theme.font_family, cx);
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
        .font_family(font_family);
    if *show_text {
        root = root.child(render_slide_foreground(
            slide,
            theme,
            audience_timer.as_ref(),
            now_ms,
            scale,
            pad_t,
            pad_r,
            pad_b,
            pad_l,
        ));
        if let Some(active_timer) = audience_timer.as_ref()
            && !matches!(
                slide.content,
                SlideContent::Custom {
                    layout: sundaydeck_core::CustomSlideLayout::CountdownSlide { .. },
                    ..
                }
            )
        {
            root = root.child(render_audience_timer_badge(active_timer, now_ms, scale));
        }
        if let Some(active_alert) = alert.as_ref() {
            root = root.child(render_audience_alert_banner(active_alert, scale));
        }
    }
    root.into_any_element()
}

/// Resolves a theme font safely against bundled and loaded fonts so missing fonts never panic.
pub fn resolve_font_family(requested: &str, cx: &App) -> SharedString {
    resolve_bundled_font(requested, cx)
}

fn render_logo_splash(
    logo_uri: Option<&str>,
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

    let logo_mark = if let Some(render_img) = logo_uri.and_then(load_background_image) {
        div()
            .w(px((360.0 * scale).max(44.0)))
            .h(px((200.0 * scale).max(26.0)))
            .rounded(px((12.0 * scale).max(3.0)))
            .overflow_hidden()
            .border(border_w)
            .border_color(TOKENS.gridline)
            .bg(TOKENS.pane)
            .flex()
            .items_center()
            .justify_center()
            .child(img(render_img).size_full().object_fit(ObjectFit::Contain))
    } else {
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
            )
    };

    let mut root = div()
        .size_full()
        .bg(TOKENS.canvas)
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(gap)
        .child(logo_mark)
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
            slide, theme, timer, now_ms, scale, pad_t, pad_r, pad_b, pad_l,
        ));
        if let Some(active_timer) = timer
            && !matches!(
                slide.content,
                SlideContent::Custom {
                    layout: sundaydeck_core::CustomSlideLayout::CountdownSlide { .. },
                    ..
                }
            )
        {
            root = root.child(render_audience_timer_badge(active_timer, now_ms, scale));
        }
        if let Some(active_alert) = alert {
            root = root.child(render_audience_alert_banner(active_alert, scale));
        }
    }

    root
}

#[allow(clippy::too_many_arguments)]
fn render_slide_foreground(
    slide: &Slide,
    theme: &Theme,
    timer: Option<&ServiceTimer>,
    now_ms: i64,
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
        SlideContent::Image { uri, caption, .. } => {
            if let Some(render_img) = uri.as_deref().and_then(load_background_image) {
                return div()
                    .absolute()
                    .inset_0()
                    .size_full()
                    .bg(rgb(0x0B0C0E))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(img(render_img).size_full().object_fit(ObjectFit::Contain));
            }
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
        SlideContent::Custom {
            layout,
            revealed_steps,
        } => {
            return render_custom_slide_layout(
                layout,
                *revealed_steps,
                theme,
                timer,
                now_ms,
                scale,
                pad_t,
                pad_r,
                pad_b,
                pad_l,
            );
        }
        SlideContent::Blank => {}
    }

    container
}
