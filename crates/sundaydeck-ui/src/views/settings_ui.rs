use gpui::prelude::FluentBuilder as _;
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
use crate::media_cache::background_image;
use crate::media_decode::{DEFAULT_CHURCH_LOGO_URI, ImageTier};
use crate::views::quick_edit_modal::ModalClosed;
use crate::views::settings_modal::{MOTION_CHOICES, SettingsModal, SettingsTab};
use crate::views::settings_tabs_ui::{render_ai_voice_tab, render_bibles_bundles_tab};

pub(crate) fn render_settings_modal(modal: &SettingsModal, cx: &mut Context<SettingsModal>) -> Div {
    let tab_btn = |id: &'static str,
                   icon: DeckIcon,
                   label: &'static str,
                   tab: SettingsTab,
                   cx: &mut Context<SettingsModal>| {
        let (bg, icon_fg, text_fg) = if modal.active_tab == tab {
            (TOKENS.elevated, TOKENS.ink_primary, TOKENS.ink_primary)
        } else {
            (rgba(0x0000_0000), TOKENS.ink_muted, TOKENS.ink_secondary)
        };
        div()
            .id(id)
            .px(px(11.0))
            .py(px(5.5))
            .rounded(px(RADIUS_CONTROL))
            .cursor_pointer()
            .bg(bg)
            .hover(|s| s.bg(TOKENS.elevated_hover))
            .flex()
            .items_center()
            .gap(px(6.0))
            .on_click(cx.listener(move |this, _, _, cx| {
                this.active_tab = tab;
                this.error_message = None;
                cx.notify();
            }))
            .child(deck_icon(icon, IconSize::Sm, icon_fg))
            .child(
                div()
                    .font_family(FONT_BODY)
                    .text_color(text_fg)
                    .text_size(px(12.0))
                    .font_weight(FontWeight::MEDIUM)
                    .child(label),
            )
    };

    let body = match modal.active_tab {
        SettingsTab::ChurchDisplay => render_church_display_tab(modal, cx),
        SettingsTab::AiVoice => render_ai_voice_tab(modal, cx),
        SettingsTab::BiblesBundles => render_bibles_bundles_tab(modal, cx),
    };

    let footer_feedback = if let Some(err) = &modal.error_message {
        status_dot_label(err, TOKENS.signal_live_dot, TOKENS.ink_primary)
    } else if let Some(note) = &modal.status_note {
        status_dot_label(note, TOKENS.signal_voice, TOKENS.ink_secondary)
    } else if modal.busy {
        status_dot_label(
            "Importing offline data...",
            TOKENS.signal_preview,
            TOKENS.ink_secondary,
        )
    } else {
        div()
            .font_family(FONT_BODY)
            .text_color(TOKENS.ink_muted)
            .text_size(px(11.5))
            .child("100% offline local storage · Changes apply immediately on save")
    };

    div()
        .w(px(790.0))
        .max_h(px(720.0))
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
                .py(px(14.0))
                .border_b_1()
                .border_color(TOKENS.hairline)
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(9.0))
                        .child(deck_icon(
                            DeckIcon::GearSix,
                            IconSize::Md,
                            TOKENS.ink_secondary,
                        ))
                        .child(
                            div()
                                .font_family(FONT_DISPLAY)
                                .text_color(TOKENS.ink_primary)
                                .text_size(px(15.0))
                                .font_weight(FontWeight::SEMIBOLD)
                                .child("SundayDeck Settings"),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .child(modal.render_update_action(cx))
                        .child(
                            div()
                                .id("settings-close")
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
                .flex()
                .items_center()
                .gap(px(6.0))
                .child(tab_btn(
                    "settings-tab-church",
                    DeckIcon::Image,
                    "Church & Display",
                    SettingsTab::ChurchDisplay,
                    cx,
                ))
                .child(tab_btn(
                    "settings-tab-ai",
                    DeckIcon::Microphone,
                    "Planner & Voice",
                    SettingsTab::AiVoice,
                    cx,
                ))
                .child(tab_btn(
                    "settings-tab-bibles",
                    DeckIcon::Stack,
                    "Bibles & Packs",
                    SettingsTab::BiblesBundles,
                    cx,
                )),
        )
        .child(
            div()
                .id("settings-modal-body")
                .flex_1()
                .overflow_y_scroll()
                .p(px(15.0))
                .child(body),
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
                .gap(px(12.0))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .overflow_hidden()
                        .child(footer_feedback),
                )
                .child(
                    div()
                        .flex_shrink_0()
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .child(
                            div()
                                .id("settings-cancel")
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
                                .id("settings-save")
                                .cursor_pointer()
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.save_and_close(cx);
                                }))
                                .child(primary_ivory_button_body(
                                    Some(DeckIcon::Check),
                                    "Save Settings",
                                    true,
                                )),
                        ),
                ),
        )
}

fn render_church_display_tab(modal: &SettingsModal, cx: &mut Context<SettingsModal>) -> Div {
    let store = modal.store.read(cx);
    let services = store.services.clone();
    let active_service_id = store.service.id;

    let logo_preview = if let Some(img_handle) = modal
        .logo_uri
        .as_deref()
        .and_then(|uri| background_image(uri, ImageTier::Thumbnail, cx))
        .or_else(|| background_image(DEFAULT_CHURCH_LOGO_URI, ImageTier::Thumbnail, cx))
    {
        div()
            .w(px(112.0))
            .h(px(64.0))
            .p(px(4.0))
            .rounded(px(RADIUS_CONTROL))
            .overflow_hidden()
            .border_1()
            .border_color(TOKENS.gridline)
            .bg(rgb(0x0E0F12))
            .flex()
            .items_center()
            .justify_center()
            .child(img(img_handle).size_full().object_fit(ObjectFit::Contain))
    } else {
        div()
            .w(px(112.0))
            .h(px(64.0))
            .rounded(px(RADIUS_CONTROL))
            .border_1()
            .border_color(TOKENS.gridline)
            .bg(TOKENS.recessed)
    };

    let mut motion_row = div().flex().items_center().gap(px(8.0));
    for (key, label) in MOTION_CHOICES {
        let selected = modal.motion_mode == *key;
        let key_str = (*key).to_owned();
        motion_row = motion_row.child(
            neutral_option_chip(
                format!("settings-motion-{key}"),
                *label,
                selected,
                TOKENS.signal_preview,
            )
            .on_click(cx.listener(move |this, _, _, cx| {
                this.motion_mode = key_str.clone();
                cx.notify();
            })),
        );
    }

    let mut service_chips = div().flex().flex_wrap().gap(px(7.0));
    for svc in services {
        let selected = svc.id == active_service_id;
        let svc_id = svc.id;
        let label = format!("{} ({})", svc.title, svc.items.len());
        service_chips = service_chips.child(
            neutral_option_chip(
                format!("settings-svc-{svc_id}"),
                label,
                selected,
                TOKENS.signal_voice,
            )
            .on_click(cx.listener(move |this, _, _, cx| {
                this.switch_service(svc_id, cx);
            })),
        );
    }

    div()
        .w_full()
        .flex()
        .flex_col()
        .gap(px(12.0))
        .child(section_card(
            "Church Branding (Show Logo · F3)",
            div()
                .flex()
                .flex_col()
                .gap(px(12.0))
                .child(field_label(
                    "Church or Ministry Name",
                    modal.church_name_input.clone(),
                ))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(14.0))
                        .child(logo_preview)
                        .child(
                            div()
                                .flex_1()
                                .flex()
                                .flex_col()
                                .gap(px(7.0))
                                .child(
                                    div()
                                        .font_family(FONT_BODY)
                                        .text_color(TOKENS.ink_secondary)
                                        .text_size(px(12.0))
                                        .child(
                                            modal
                                                .logo_uri
                                                .as_deref()
                                                .map(|_| {
                                                    "Custom church logo active for Show Logo (F3)."
                                                })
                                                .unwrap_or(
                                                    "Default church emblem active until you choose your own church logo.",
                                                ),
                                        ),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(px(8.0))
                                        .child(
                                            div()
                                                .id("settings-choose-logo")
                                                .cursor_pointer()
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    this.browse_church_logo(cx);
                                                }))
                                                .child(sculpted_secondary_button_body(
                                                    Some(DeckIcon::Image),
                                                    "Choose Logo Image",
                                                    TOKENS.ink_primary,
                                                )),
                                        )
                                        .when(modal.logo_uri.is_some(), |el| {
                                            el.child(
                                                div()
                                                    .id("settings-clear-logo")
                                                    .cursor_pointer()
                                                    .on_click(cx.listener(|this, _, _, cx| {
                                                        this.logo_uri = None;
                                                        cx.notify();
                                                    }))
                                                    .child(sculpted_secondary_button_body(
                                                        Some(DeckIcon::Trash),
                                                        "Reset Logo",
                                                        TOKENS.ink_secondary,
                                                    )),
                                            )
                                        }),
                                ),
                        ),
                ),
        ))
        .child(section_card(
            "Slide Motion & Live Control",
            div()
                .flex()
                .flex_col()
                .gap(px(12.0))
                .child(motion_row)
                .child(
                    neutral_option_chip(
                        "settings-direct-click",
                        if modal.direct_click_live {
                            "Click slide to show immediately: On"
                        } else {
                            "Click slide to show immediately: Off (Stage in Preview first)"
                        },
                        modal.direct_click_live,
                        TOKENS.signal_preview,
                    )
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.direct_click_live = !this.direct_click_live;
                        cx.notify();
                    })),
                )
                .child(
                    neutral_option_chip(
                        "settings-auto-updates",
                        if modal.auto_check_updates {
                            "Check for updates automatically: On (Silent when offline)"
                        } else {
                            "Check for updates automatically: Off (Manual only)"
                        },
                        modal.auto_check_updates,
                        TOKENS.signal_voice,
                    )
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.auto_check_updates = !this.auto_check_updates;
                        cx.notify();
                    })),
                ),
        ))
        .child(section_card(
            "Saved Services",
            div()
                .flex()
                .flex_col()
                .gap(px(10.0))
                .child(service_chips)
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .child(div().flex_1().child(modal.new_service_input.clone()))
                        .child(
                            div()
                                .id("settings-create-service")
                                .cursor_pointer()
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.create_new_service(cx);
                                }))
                                .child(sculpted_secondary_button_body(
                                    Some(DeckIcon::Plus),
                                    "New Service",
                                    TOKENS.ink_primary,
                                )),
                        ),
                ),
        ))
}

pub(crate) fn section_card(title: &str, body: Div) -> Div {
    div()
        .w_full()
        .p(px(12.0))
        .rounded(px(RADIUS_TILE))
        .bg(TOKENS.slide_card)
        .border_1()
        .border_color(TOKENS.gridline)
        .flex()
        .flex_col()
        .gap(px(8.0))
        .child(
            div()
                .font_family(FONT_DISPLAY)
                .text_color(TOKENS.ink_primary)
                .text_size(px(13.0))
                .font_weight(FontWeight::SEMIBOLD)
                .child(title.to_owned()),
        )
        .child(body)
}

pub(crate) fn field_label(label: &str, input: impl gpui::IntoElement) -> Div {
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
