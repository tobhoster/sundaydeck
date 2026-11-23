use gpui::{
    Context, Div, FontWeight, InteractiveElement, IntoElement, ParentElement, Render,
    StatefulInteractiveElement, Styled, Window, div, px, rgba,
};

use crate::design::gallery_console::{
    render_color_swatches, render_operator_typography, render_slide_tile_showcase,
};
use crate::design::gallery_projection::{
    render_icon_catalog, render_projection_font_pack, render_switcher_and_voice_showcase,
};
use crate::design::icons::{DeckIcon, IconSize, deck_icon};
use crate::design::motion::MotionMode;
use crate::design::primitives::keycap;
use crate::design::tokens::{
    FONT_BODY, FONT_DISPLAY, FONT_MONO, RADIUS_CONTROL, TOKENS, TOP_BAR_HEIGHT,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GalleryPage {
    #[default]
    ConsoleAndControls,
    ProjectionAndIcons,
}

/// Runnable Kitchen-Sink / Storybook debug view (`sundaydeck --ui-gallery`)
/// showcasing every design token, bundled OFL font, 16:9 slide tile, hardware switcher key,
/// voice-sync telemetry pill, and vendored Phosphor SVG icon.
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

        let body: Div = match page {
            GalleryPage::ConsoleAndControls => div()
                .w_full()
                .p(px(16.0))
                .flex()
                .flex_col()
                .gap(px(16.0))
                .child(render_color_swatches())
                .child(render_operator_typography())
                .child(render_slide_tile_showcase())
                .child(render_switcher_and_voice_showcase(motion)),
            GalleryPage::ProjectionAndIcons => div()
                .w_full()
                .p(px(16.0))
                .flex()
                .flex_col()
                .gap(px(16.0))
                .child(render_projection_font_pack())
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
            .h(px(24.0))
            .px(px(10.0))
            .rounded(px(RADIUS_CONTROL))
            .cursor_pointer()
            .bg(if active {
                TOKENS.elevated
            } else {
                rgba(0x0000_0000)
            })
            .border_1()
            .border_color(if active {
                TOKENS.focus_border
            } else {
                rgba(0x0000_0000)
            })
            .flex()
            .items_center()
            .font_family(FONT_MONO)
            .text_color(if active {
                TOKENS.ink_primary
            } else {
                TOKENS.ink_secondary
            })
            .text_size(px(10.5))
            .font_weight(FontWeight::BOLD)
            .on_click(cx.listener(move |this, _, _, cx| {
                this.active_page = target;
                cx.notify();
            }))
            .child(label)
    };

    let mut motion_segs = div()
        .flex()
        .items_center()
        .bg(TOKENS.canvas)
        .border_1()
        .border_color(TOKENS.gridline)
        .rounded(px(RADIUS_CONTROL));

    for opt in MotionMode::ALL {
        let active = opt == motion;
        motion_segs = motion_segs.child(
            div()
                .id(format!("gallery-motion-{}", opt.label()))
                .px(px(7.0))
                .py(px(3.0))
                .cursor_pointer()
                .bg(if active {
                    TOKENS.elevated
                } else {
                    rgba(0x0000_0000)
                })
                .font_family(FONT_MONO)
                .text_color(if active {
                    TOKENS.signal_preview
                } else {
                    TOKENS.ink_muted
                })
                .text_size(px(9.5))
                .font_weight(FontWeight::BOLD)
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.motion_mode = opt;
                    cx.notify();
                }))
                .child(opt.label()),
        );
    }

    div()
        .w_full()
        .h(px(TOP_BAR_HEIGHT))
        .px(px(14.0))
        .bg(TOKENS.pane)
        .border_b_1()
        .border_color(TOKENS.gridline)
        .flex()
        .items_center()
        .justify_between()
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(10.0))
                .child(deck_icon(
                    DeckIcon::SlidersHorizontal,
                    IconSize::Md,
                    TOKENS.signal_preview,
                ))
                .child(
                    div()
                        .font_family(FONT_DISPLAY)
                        .text_color(TOKENS.ink_primary)
                        .text_size(px(13.0))
                        .font_weight(FontWeight::BOLD)
                        .child("SUNDAYDECK"),
                )
                .child(
                    div()
                        .font_family(FONT_MONO)
                        .text_color(TOKENS.ink_muted)
                        .text_size(px(10.5))
                        .child("// UI-GALLERY"),
                )
                .child(div().w(px(1.0)).h(px(16.0)).bg(TOKENS.gridline))
                .child(page_tab(
                    "gal-tab-console",
                    "01 CONSOLE & SWITCHER",
                    GalleryPage::ConsoleAndControls,
                    cx,
                ))
                .child(page_tab(
                    "gal-tab-icons",
                    "02 PROJECTION & ICONS (49)",
                    GalleryPage::ProjectionAndIcons,
                    cx,
                )),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(8.0))
                .child(
                    div()
                        .font_family(FONT_MONO)
                        .text_color(TOKENS.ink_muted)
                        .text_size(px(10.0))
                        .child("MOTION"),
                )
                .child(motion_segs)
                .child(keycap("--ui-gallery", false)),
        )
}
