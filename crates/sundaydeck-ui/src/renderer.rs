use std::sync::Arc;

use gpui::{
    AnyElement, App, Div, Edges, FontWeight, IntoElement, ObjectFit, ParentElement, Pixels,
    RenderImage, Styled, StyledImage, div, img, px, rgb,
};
use sundaydeck_core::{
    AudienceAlert, AudienceRenderState, CustomSlideLayout, HorizontalAlign, ServiceTimer, Slide,
    SlideContent, Theme, VerticalAlign,
};

use crate::custom_slide_renderer::{
    PHOTO_MATTE, SlideCanvas, render_audience_alert_banner, render_audience_timer_badge,
    render_custom_slide_layout,
};
use crate::design::fonts::resolve_font_family;
use crate::design::tokens::{
    FONT_DISPLAY, FONT_MONO, RADIUS_CONTROL, TOKENS, rgba8_to_gpui, weight_from_u16,
};
use crate::media::{BackgroundPaint, apply_theme_background};
use crate::media_cache::{MediaLookup, background_image, background_image_lookup};
use crate::media_decode::{DEFAULT_CHURCH_LOGO_URI, ImageTier};

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
            render_logo_mark(logo_uri.as_deref(), scale, cx),
            fallback_title,
            scale,
        )
        .children(
            audience_timer
                .as_ref()
                .map(|timer| render_splash_timer(timer, now_ms, scale)),
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
pub fn render_audience_background_only(
    theme: &Theme,
    canvas_width_px: f32,
    now_ms: i64,
    cx: &App,
) -> AnyElement {
    apply_theme_background(
        div().relative().size_full().overflow_hidden(),
        BackgroundPaint {
            spec: &theme.background,
            now_ms,
            tier: ImageTier::for_canvas_width(canvas_width_px),
        },
        cx,
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

    let mut root = div()
        .relative()
        .size_full()
        .overflow_hidden()
        .font_family(font_family);
    if *show_text {
        let canvas = SlideCanvas {
            theme,
            timer: audience_timer.as_ref(),
            now_ms,
            scale,
            padding: safe_area(theme, scale),
            image: slide_image(&slide.content, canvas_tier(scale), cx),
        };
        root = root.children(slide_text_layers(slide, &canvas, alert.as_ref()));
    }
    root.into_any_element()
}

/// Decode tier for a canvas drawn at `scale` of the reference projector width.
fn canvas_tier(scale: f32) -> ImageTier {
    ImageTier::for_canvas_width(Theme::REFERENCE_WIDTH_PX * scale)
}

/// Safe-area padding from the theme's margins on a canvas drawn at `scale`.
fn safe_area(theme: &Theme, scale: f32) -> Edges<Pixels> {
    let canvas_w = Theme::REFERENCE_WIDTH_PX * scale;
    let canvas_h = Theme::REFERENCE_HEIGHT_PX * scale;
    Edges {
        top: px(canvas_h * theme.margins.top_pct),
        right: px(canvas_w * theme.margins.right_pct),
        bottom: px(canvas_h * theme.margins.bottom_pct),
        left: px(canvas_w * theme.margins.left_pct),
    }
}

/// The decode state of the image a slide paints (a photo slide or an image-bearing custom
/// layout); `Failed` when the slide has no image.
fn slide_image(content: &SlideContent, tier: ImageTier, cx: &App) -> MediaLookup<Arc<RenderImage>> {
    let uri = match content {
        SlideContent::Image { uri, .. }
        | SlideContent::Custom {
            layout:
                CustomSlideLayout::ImageTextSplit { image_uri: uri, .. }
                | CustomSlideLayout::FullBleedImage { image_uri: uri, .. },
            ..
        } => uri.as_deref(),
        _ => None,
    };
    match uri {
        Some(uri) => background_image_lookup(uri, tier, cx),
        None => MediaLookup::Failed,
    }
}

/// The thumbnail-tier image of a photo slide, once ready (custom layouts excluded).
pub(crate) fn photo_slide_thumbnail(content: &SlideContent, cx: &App) -> Option<Arc<RenderImage>> {
    match content {
        SlideContent::Image { uri, .. } => uri
            .as_deref()
            .and_then(|uri| background_image(uri, ImageTier::Thumbnail, cx)),
        _ => None,
    }
}

/// The text-mode layers stacked over a slide canvas, in paint order: the slide foreground, the
/// live timer badge (unless the slide is itself a countdown), then the alert banner.
fn slide_text_layers(
    slide: &Slide,
    canvas: &SlideCanvas<'_>,
    alert: Option<&AudienceAlert>,
) -> impl Iterator<Item = Div> + use<> {
    let foreground = render_slide_foreground(slide, canvas);
    let is_countdown = matches!(
        slide.content,
        SlideContent::Custom {
            layout: CustomSlideLayout::CountdownSlide { .. },
            ..
        }
    );
    let badge = canvas
        .timer
        .filter(|_| !is_countdown)
        .map(|timer| render_audience_timer_badge(timer, canvas.now_ms, canvas.scale));
    let banner = alert.map(|alert| render_audience_alert_banner(alert, canvas.scale));
    [Some(foreground), badge, banner].into_iter().flatten()
}

/// The church logo framed for the splash, else the bundled emblem, else a matte placeholder.
/// The frame stays empty while a decode is pending; fallbacks show only once a decode failed.
fn render_logo_mark(logo_uri: Option<&str>, scale: f32, cx: &App) -> Div {
    let tier = canvas_tier(scale);
    let logo = logo_uri.map_or(MediaLookup::Failed, |uri| {
        background_image_lookup(uri, tier, cx)
    });
    match logo {
        MediaLookup::Ready(logo) => framed_logo(logo, scale),
        MediaLookup::Pending => logo_mark_box(scale),
        MediaLookup::Failed => default_logo_mark(tier, scale, cx),
    }
}

fn framed_logo(logo: Arc<RenderImage>, scale: f32) -> Div {
    div()
        .w(px((360.0 * scale).max(44.0)))
        .h(px((200.0 * scale).max(26.0)))
        .rounded(px((12.0 * scale).max(3.0)))
        .overflow_hidden()
        .border(px((2.0 * scale).max(1.0)))
        .border_color(TOKENS.gridline)
        .bg(TOKENS.pane)
        .flex()
        .items_center()
        .justify_center()
        .child(img(logo).size_full().object_fit(ObjectFit::Contain))
}

fn logo_mark_box(scale: f32) -> Div {
    let mark_size = px((156.0 * scale).max(24.0));
    div().size(mark_size).flex().items_center().justify_center()
}

/// The bundled emblem, or a matte placeholder once the emblem failed to decode.
fn default_logo_mark(tier: ImageTier, scale: f32, cx: &App) -> Div {
    let mark = logo_mark_box(scale);
    match background_image_lookup(DEFAULT_CHURCH_LOGO_URI, tier, cx) {
        MediaLookup::Ready(emblem) => {
            mark.child(img(emblem).size_full().object_fit(ObjectFit::Contain))
        }
        MediaLookup::Pending => mark,
        MediaLookup::Failed => mark
            .rounded(px((14.0 * scale).max(3.0)))
            .border(px((2.0 * scale).max(1.0)))
            .border_color(TOKENS.gridline)
            .bg(TOKENS.pane)
            .child(
                div()
                    .w(px((48.0 * scale).max(8.0)))
                    .h(px((4.0 * scale).max(1.5)))
                    .bg(TOKENS.signal_preview),
            ),
    }
}

fn render_logo_splash(logo_mark: Div, church_name: &str, scale: f32) -> Div {
    let title = if church_name.trim().is_empty() {
        "SundayDeck"
    } else {
        church_name.trim()
    };

    let title_px = px((62.0 * scale).max(10.0));
    let sub_px = px((20.0 * scale).max(6.0));
    let gap = px((22.0 * scale).max(4.0));

    div()
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
        )
}

/// The service timer pill shown beneath the logo splash title.
fn render_splash_timer(timer: &ServiceTimer, now_ms: i64, scale: f32) -> Div {
    let disp = timer.display(now_ms);
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
                .child(timer.label.to_ascii_uppercase()),
        )
        .child(
            div()
                .font_family(FONT_MONO)
                .text_color(TOKENS.signal_preview)
                .text_size(px((46.0 * scale).max(9.5)))
                .font_weight(FontWeight::BOLD)
                .child(disp.formatted),
        )
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
    let tier = canvas_tier(scale);

    let mut root = apply_theme_background(
        div()
            .relative()
            .size_full()
            .overflow_hidden()
            .font_family(font_family),
        BackgroundPaint {
            spec: &theme.background,
            now_ms,
            tier,
        },
        cx,
    );

    if show_text {
        let canvas = SlideCanvas {
            theme,
            timer,
            now_ms,
            scale,
            padding: safe_area(theme, scale),
            image: slide_image(&slide.content, tier, cx),
        };
        root = root.children(slide_text_layers(slide, &canvas, alert));
    }

    root
}

fn render_slide_foreground(slide: &Slide, canvas: &SlideCanvas<'_>) -> Div {
    let SlideCanvas { theme, scale, .. } = *canvas;
    let Edges {
        top: pad_t,
        right: pad_r,
        bottom: pad_b,
        left: pad_l,
    } = canvas.padding;
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
            let matte = || div().absolute().inset_0().size_full().bg(rgb(PHOTO_MATTE));
            match canvas.image.clone() {
                MediaLookup::Ready(render_img) => {
                    return matte()
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(img(render_img).size_full().object_fit(ObjectFit::Contain));
                }
                MediaLookup::Pending => return matte(),
                MediaLookup::Failed => {}
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
            return render_custom_slide_layout(layout, *revealed_steps, canvas);
        }
        SlideContent::Blank => {}
    }

    container
}
