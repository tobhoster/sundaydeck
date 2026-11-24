use gpui::{
    Context, Div, FontWeight, InteractiveElement, ObjectFit, ParentElement,
    StatefulInteractiveElement, Styled, StyledImage, div, img, px, rgb, rgba,
};

use crate::design::icons::{DeckIcon, IconSize, deck_icon};
use crate::design::primitives::{
    neutral_option_chip, primary_ivory_button_body, sculpted_secondary_button_body,
    status_dot_label,
};
use crate::design::tokens::{
    FONT_BODY, FONT_DISPLAY, RADIUS_CONTROL, RADIUS_MODAL, RADIUS_TILE, TOKENS,
};
use crate::media::load_background_image;
use crate::views::background_picker_modal::{
    BackgroundPickerModal, BackgroundTab, FONT_CHOICES, SCRIM_CHOICES, UNSPLASH_PRESETS,
};
use crate::views::quick_edit_modal::ModalClosed;

pub(crate) fn render_background_picker_modal(
    modal: &BackgroundPickerModal,
    cx: &mut Context<BackgroundPickerModal>,
) -> Div {
    let tab_btn = |id: &'static str,
                   icon: DeckIcon,
                   label: &'static str,
                   tab: BackgroundTab,
                   cx: &mut Context<BackgroundPickerModal>| {
        let active = modal.active_tab == tab;
        div()
            .id(id)
            .px(px(11.0))
            .py(px(5.0))
            .rounded(px(RADIUS_CONTROL))
            .cursor_pointer()
            .bg(if active {
                TOKENS.elevated
            } else {
                rgba(0x0000_0000)
            })
            .flex()
            .items_center()
            .gap(px(6.0))
            .on_click(cx.listener(move |this, _, _, cx| {
                this.active_tab = tab;
                this.error_message = None;
                cx.notify();
            }))
            .child(deck_icon(
                icon,
                IconSize::Sm,
                if active {
                    TOKENS.ink_primary
                } else {
                    TOKENS.ink_muted
                },
            ))
            .child(
                div()
                    .font_family(FONT_BODY)
                    .text_color(if active {
                        TOKENS.ink_primary
                    } else {
                        TOKENS.ink_secondary
                    })
                    .text_size(px(12.0))
                    .font_weight(FontWeight::MEDIUM)
                    .child(label),
            )
    };

    let body = match modal.active_tab {
        BackgroundTab::UnsplashGallery => render_unsplash_gallery_body(modal, cx),
        BackgroundTab::LocalMedia => render_local_media_body(modal, cx),
    };

    let mut root = div()
        .w(px(900.0))
        .rounded(px(RADIUS_MODAL))
        .bg(TOKENS.pane)
        .border_1()
        .border_color(TOKENS.focus_border)
        .p(px(20.0))
        .flex()
        .flex_col()
        .gap(px(14.0))
        .child(
            div()
                .pb(px(12.0))
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
                        .child(deck_icon(DeckIcon::Image, IconSize::Md, TOKENS.ink_primary))
                        .child(
                            div()
                                .font_family(FONT_DISPLAY)
                                .text_color(TOKENS.ink_primary)
                                .text_size(px(16.0))
                                .font_weight(FontWeight::SEMIBOLD)
                                .child("Add Sanctuary Background"),
                        )
                        .child(div().w(px(1.0)).h(px(16.0)).bg(TOKENS.hairline))
                        .child(tab_btn(
                            "bg-tab-unsplash",
                            DeckIcon::Palette,
                            "Unsplash Worship",
                            BackgroundTab::UnsplashGallery,
                            cx,
                        ))
                        .child(tab_btn(
                            "bg-tab-local",
                            DeckIcon::UploadSimple,
                            "Local Video / Photo",
                            BackgroundTab::LocalMedia,
                            cx,
                        )),
                )
                .child(
                    div()
                        .id("bg-modal-close")
                        .w(px(30.0))
                        .h(px(30.0))
                        .rounded(px(RADIUS_CONTROL))
                        .cursor_pointer()
                        .bg(TOKENS.status_surface)
                        .hover(|s| s.bg(TOKENS.elevated_hover))
                        .border_1()
                        .border_color(TOKENS.gridline)
                        .flex()
                        .items_center()
                        .justify_center()
                        .on_click(cx.listener(|_, _, _, cx| cx.emit(ModalClosed)))
                        .child(deck_icon(DeckIcon::X, IconSize::Sm, TOKENS.ink_secondary)),
                ),
        )
        .child(body)
        .child(render_style_options_bar(modal, cx));

    if let Some(err) = &modal.error_message {
        root = root.child(
            div()
                .px(px(12.0))
                .py(px(8.0))
                .rounded(px(RADIUS_CONTROL))
                .bg(TOKENS.recessed)
                .border_1()
                .border_color(TOKENS.signal_live)
                .font_family(FONT_BODY)
                .text_color(TOKENS.ink_primary)
                .text_size(px(12.0))
                .child(err.clone()),
        );
    }

    root.child(
        div()
            .pt(px(12.0))
            .border_t_1()
            .border_color(TOKENS.hairline)
            .flex()
            .items_center()
            .justify_between()
            .child(
                div()
                    .font_family(FONT_BODY)
                    .text_color(TOKENS.ink_muted)
                    .text_size(px(11.5))
                    .child(
                        "Downloaded photos and local videos are saved offline for Sunday morning.",
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(
                        div()
                            .id("bg-cancel-btn")
                            .cursor_pointer()
                            .on_click(cx.listener(|_, _, _, cx| cx.emit(ModalClosed)))
                            .child(sculpted_secondary_button_body(
                                None,
                                "Cancel",
                                TOKENS.ink_secondary,
                            )),
                    )
                    .child(
                        div()
                            .id("bg-apply-btn")
                            .cursor_pointer()
                            .on_click(cx.listener(|this, _, _, cx| this.create_and_apply(cx)))
                            .child(primary_ivory_button_body(
                                Some(DeckIcon::Check),
                                if modal.busy {
                                    "Downloading..."
                                } else {
                                    "Save & Apply Background"
                                },
                                !modal.busy,
                            )),
                    ),
            ),
    )
}

fn render_unsplash_gallery_body(
    modal: &BackgroundPickerModal,
    cx: &mut Context<BackgroundPickerModal>,
) -> Div {
    let mut grid = div().w_full().flex().flex_wrap().gap(px(10.0));
    for (idx, preset) in UNSPLASH_PRESETS.iter().enumerate() {
        let selected = idx == modal.selected_preset_idx;
        let is_motion = preset.slug == "golden-highlands";
        let mut preview = div()
            .relative()
            .w_full()
            .h(px(76.0))
            .rounded(px(RADIUS_CONTROL))
            .overflow_hidden()
            .bg(match idx {
                3 => rgb(0x161B26),
                4 => rgb(0x1E221A),
                5 => rgb(0x261C15),
                _ => TOKENS.slide_card,
            })
            .border_1()
            .border_color(TOKENS.gridline)
            .flex()
            .items_center()
            .justify_center();

        if let Some(embedded) = preset.embedded_preview
            && let Some(render_img) = load_background_image(embedded)
        {
            preview = preview
                .child(
                    img(render_img)
                        .absolute()
                        .inset_0()
                        .size_full()
                        .object_fit(ObjectFit::Cover),
                )
                .child(div().absolute().inset_0().bg(rgba(0x0B0C_0E66)));
        } else {
            preview = preview.child(deck_icon(DeckIcon::Image, IconSize::Lg, TOKENS.ink_muted));
        }

        grid = grid.child(
            div()
                .id(format!("unsplash-preset-{idx}"))
                .w(px(278.0))
                .p(px(9.0))
                .rounded(px(RADIUS_TILE))
                .cursor_pointer()
                .bg(if selected {
                    TOKENS.elevated
                } else {
                    TOKENS.status_surface
                })
                .hover(|s| s.bg(TOKENS.elevated_hover))
                .border(px(if selected { 1.5 } else { 1.0 }))
                .border_color(if selected {
                    TOKENS.signal_preview
                } else {
                    TOKENS.gridline
                })
                .flex()
                .flex_col()
                .gap(px(6.0))
                .on_click(cx.listener(move |this, _, _, cx| this.select_preset(idx, cx)))
                .child(preview)
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(
                            div()
                                .font_family(FONT_DISPLAY)
                                .text_color(TOKENS.ink_primary)
                                .text_size(px(12.5))
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(preset.title),
                        )
                        .child(status_dot_label(
                            if is_motion { "Motion" } else { "Unsplash" },
                            if is_motion {
                                TOKENS.signal_preview
                            } else {
                                rgb(0x6B9BF2)
                            },
                            TOKENS.ink_secondary,
                        )),
                )
                .child(
                    div()
                        .font_family(FONT_BODY)
                        .text_color(TOKENS.ink_muted)
                        .text_size(px(10.5))
                        .child(preset.subtitle),
                ),
        );
    }

    div()
        .w_full()
        .flex()
        .flex_col()
        .gap(px(10.0))
        .child(grid)
        .child(
            div()
                .w_full()
                .flex()
                .items_center()
                .gap(px(8.0))
                .child(div().w(px(230.0)).child(modal.theme_name_input.clone()))
                .child(div().flex_1().child(modal.url_input.clone())),
        )
}

fn render_local_media_body(
    modal: &BackgroundPickerModal,
    cx: &mut Context<BackgroundPickerModal>,
) -> Div {
    div()
        .w_full()
        .p(px(16.0))
        .rounded(px(RADIUS_TILE))
        .bg(TOKENS.canvas)
        .border_1()
        .border_color(TOKENS.hairline)
        .flex()
        .flex_col()
        .gap(px(12.0))
        .child(
            div()
                .font_family(FONT_BODY)
                .text_color(TOKENS.ink_secondary)
                .text_size(px(12.5))
                .child(
                    "Import any local worship motion loop (.mp4, .mov, .webm) or custom photo (.jpg, .png, .webp). Looping videos automatically extract GPU-crossfaded frames via ffmpeg.",
                ),
        )
        .child(
            div()
                .w_full()
                .flex()
                .items_center()
                .gap(px(8.0))
                .child(
                    div()
                        .id("bg-browse-local-btn")
                        .cursor_pointer()
                        .on_click(cx.listener(|this, _, _, cx| this.browse_local_media(cx)))
                        .child(primary_ivory_button_body(
                            Some(DeckIcon::UploadSimple),
                            "Choose Video / Photo",
                            true,
                        )),
                )
                .child(div().flex_1().child(modal.file_path_input.clone())),
        )
        .child(
            div()
                .w_full()
                .flex()
                .items_center()
                .gap(px(8.0))
                .child(
                    div()
                        .font_family(FONT_BODY)
                        .text_color(TOKENS.ink_muted)
                        .text_size(px(11.5))
                        .child("Theme name:"),
                )
                .child(div().w(px(320.0)).child(modal.theme_name_input.clone())),
        )
}

fn render_style_options_bar(
    modal: &BackgroundPickerModal,
    cx: &mut Context<BackgroundPickerModal>,
) -> Div {
    let mut font_row = div().flex().items_center().gap(px(6.0));
    for &(family, label) in FONT_CHOICES {
        let fam_owned = family.to_owned();
        font_row = font_row.child(
            neutral_option_chip(
                format!("bg-font-{family}"),
                label,
                modal.selected_font == family,
                TOKENS.signal_preview,
            )
            .on_click(cx.listener(move |this, _, _, cx| {
                this.selected_font = fam_owned.clone();
                cx.notify();
            })),
        );
    }

    let mut scrim_row = div().flex().items_center().gap(px(6.0));
    for &(label, val) in SCRIM_CHOICES {
        let active = (modal.scrim_opacity - val).abs() < 0.03;
        scrim_row = scrim_row.child(
            neutral_option_chip(format!("bg-scrim-{label}"), label, active, TOKENS.ivory_bg)
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.scrim_opacity = val;
                    cx.notify();
                })),
        );
    }

    div()
        .w_full()
        .px(px(12.0))
        .py(px(9.0))
        .rounded(px(RADIUS_TILE))
        .bg(TOKENS.canvas)
        .border_1()
        .border_color(TOKENS.hairline)
        .flex()
        .items_center()
        .justify_between()
        .gap(px(12.0))
        .child(font_row)
        .child(scrim_row)
}
