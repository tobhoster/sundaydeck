use gpui::{
    Context, Div, FontWeight, InteractiveElement, IntoElement, ParentElement, Render,
    StatefulInteractiveElement, Styled, Window, div, px, rgba,
};

use crate::design::gallery_console::{
    render_color_swatches, render_operator_typography, render_slide_tile_showcase,
};
use crate::design::gallery_projection::{
    render_custom_layouts_and_motions, render_icon_catalog, render_projection_font_pack,
    render_switcher_and_voice_showcase,
};
use crate::design::icons::{DeckIcon, IconSize, deck_icon};
use crate::design::motion::MotionMode;
use crate::design::tokens::{
    FONT_BODY, FONT_DISPLAY, RADIUS_CONTROL, RADIUS_PILL, TOKENS, TOP_BAR_HEIGHT,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GalleryPage {
    #[default]
    ConsoleAndControls,
    ProjectionAndIcons,
}

/// Runnable Design Gallery (`sundaydeck --ui-gallery`) showcasing SundayDeck's bespoke native
/// dark-linen palette, sculpted tactile controls, neutral status bar, typographic section labels,
/// 16:9 sanctuary slide cards (with Public Domain photo & motion video themes), and 49 Phosphor SVGs.
pub struct DesignGalleryView {
    pub active_page: GalleryPage,
    pub motion_mode: MotionMode,
}

impl DesignGalleryView {
    pub fn new() -> Self {
        Self {
            active_page: GalleryPage::ConsoleAndControls,
            motion_mode: MotionMode::Full,
        }
    }
}

impl Default for DesignGalleryView {
    fn default() -> Self {
        Self::new()
    }
}

impl Render for DesignGalleryView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let page = self.active_page;
        let motion = self.motion_mode;

        let divider = || div().w_full().h(px(1.0)).bg(TOKENS.hairline);

        let body: Div = match page {
            GalleryPage::ConsoleAndControls => div()
                .w_full()
                .px(px(28.0))
                .py(px(24.0))
                .flex()
                .flex_col()
                .gap(px(24.0))
                .child(render_color_swatches())
                .child(divider())
                .child(render_operator_typography())
                .child(divider())
                .child(render_slide_tile_showcase(cx))
                .child(divider())
                .child(render_switcher_and_voice_showcase(motion)),
            GalleryPage::ProjectionAndIcons => div()
                .w_full()
                .px(px(28.0))
                .py(px(24.0))
                .flex()
                .flex_col()
                .gap(px(24.0))
                .child(render_projection_font_pack(cx))
                .child(divider())
                .child(render_custom_layouts_and_motions())
                .child(divider())
                .child(render_icon_catalog()),
        };

        div()
            .size_full()
            .bg(TOKENS.canvas)
            .font_family(FONT_BODY)
            .text_color(TOKENS.ink_primary)
            .flex()
            .flex_col()
            .overflow_hidden()
            .child(render_gallery_top_bar(page, motion, cx))
            .child(
                div()
                    .id("gallery-scroll-body")
                    .flex_1()
                    .w_full()
                    .overflow_y_scroll()
                    .child(body),
            )
    }
}

fn render_gallery_top_bar(
    page: GalleryPage,
    motion: MotionMode,
    cx: &mut Context<DesignGalleryView>,
) -> Div {
    let page_tab = |id: &'static str,
                    label: &'static str,
                    target: GalleryPage,
                    cx: &mut Context<DesignGalleryView>| {
        let active = page == target;
        div()
            .id(id)
            .h(px(28.0))
            .px(px(12.0))
            .rounded(px(RADIUS_CONTROL))
            .cursor_pointer()
            .bg(if active {
                TOKENS.elevated
            } else {
                rgba(0x0000_0000)
            })
            .flex()
            .items_center()
            .font_family(FONT_BODY)
            .text_color(if active {
                TOKENS.ink_primary
            } else {
                TOKENS.ink_secondary
            })
            .text_size(px(12.0))
            .font_weight(FontWeight::MEDIUM)
            .on_click(cx.listener(move |this, _, _, cx| {
                this.active_page = target;
                cx.notify();
            }))
            .child(label)
    };

    let mut motion_segs = div()
        .flex()
        .items_center()
        .p(px(2.0))
        .bg(TOKENS.recessed)
        .border_1()
        .border_color(TOKENS.gridline)
        .rounded(px(RADIUS_PILL));

    for opt in MotionMode::ALL {
        let active = opt == motion;
        let friendly = match opt {
            MotionMode::Full => "Smooth",
            MotionMode::CrossfadeOnly => "Calm",
            MotionMode::Instant => "Instant",
        };
        motion_segs = motion_segs.child(
            div()
                .id(format!("gallery-motion-{}", opt.label()))
                .px(px(10.0))
                .py(px(3.0))
                .rounded(px(RADIUS_PILL))
                .cursor_pointer()
                .bg(if active {
                    TOKENS.elevated
                } else {
                    rgba(0x0000_0000)
                })
                .font_family(FONT_BODY)
                .text_color(if active {
                    TOKENS.ink_primary
                } else {
                    TOKENS.ink_muted
                })
                .text_size(px(11.0))
                .font_weight(FontWeight::MEDIUM)
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.motion_mode = opt;
                    cx.notify();
                }))
                .child(friendly),
        );
    }

    div()
        .w_full()
        .h(px(TOP_BAR_HEIGHT))
        .px(px(20.0))
        .bg(TOKENS.pane)
        .border_b_1()
        .border_color(TOKENS.hairline)
        .flex()
        .items_center()
        .justify_between()
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(10.0))
                .child(deck_icon(
                    DeckIcon::Palette,
                    IconSize::Md,
                    TOKENS.ink_primary,
                ))
                .child(
                    div()
                        .font_family(FONT_DISPLAY)
                        .text_color(TOKENS.ink_primary)
                        .text_size(px(14.5))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child("SundayDeck"),
                )
                .child(
                    div()
                        .font_family(FONT_BODY)
                        .text_color(TOKENS.ink_muted)
                        .text_size(px(12.5))
                        .child("Design Gallery"),
                )
                .child(div().w(px(1.0)).h(px(16.0)).bg(TOKENS.hairline))
                .child(page_tab(
                    "gal-tab-console",
                    "Studio & Controls",
                    GalleryPage::ConsoleAndControls,
                    cx,
                ))
                .child(page_tab(
                    "gal-tab-icons",
                    "Projection & Icons",
                    GalleryPage::ProjectionAndIcons,
                    cx,
                )),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(10.0))
                .child(
                    div()
                        .font_family(FONT_BODY)
                        .text_color(TOKENS.ink_muted)
                        .text_size(px(11.5))
                        .child("Motion"),
                )
                .child(motion_segs),
        )
}
