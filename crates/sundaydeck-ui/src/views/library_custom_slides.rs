use std::sync::Arc;

use gpui::{
    AnyElement, ClickEvent, Context, Div, FontWeight, InteractiveElement, IntoElement, MouseButton,
    MouseDownEvent, ParentElement, StatefulInteractiveElement, Styled, div, px, rgb,
};
use sundaydeck_core::{
    AudienceRenderState, BackgroundSpec, PresentationCommand, Slide, SlideContent, SlideTransition,
    TransitionDirection,
};

use crate::design::icons::{DeckIcon, IconSize, deck_icon};
use crate::design::primitives::status_dot_label;
use crate::design::tokens::{
    FONT_BODY, FONT_DISPLAY, RADIUS_CONTROL, RADIUS_TILE, TOKENS, rgba8_to_gpui,
};
use crate::media::{BackgroundPaint, apply_theme_background};
use crate::media_decode::ImageTier;
use crate::renderer::render_audience_canvas;
use crate::views::context_menu::ContextMenuTarget;
use crate::views::operator_workspace::OperatorWorkspace;
use crate::views::quick_slides::QuickSlidePreset;

/// Renders the **Custom Slides** tab inside the Center Library Drawer — showing all reusable
/// structured slides (Title, Big Statement, Progressive Bullets, Image+Text, Full-Bleed, Countdown)
/// with live 16:9 WYSIWYG thumbnails, 1-click staging, editing, and adding to Service Order.
pub(crate) fn render_custom_slides_library(
    workspace: &OperatorWorkspace,
    cx: &mut Context<OperatorWorkspace>,
) -> AnyElement {
    let store = workspace.store.read(cx);
    let active_theme = store.active_theme();
    let now_ms = store.now_epoch_ms();

    let mut grid = div()
        .id("custom-slides-library-grid")
        .flex_1()
        .overflow_y_scroll()
        .p(px(10.0))
        .flex()
        .flex_wrap()
        .items_start()
        .content_start()
        .gap(px(10.0));

    for spec in &store.custom_slides {
        let slide_id = spec.id;
        let template_label = spec.layout.template_label();
        let tr = spec
            .transition_override
            .unwrap_or(SlideTransition::DEFAULT_SOFT_RISE);

        let mini_state = AudienceRenderState::Slide {
            slide: Slide {
                id: spec.id,
                source_section_id: None,
                label: spec.label.clone(),
                content: SlideContent::Custom {
                    layout: spec.layout.clone(),
                    revealed_steps: None,
                },
                transition_override: Some(tr),
            },
            theme: Arc::clone(&active_theme),
            show_text: true,
            alert: None,
            audience_timer: None,
            transition_ms: tr.duration_ms(),
            active_transition: tr,
            transition_direction: TransitionDirection::Forward,
        };

        let card = div()
            .id(format!("lib-custom-{slide_id}"))
            .w(px(210.0))
            .p(px(7.0))
            .rounded(px(RADIUS_TILE))
            .cursor_pointer()
            .bg(TOKENS.status_surface)
            .hover(|s| s.bg(TOKENS.elevated_hover))
            .border_1()
            .border_color(TOKENS.gridline)
            .flex()
            .flex_col()
            .gap(px(6.0))
            .on_click(cx.listener(move |this, ev: &ClickEvent, _, cx| {
                this.store.update(cx, |s, cx| {
                    s.stage_custom_slide(slide_id, cx);
                    if ev.click_count() >= 2 && !s.snapshot().direct_click_live {
                        let _ = s.dispatch(PresentationCommand::GoLive, cx);
                    }
                });
            }))
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |this, ev: &MouseDownEvent, _, cx| {
                    this.open_context_menu(
                        ev.position,
                        ContextMenuTarget::CustomSlide(slide_id),
                        cx,
                    );
                }),
            )
            .child(
                div()
                    .w(px(194.0))
                    .h(px(109.0))
                    .rounded(px(RADIUS_CONTROL))
                    .overflow_hidden()
                    .border_1()
                    .border_color(TOKENS.hairline)
                    .bg(rgb(0x0E0F12))
                    .child(render_audience_canvas(&mini_state, 194.0, now_ms, cx)),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(6.0))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .font_family(FONT_DISPLAY)
                            .text_color(TOKENS.ink_primary)
                            .text_size(px(11.5))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(spec.label.clone()),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(4.0))
                            .child(mini_action_btn(
                                format!("edit-custom-{slide_id}"),
                                DeckIcon::PencilSimple,
                                TOKENS.ink_secondary,
                                cx,
                                move |this, cx| {
                                    let spec =
                                        this.store.read(cx).custom_slide_by_id(slide_id).cloned();
                                    if let Some(spec) = spec {
                                        this.open_custom_slide_builder(Some(spec), cx);
                                    }
                                },
                            ))
                            .child(mini_action_btn(
                                format!("add-custom-{slide_id}"),
                                DeckIcon::Plus,
                                TOKENS.ink_primary,
                                cx,
                                move |this, cx| {
                                    let spec =
                                        this.store.read(cx).custom_slide_by_id(slide_id).cloned();
                                    let Some(spec) = spec else {
                                        return;
                                    };
                                    let sched = this.store.update(cx, |s, cx| {
                                        s.upsert_custom_slide(spec, true, None, cx)
                                    });
                                    if let (Some(schedule), Some(db)) = (sched, this.db.clone()) {
                                        cx.spawn(async move |_, _| {
                                            let _ = db.save_service(schedule).await;
                                        })
                                        .detach();
                                    }
                                },
                            )),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(status_dot_label(
                        template_label,
                        TOKENS.signal_preview,
                        TOKENS.ink_secondary,
                    ))
                    .child(
                        div()
                            .font_family(FONT_BODY)
                            .text_color(TOKENS.ink_muted)
                            .text_size(px(10.0))
                            .child(tr.label()),
                    ),
            );

        grid = grid.child(card);
    }

    let quick_bar = div()
        .px(px(12.0))
        .py(px(7.0))
        .border_b_1()
        .border_color(TOKENS.hairline)
        .flex()
        .items_center()
        .gap(px(6.0))
        .child(
            div()
                .font_family(FONT_BODY)
                .text_color(TOKENS.ink_muted)
                .text_size(px(11.0))
                .font_weight(FontWeight::MEDIUM)
                .child("Quick Slide:"),
        )
        .child(quick_preset_chip(
            "quick-slide-pastor-quote",
            DeckIcon::ChatCenteredText,
            "Pastor Quote",
            TOKENS.signal_preview,
            QuickSlidePreset::PastorQuote,
            cx,
        ))
        .child(quick_preset_chip(
            "quick-slide-birthday",
            DeckIcon::BellRinging,
            "Member Birthday",
            TOKENS.signal_voice,
            QuickSlidePreset::MemberBirthday,
            cx,
        ))
        .child(quick_preset_chip(
            "quick-slide-sermon-points",
            DeckIcon::ListNumbers,
            "Sermon Points",
            TOKENS.ink_primary,
            QuickSlidePreset::SermonPoint,
            cx,
        ))
        .child(quick_preset_chip(
            "quick-slide-announcement",
            DeckIcon::Megaphone,
            "Announcement",
            rgb(0x6B9BF2),
            QuickSlidePreset::Announcement,
            cx,
        ));

    div()
        .flex_1()
        .w_full()
        .flex()
        .flex_col()
        .overflow_hidden()
        .child(quick_bar)
        .child(grid)
        .into_any_element()
}

fn quick_preset_chip(
    id: &'static str,
    icon: DeckIcon,
    label: &'static str,
    tint: gpui::Rgba,
    preset: QuickSlidePreset,
    cx: &mut Context<OperatorWorkspace>,
) -> gpui::Stateful<Div> {
    div()
        .id(id)
        .h(px(26.0))
        .px(px(9.0))
        .rounded(px(RADIUS_CONTROL))
        .cursor_pointer()
        .bg(TOKENS.status_surface)
        .hover(|s| s.bg(TOKENS.elevated_hover))
        .border_1()
        .border_color(TOKENS.key_border)
        .flex()
        .items_center()
        .gap(px(5.0))
        .on_click(cx.listener(move |this, _, _, cx| {
            this.open_quick_slide_preset(preset, cx);
        }))
        .child(deck_icon(icon, IconSize::Sm, tint))
        .child(
            div()
                .font_family(FONT_BODY)
                .text_color(TOKENS.ink_primary)
                .text_size(px(11.0))
                .font_weight(FontWeight::MEDIUM)
                .child(label),
        )
}

pub(crate) fn render_themes_library(
    workspace: &OperatorWorkspace,
    cx: &mut Context<OperatorWorkspace>,
) -> AnyElement {
    let store = workspace.store.read(cx);
    let themes = store.themes.clone();
    let active_id = store.active_theme().id;
    let now_ms = store.now_epoch_ms();

    let mut row = div()
        .id("themes-library-grid")
        .flex_1()
        .overflow_y_scroll()
        .p(px(10.0))
        .flex()
        .flex_wrap()
        .items_start()
        .content_start()
        .gap(px(9.0));

    for theme in themes {
        let is_active = theme.id == active_id;
        let theme_click = Arc::clone(&theme);
        let (kind_label, kind_dot) = match &theme.background {
            BackgroundSpec::VideoLoop { .. } => ("Motion", TOKENS.signal_preview),
            BackgroundSpec::Image { .. } => ("Photo", rgb(0x6B9BF2)),
            BackgroundSpec::LiveCamera { .. } => ("Camera", TOKENS.signal_voice),
            BackgroundSpec::LinearGradient { .. } => ("Gradient", TOKENS.ink_muted),
            BackgroundSpec::Solid { color } if color.g() > 150 && color.r() < 40 => {
                ("Chroma", TOKENS.signal_voice)
            }
            BackgroundSpec::Solid { .. } => ("Matte", TOKENS.ink_muted),
        };

        let preview_thumb = apply_theme_background(
            div()
                .relative()
                .w_full()
                .h(px(52.0))
                .rounded(px(RADIUS_CONTROL))
                .overflow_hidden()
                .border_1()
                .border_color(TOKENS.gridline)
                .flex()
                .items_center()
                .justify_center(),
            BackgroundPaint {
                spec: &theme.background,
                now_ms,
                tier: ImageTier::Thumbnail,
            },
            cx,
        )
        .child(
            div()
                .relative()
                .font_family(theme.font_family.clone())
                .text_color(rgba8_to_gpui(theme.text_color))
                .text_size(px(15.0))
                .font_weight(FontWeight::BOLD)
                .child("Aa Bb"),
        );

        let card = div()
            .id(format!("lib-theme-{}", theme.id))
            .w(px(184.0))
            .p(px(7.0))
            .rounded(px(RADIUS_TILE))
            .cursor_pointer()
            .bg(if is_active {
                TOKENS.elevated
            } else {
                TOKENS.status_surface
            })
            .hover(|s| s.bg(TOKENS.elevated_hover))
            .border(px(if is_active { 1.5 } else { 1.0 }))
            .border_color(if is_active {
                TOKENS.signal_preview
            } else {
                TOKENS.gridline
            })
            .flex()
            .flex_col()
            .gap(px(5.0))
            .on_click(cx.listener(move |this, _, _, cx| {
                this.store.update(cx, |s, cx| {
                    let _ = s.dispatch(
                        PresentationCommand::ApplyLiveTheme(Arc::clone(&theme_click)),
                        cx,
                    );
                });
            }))
            .child(preview_thumb)
            .child(
                div()
                    .font_family(FONT_DISPLAY)
                    .text_color(TOKENS.ink_primary)
                    .text_size(px(11.5))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(theme.name.clone()),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(status_dot_label(kind_label, kind_dot, TOKENS.ink_secondary))
                    .child(
                        div()
                            .font_family(FONT_BODY)
                            .text_color(TOKENS.ink_muted)
                            .text_size(px(10.0))
                            .child(theme.transition.label()),
                    ),
            );

        row = row.child(card);
    }
    row.into_any_element()
}

fn mini_action_btn(
    id: String,
    icon: DeckIcon,
    tint: gpui::Rgba,
    cx: &Context<OperatorWorkspace>,
    on_click: impl Fn(&mut OperatorWorkspace, &mut Context<OperatorWorkspace>) + 'static,
) -> gpui::Stateful<Div> {
    div()
        .id(id)
        .w(px(22.0))
        .h(px(22.0))
        .rounded(px(4.0))
        .bg(TOKENS.elevated)
        .hover(|s| s.bg(TOKENS.elevated_hover))
        .border_1()
        .border_color(TOKENS.gridline)
        .flex()
        .items_center()
        .justify_center()
        .on_click(cx.listener(move |this, _, _, cx| on_click(this, cx)))
        .child(deck_icon(icon, IconSize::Sm, tint))
}
