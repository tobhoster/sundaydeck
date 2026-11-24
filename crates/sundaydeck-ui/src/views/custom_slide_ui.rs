use std::sync::Arc;

use gpui::{
    Context, Div, FontWeight, InteractiveElement, ParentElement, StatefulInteractiveElement,
    Styled, div, px, rgb,
};
use sundaydeck_core::{
    AudienceRenderState, HorizontalAlign, Slide, SlideContent, SlideTransition, TransitionDirection,
};

use crate::design::icons::{DeckIcon, IconSize, deck_icon};
use crate::design::primitives::{
    neutral_option_chip, primary_ivory_button_body, sculpted_secondary_button_body,
    status_dot_label,
};
use crate::design::tokens::{
    FONT_BODY, FONT_DISPLAY, RADIUS_CONTROL, RADIUS_MODAL, RADIUS_TILE, TOKENS,
};
use crate::renderer::render_audience_canvas;
use crate::views::custom_slide_modal::{
    BUNDLED_SPLIT_IMAGES, CUSTOM_LOOP_OPTIONS, CustomSlideModal, CustomTemplateKind,
};
use crate::views::quick_edit_modal::ModalClosed;

const PREVIEW_W: f32 = 384.0;
const PREVIEW_H: f32 = 216.0;

pub(crate) fn render_custom_slide_modal(
    modal: &CustomSlideModal,
    cx: &mut Context<CustomSlideModal>,
) -> Div {
    let preview_spec = modal.build_spec(cx);
    let store_ref = modal.store.read(cx);
    let active_theme = store_ref.active_theme();
    let now_ms = store_ref.now_epoch_ms();

    let preview_slide = Slide {
        id: preview_spec.id,
        source_section_id: None,
        label: preview_spec.label.clone(),
        content: SlideContent::Custom {
            revealed_steps: None,
            layout: preview_spec.layout.clone(),
        },
        transition_override: Some(modal.transition),
    };
    let preview_state = AudienceRenderState::Slide {
        slide: preview_slide,
        theme: Arc::clone(&active_theme),
        show_text: true,
        alert: None,
        audience_timer: None,
        transition_ms: modal.transition.duration_ms(),
        active_transition: modal.transition,
        transition_direction: TransitionDirection::Forward,
    };

    let mut template_bar = div().flex().flex_wrap().gap(px(6.0));
    for tpl in CustomTemplateKind::ALL {
        let selected = modal.template == tpl;
        template_bar = template_bar.child(
            neutral_option_chip(
                format!("custom-tpl-{tpl:?}"),
                tpl.label(),
                selected,
                TOKENS.signal_preview,
            )
            .on_click(cx.listener(move |this, _, _, cx| {
                this.select_template(tpl, cx);
            })),
        );
    }

    let mut transition_bar = div().flex().flex_wrap().gap(px(6.0));
    for tr in SlideTransition::ALL_PRESETS {
        let selected = modal.transition == tr;
        transition_bar = transition_bar.child(
            neutral_option_chip(
                format!("custom-tr-{}", tr.css_kind()),
                tr.label(),
                selected,
                TOKENS.signal_voice,
            )
            .on_click(cx.listener(move |this, _, _, cx| {
                this.transition = tr;
                cx.notify();
            })),
        );
    }

    let mut loop_bar = div().flex().flex_wrap().gap(px(6.0));
    for &(secs, label) in CUSTOM_LOOP_OPTIONS {
        let selected = modal.auto_advance_secs == secs;
        loop_bar = loop_bar.child(
            neutral_option_chip(
                format!("custom-loop-{label}"),
                label,
                selected,
                TOKENS.signal_preview,
            )
            .on_click(cx.listener(move |this, _, _, cx| {
                this.auto_advance_secs = secs;
                cx.notify();
            })),
        );
    }

    let (primary_label, secondary_label) = match modal.template {
        CustomTemplateKind::TitleSubtitle => ("Title", "Subtitle"),
        CustomTemplateKind::BigStatement => ("Key Statement / Quote", "Attribution (Optional)"),
        CustomTemplateKind::BulletPoints => ("Heading", "Bullet Points (one per line)"),
        CustomTemplateKind::ImageTextSplit => ("Heading", "Announcement Copy"),
        CustomTemplateKind::FullBleedImage => ("Lower Caption (Optional)", "Notes"),
        CustomTemplateKind::CountdownSlide => ("Countdown Heading", "Subtitle (Optional)"),
    };

    let mut form = div()
        .flex_1()
        .min_w_0()
        .flex()
        .flex_col()
        .gap(px(10.0))
        .child(labeled_field("Slide Name", modal.label_input.clone()))
        .child(labeled_field(primary_label, modal.primary_input.clone()));

    if modal.template != CustomTemplateKind::FullBleedImage {
        form = form.child(labeled_field(
            secondary_label,
            modal.secondary_input.clone(),
        ));
    }

    // Template-specific clean option row
    match modal.template {
        CustomTemplateKind::TitleSubtitle | CustomTemplateKind::BigStatement => {
            let mut align_row = div().flex().items_center().gap(px(6.0));
            for (al, lbl) in [
                (HorizontalAlign::Left, "Left"),
                (HorizontalAlign::Center, "Center"),
                (HorizontalAlign::Right, "Right"),
            ] {
                align_row = align_row.child(
                    neutral_option_chip(
                        format!("custom-align-{lbl}"),
                        lbl,
                        modal.align == al,
                        TOKENS.signal_preview,
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.align = al;
                        cx.notify();
                    })),
                );
            }
            form = form.child(labeled_group("Alignment", align_row));
        }
        CustomTemplateKind::BulletPoints => {
            form = form.child(
                neutral_option_chip(
                    "custom-progressive-toggle",
                    if modal.progressive_reveal {
                        "Click-to-reveal bullets one by one: On"
                    } else {
                        "Click-to-reveal bullets one by one: Off (Show all at once)"
                    },
                    modal.progressive_reveal,
                    TOKENS.signal_voice,
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    this.progressive_reveal = !this.progressive_reveal;
                    cx.notify();
                })),
            );
        }
        CustomTemplateKind::ImageTextSplit | CustomTemplateKind::FullBleedImage => {
            let mut img_row = div().flex().flex_wrap().items_center().gap(px(6.0));
            for &(uri, name) in BUNDLED_SPLIT_IMAGES {
                let selected = modal.image_uri.as_deref() == Some(uri);
                let u = uri.to_owned();
                img_row = img_row.child(
                    neutral_option_chip(
                        format!("custom-img-{name}"),
                        name,
                        selected,
                        TOKENS.signal_preview,
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.image_uri = Some(u.clone());
                        cx.notify();
                    })),
                );
            }
            img_row = img_row.child(
                div()
                    .id("custom-browse-image")
                    .cursor_pointer()
                    .on_click(cx.listener(|this, _, _, cx| this.browse_image(cx)))
                    .child(sculpted_secondary_button_body(
                        Some(DeckIcon::Image),
                        "Choose Image...",
                        TOKENS.ink_primary,
                    )),
            );
            if modal.template == CustomTemplateKind::ImageTextSplit {
                img_row = img_row.child(
                    neutral_option_chip(
                        "custom-split-side",
                        if modal.image_on_left {
                            "Image Left"
                        } else {
                            "Image Right"
                        },
                        true,
                        TOKENS.signal_voice,
                    )
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.image_on_left = !this.image_on_left;
                        cx.notify();
                    })),
                );
            }
            form = form.child(labeled_group("Slide Photo / Graphic", img_row));
        }
        CustomTemplateKind::CountdownSlide => {
            let mut mins_row = div().flex().items_center().gap(px(6.0));
            for m in [3u32, 5, 10, 15] {
                mins_row = mins_row.child(
                    neutral_option_chip(
                        format!("custom-cd-{m}"),
                        format!("{m} min"),
                        modal.countdown_mins == m,
                        TOKENS.signal_preview,
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.countdown_mins = m;
                        cx.notify();
                    })),
                );
            }
            form = form.child(labeled_group("Countdown Duration", mins_row));
        }
    }

    let right_preview = div()
        .w(px(PREVIEW_W + 2.0))
        .flex_shrink_0()
        .flex()
        .flex_col()
        .gap(px(12.0))
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(status_dot_label(
                    "Live 16:9 Preview",
                    TOKENS.signal_preview,
                    TOKENS.ink_secondary,
                ))
                .child(
                    div()
                        .font_family(FONT_BODY)
                        .text_color(TOKENS.ink_muted)
                        .text_size(px(11.0))
                        .child(active_theme.name.clone()),
                ),
        )
        .child(
            div()
                .w(px(PREVIEW_W))
                .h(px(PREVIEW_H))
                .rounded(px(RADIUS_TILE))
                .overflow_hidden()
                .border_1()
                .border_color(TOKENS.focus_border)
                .bg(rgb(0x0E0F12))
                .child(render_audience_canvas(
                    &preview_state,
                    PREVIEW_W,
                    now_ms,
                    cx,
                )),
        )
        .child(labeled_group("Slide Transition Motion", transition_bar))
        .child(labeled_group("Auto-Advance (Optional)", loop_bar));

    div()
        .w(px(900.0))
        .max_h(px(710.0))
        .rounded(px(RADIUS_MODAL))
        .bg(TOKENS.pane)
        .border_1()
        .border_color(TOKENS.gridline)
        .overflow_hidden()
        .flex()
        .flex_col()
        .child(
            div()
                .w_full()
                .px(px(18.0))
                .py(px(13.0))
                .border_b_1()
                .border_color(TOKENS.hairline)
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .child(deck_icon(
                            DeckIcon::Stack,
                            IconSize::Md,
                            TOKENS.signal_preview,
                        ))
                        .child(
                            div()
                                .font_family(FONT_DISPLAY)
                                .text_color(TOKENS.ink_primary)
                                .text_size(px(15.0))
                                .font_weight(FontWeight::SEMIBOLD)
                                .child("Custom Slide Builder"),
                        ),
                )
                .child(
                    div()
                        .id("custom-slide-close")
                        .w(px(28.0))
                        .h(px(28.0))
                        .rounded(px(RADIUS_CONTROL))
                        .cursor_pointer()
                        .hover(|s| s.bg(TOKENS.elevated_hover))
                        .flex()
                        .items_center()
                        .justify_center()
                        .on_click(cx.listener(|_, _, _, cx| cx.emit(ModalClosed)))
                        .child(deck_icon(DeckIcon::X, IconSize::Sm, TOKENS.ink_secondary)),
                ),
        )
        .child(
            div()
                .w_full()
                .px(px(18.0))
                .py(px(10.0))
                .bg(TOKENS.recessed)
                .border_b_1()
                .border_color(TOKENS.hairline)
                .child(template_bar),
        )
        .child(
            div()
                .id("custom-slide-body")
                .flex_1()
                .overflow_y_scroll()
                .p(px(18.0))
                .flex()
                .flex_row()
                .gap(px(18.0))
                .child(form)
                .child(right_preview),
        )
        .child(
            div()
                .w_full()
                .px(px(18.0))
                .py(px(12.0))
                .bg(TOKENS.recessed)
                .border_t_1()
                .border_color(TOKENS.hairline)
                .flex()
                .items_center()
                .justify_between()
                .child(if let Some(err) = &modal.error_message {
                    status_dot_label(err, TOKENS.signal_live_dot, TOKENS.ink_primary)
                } else {
                    div()
                        .font_family(FONT_BODY)
                        .text_color(TOKENS.ink_muted)
                        .text_size(px(11.5))
                        .child("Inherits active sanctuary theme typography & safe margins")
                })
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .child(
                            div()
                                .id("custom-slide-preview-btn")
                                .cursor_pointer()
                                .on_click(cx.listener(|this, _, _, cx| this.save(false, cx)))
                                .child(sculpted_secondary_button_body(
                                    Some(DeckIcon::PlayFill),
                                    "Save & Preview",
                                    TOKENS.ink_primary,
                                )),
                        )
                        .child(
                            div()
                                .id("custom-slide-add-service-btn")
                                .cursor_pointer()
                                .on_click(cx.listener(|this, _, _, cx| this.save(true, cx)))
                                .child(primary_ivory_button_body(
                                    Some(DeckIcon::Plus),
                                    "Add to Service Order",
                                    true,
                                )),
                        ),
                ),
        )
}

fn labeled_field(label: &str, input: impl gpui::IntoElement) -> Div {
    div()
        .w_full()
        .flex()
        .flex_col()
        .gap(px(5.0))
        .child(
            div()
                .font_family(FONT_BODY)
                .text_color(TOKENS.ink_secondary)
                .text_size(px(11.5))
                .font_weight(FontWeight::MEDIUM)
                .child(label.to_owned()),
        )
        .child(input)
}

fn labeled_group(label: &str, body: Div) -> Div {
    div()
        .w_full()
        .flex()
        .flex_col()
        .gap(px(6.0))
        .child(
            div()
                .font_family(FONT_BODY)
                .text_color(TOKENS.ink_secondary)
                .text_size(px(11.5))
                .font_weight(FontWeight::MEDIUM)
                .child(label.to_owned()),
        )
        .child(body)
}
