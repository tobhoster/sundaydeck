use gpui::{
    Div, FontWeight, ObjectFit, ParentElement, Pixels, Styled, StyledImage, div, img, px, rgb, rgba,
};
use sundaydeck_core::{CustomSlideLayout, HorizontalAlign, ServiceTimer, Theme};

use crate::design::tokens::{
    FONT_BODY, FONT_DISPLAY, FONT_MONO, RADIUS_CONTROL, RADIUS_PILL, RADIUS_TILE, TOKENS,
    rgba8_to_gpui, weight_from_u16,
};
use crate::media::load_background_image;
pub use crate::overlay_badges::{render_audience_alert_banner, render_audience_timer_badge};

/// Renders all curated [`CustomSlideLayout`] templates cleanly, inheriting the active [`Theme`]'s
/// typography, colors, and safe margins so volunteer-authored slides always look professionally typeset.
#[allow(clippy::too_many_arguments)]
pub fn render_custom_slide_layout(
    layout: &CustomSlideLayout,
    revealed_steps: Option<usize>,
    theme: &Theme,
    active_timer: Option<&ServiceTimer>,
    now_ms: i64,
    scale: f32,
    pad_t: Pixels,
    pad_r: Pixels,
    pad_b: Pixels,
    pad_l: Pixels,
) -> Div {
    let text_color = rgba8_to_gpui(theme.text_color);
    let weight = weight_from_u16(theme.font_weight);

    match layout {
        CustomSlideLayout::TitleSubtitle {
            title,
            subtitle,
            align,
        } => {
            let mut col = padded_stage(pad_t, pad_r, pad_b, pad_l, *align)
                .justify_center()
                .gap(px((18.0 * scale).max(4.0)));
            col = col.child(
                div()
                    .w(px((64.0 * scale).max(12.0)))
                    .h(px((4.0 * scale).max(1.5)))
                    .rounded(px(RADIUS_PILL))
                    .bg(TOKENS.signal_preview),
            );
            col = col.child(
                align_text(div(), *align)
                    .w_full()
                    .text_color(text_color)
                    .text_size(px((86.0 * scale).max(11.0)))
                    .line_height(px((100.0 * scale).max(13.0)))
                    .font_weight(weight)
                    .child(title.clone()),
            );
            if !subtitle.trim().is_empty() {
                col = col.child(
                    align_text(div(), *align)
                        .w_full()
                        .font_family(FONT_BODY)
                        .text_color(text_color.alpha(0.76))
                        .text_size(px((34.0 * scale).max(7.5)))
                        .line_height(px((46.0 * scale).max(10.0)))
                        .font_weight(FontWeight::MEDIUM)
                        .child(subtitle.clone()),
                );
            }
            col
        }
        CustomSlideLayout::BigStatement {
            statement,
            attribution,
            align,
        } => {
            let mut col = padded_stage(pad_t, pad_r, pad_b, pad_l, *align)
                .justify_center()
                .gap(px((24.0 * scale).max(5.0)));
            col = col.child(
                align_text(div(), *align)
                    .w_full()
                    .text_color(text_color)
                    .text_size(px((68.0 * scale).max(10.0)))
                    .line_height(px((88.0 * scale).max(12.5)))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(format!("\u{201C}{statement}\u{201D}")),
            );
            if let Some(attr) = attribution
                && !attr.trim().is_empty()
            {
                col = col.child(
                    align_text(div(), *align)
                        .w_full()
                        .font_family(FONT_BODY)
                        .text_color(TOKENS.signal_preview)
                        .text_size(px((28.0 * scale).max(7.0)))
                        .font_weight(FontWeight::MEDIUM)
                        .child(format!("\u{2014} {attr}")),
                );
            }
            col
        }
        CustomSlideLayout::BulletPoints {
            heading,
            bullets,
            progressive_reveal,
        } => {
            let visible_count = if *progressive_reveal {
                revealed_steps.unwrap_or(1).clamp(1, bullets.len().max(1))
            } else {
                bullets.len()
            };

            let mut list = div()
                .w_full()
                .flex()
                .flex_col()
                .gap(px((18.0 * scale).max(4.0)));

            for bullet in bullets.iter().take(visible_count) {
                list = list.child(
                    div()
                        .w_full()
                        .flex()
                        .items_start()
                        .gap(px((18.0 * scale).max(5.0)))
                        .child(
                            div()
                                .mt(px((16.0 * scale).max(4.0)))
                                .w(px((12.0 * scale).max(3.0)))
                                .h(px((12.0 * scale).max(3.0)))
                                .flex_shrink_0()
                                .rounded(px(RADIUS_PILL))
                                .bg(TOKENS.signal_preview),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .text_color(text_color)
                                .text_size(px((44.0 * scale).max(8.5)))
                                .line_height(px((58.0 * scale).max(11.0)))
                                .font_weight(FontWeight::MEDIUM)
                                .child(bullet.clone()),
                        ),
                );
            }

            padded_stage(pad_t, pad_r, pad_b, pad_l, HorizontalAlign::Left)
                .justify_center()
                .gap(px((28.0 * scale).max(6.0)))
                .child(
                    div()
                        .w_full()
                        .pb(px((14.0 * scale).max(3.0)))
                        .border_b(px((2.0 * scale).max(1.0)))
                        .border_color(text_color.alpha(0.22))
                        .font_family(FONT_DISPLAY)
                        .text_color(text_color)
                        .text_size(px((54.0 * scale).max(9.5)))
                        .font_weight(FontWeight::BOLD)
                        .child(heading.clone()),
                )
                .child(list)
        }
        CustomSlideLayout::ImageTextSplit {
            heading,
            body,
            image_uri,
            image_on_left,
        } => {
            let media_box = div()
                .flex_1()
                .min_w_0()
                .h_full()
                .rounded(px((RADIUS_TILE * 2.0 * scale).max(4.0)))
                .overflow_hidden()
                .border(px((2.0 * scale).max(1.0)))
                .border_color(TOKENS.gridline)
                .bg(TOKENS.recessed)
                .flex()
                .items_center()
                .justify_center()
                .child(
                    if let Some(render_img) = image_uri.as_deref().and_then(load_background_image) {
                        div()
                            .size_full()
                            .child(img(render_img).size_full().object_fit(ObjectFit::Cover))
                    } else {
                        div()
                            .font_family(FONT_BODY)
                            .text_color(TOKENS.ink_muted)
                            .text_size(px((24.0 * scale).max(7.0)))
                            .child("Image Slot")
                    },
                );

            let text_box = div()
                .flex_1()
                .min_w_0()
                .h_full()
                .flex()
                .flex_col()
                .justify_center()
                .gap(px((18.0 * scale).max(4.0)))
                .child(
                    div()
                        .w_full()
                        .font_family(FONT_DISPLAY)
                        .text_color(text_color)
                        .text_size(px((54.0 * scale).max(9.5)))
                        .line_height(px((66.0 * scale).max(11.5)))
                        .font_weight(FontWeight::BOLD)
                        .child(heading.clone()),
                )
                .child(
                    div()
                        .w_full()
                        .font_family(FONT_BODY)
                        .text_color(text_color.alpha(0.84))
                        .text_size(px((34.0 * scale).max(7.5)))
                        .line_height(px((48.0 * scale).max(10.0)))
                        .child(body.clone()),
                );

            let row = div()
                .relative()
                .size_full()
                .pt(pad_t)
                .pr(pad_r)
                .pb(pad_b)
                .pl(pad_l)
                .flex()
                .flex_row()
                .items_center()
                .gap(px((44.0 * scale).max(10.0)));

            if *image_on_left {
                row.child(media_box).child(text_box)
            } else {
                row.child(text_box).child(media_box)
            }
        }
        CustomSlideLayout::FullBleedImage { image_uri, caption } => {
            let mut full = div()
                .absolute()
                .inset_0()
                .size_full()
                .bg(rgb(0x0B0C0E))
                .flex()
                .items_center()
                .justify_center();

            if let Some(render_img) = image_uri.as_deref().and_then(load_background_image) {
                full = full.child(img(render_img).size_full().object_fit(ObjectFit::Cover));
            }
            if let Some(cap) = caption
                && !cap.trim().is_empty()
            {
                full = full.child(
                    div()
                        .absolute()
                        .bottom(px((36.0 * scale).max(6.0)))
                        .px(px((28.0 * scale).max(8.0)))
                        .py(px((12.0 * scale).max(3.5)))
                        .rounded(px((RADIUS_CONTROL * 1.5 * scale).max(3.0)))
                        .bg(rgba(0x1012_16D6))
                        .border(px((1.5 * scale).max(1.0)))
                        .border_color(TOKENS.gridline)
                        .font_family(FONT_DISPLAY)
                        .text_color(TOKENS.ink_primary)
                        .text_size(px((32.0 * scale).max(7.5)))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(cap.clone()),
                );
            }
            full
        }
        CustomSlideLayout::CountdownSlide {
            heading,
            subtitle,
            duration_secs,
        } => {
            let formatted = match active_timer {
                Some(timer) => timer.display(now_ms).formatted,
                None => format!("{:02}:{:02}", duration_secs / 60, duration_secs % 60),
            };
            let mut col = padded_stage(pad_t, pad_r, pad_b, pad_l, HorizontalAlign::Center)
                .justify_center()
                .gap(px((20.0 * scale).max(4.5)))
                .child(
                    div()
                        .font_family(FONT_DISPLAY)
                        .text_color(text_color.alpha(0.86))
                        .text_size(px((44.0 * scale).max(8.5)))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(heading.clone()),
                )
                .child(
                    div()
                        .px(px((42.0 * scale).max(10.0)))
                        .py(px((16.0 * scale).max(4.0)))
                        .rounded(px((RADIUS_TILE * 2.0 * scale).max(4.0)))
                        .bg(rgba(0x1415_18CC))
                        .border(px((2.0 * scale).max(1.0)))
                        .border_color(TOKENS.gridline)
                        .font_family(FONT_MONO)
                        .text_color(TOKENS.signal_preview)
                        .text_size(px((112.0 * scale).max(16.0)))
                        .font_weight(FontWeight::BOLD)
                        .child(formatted),
                );
            if let Some(sub) = subtitle
                && !sub.trim().is_empty()
            {
                col = col.child(
                    div()
                        .font_family(FONT_BODY)
                        .text_color(text_color.alpha(0.72))
                        .text_size(px((26.0 * scale).max(6.5)))
                        .child(sub.clone()),
                );
            }
            col
        }
        CustomSlideLayout::LiturgyResponse { title, lines } => {
            let mut list = div()
                .w_full()
                .flex()
                .flex_col()
                .gap(px((18.0 * scale).max(4.0)));
            for line in lines {
                let is_cong = line.speaker.is_congregation_emphasis();
                let mut item = div()
                    .w_full()
                    .flex()
                    .flex_col()
                    .gap(px((4.0 * scale).max(1.5)))
                    .child(
                        div()
                            .flex()
                            .items_baseline()
                            .gap(px((12.0 * scale).max(3.5)))
                            .child(
                                div()
                                    .font_family(FONT_BODY)
                                    .text_color(if is_cong {
                                        TOKENS.signal_preview
                                    } else {
                                        text_color.alpha(0.62)
                                    })
                                    .text_size(px((24.0 * scale).max(6.5)))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(format!("{}:", line.speaker.label())),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .text_color(if is_cong {
                                        text_color
                                    } else {
                                        text_color.alpha(0.78)
                                    })
                                    .text_size(px(
                                        (if is_cong { 44.0 } else { 36.0 } * scale).max(8.0)
                                    ))
                                    .line_height(px(
                                        (if is_cong { 56.0 } else { 46.0 } * scale).max(10.0)
                                    ))
                                    .font_weight(if is_cong {
                                        FontWeight::BOLD
                                    } else {
                                        FontWeight::NORMAL
                                    })
                                    .child(line.text.clone()),
                            ),
                    );
                if let Some(sec) = &line.secondary_text {
                    item = item.child(
                        div()
                            .pl(px((22.0 * scale).max(5.0)))
                            .font_family(FONT_BODY)
                            .text_color(text_color.alpha(0.58))
                            .text_size(px((26.0 * scale).max(6.5)))
                            .child(sec.clone()),
                    );
                }
                list = list.child(item);
            }
            padded_stage(pad_t, pad_r, pad_b, pad_l, HorizontalAlign::Left)
                .justify_center()
                .gap(px((22.0 * scale).max(5.0)))
                .child(
                    div()
                        .w_full()
                        .pb(px((10.0 * scale).max(2.5)))
                        .border_b(px((2.0 * scale).max(1.0)))
                        .border_color(text_color.alpha(0.22))
                        .font_family(FONT_DISPLAY)
                        .text_color(text_color)
                        .text_size(px((42.0 * scale).max(8.5)))
                        .font_weight(FontWeight::BOLD)
                        .child(title.clone()),
                )
                .child(list)
        }
    }
}

fn padded_stage(
    pad_t: Pixels,
    pad_r: Pixels,
    pad_b: Pixels,
    pad_l: Pixels,
    align: HorizontalAlign,
) -> Div {
    let base = div()
        .relative()
        .size_full()
        .pt(pad_t)
        .pr(pad_r)
        .pb(pad_b)
        .pl(pad_l)
        .flex()
        .flex_col();
    match align {
        HorizontalAlign::Left => base.items_start(),
        HorizontalAlign::Center => base.items_center(),
        HorizontalAlign::Right => base.items_end(),
    }
}

fn align_text(el: Div, align: HorizontalAlign) -> Div {
    match align {
        HorizontalAlign::Left => el.text_left(),
        HorizontalAlign::Center => el.text_center(),
        HorizontalAlign::Right => el.text_right(),
    }
}
