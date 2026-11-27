use gpui::{
    Context, Div, FontWeight, InteractiveElement, IntoElement as _, ParentElement,
    StatefulInteractiveElement, Styled, div, px,
};

use crate::content_downloader::{FREE_BIBLE_CATALOG, is_translation_full_offline};
use crate::design::icons::DeckIcon;
use crate::design::primitives::sculpted_secondary_button_body;
use crate::design::tokens::{FONT_BODY, FONT_MONO, RADIUS_CONTROL, TOKENS};
use crate::views::settings_modal::SettingsModal;
use crate::views::settings_ui::section_card;

/// Renders the 1-click Public Domain / CC0 offline Bible downloader inside Settings.
pub(crate) fn render_free_bible_catalog_section(
    modal: &SettingsModal,
    cx: &mut Context<SettingsModal>,
) -> Div {
    let translations = modal.store.read(cx).bible_translations.clone();
    let mut rows = div().flex().flex_col().gap(px(7.0));

    for entry in FREE_BIBLE_CATALOG {
        let installed = is_translation_full_offline(&translations, entry.code);
        let code = entry.code;
        rows = rows.child(
            div()
                .w_full()
                .px(px(11.0))
                .py(px(8.0))
                .rounded(px(RADIUS_CONTROL))
                .bg(TOKENS.recessed)
                .border_1()
                .border_color(if installed {
                    TOKENS.signal_voice
                } else {
                    TOKENS.gridline
                })
                .flex()
                .items_center()
                .justify_between()
                .gap(px(10.0))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap(px(2.0))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(7.0))
                                .child(
                                    div()
                                        .font_family(FONT_MONO)
                                        .text_color(TOKENS.ink_primary)
                                        .text_size(px(11.0))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .child(entry.code),
                                )
                                .child(
                                    div()
                                        .font_family(FONT_BODY)
                                        .text_color(TOKENS.ink_primary)
                                        .text_size(px(12.0))
                                        .font_weight(FontWeight::MEDIUM)
                                        .child(entry.name),
                                ),
                        )
                        .child(
                            div()
                                .font_family(FONT_BODY)
                                .text_color(TOKENS.ink_muted)
                                .text_size(px(11.0))
                                .child(format!("{} · {}", entry.books_label, entry.license)),
                        ),
                )
                .child(if installed {
                    div()
                        .flex_shrink_0()
                        .px(px(9.0))
                        .py(px(4.0))
                        .rounded(px(RADIUS_CONTROL))
                        .bg(TOKENS.elevated)
                        .font_family(FONT_MONO)
                        .text_color(TOKENS.signal_voice)
                        .text_size(px(10.5))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child("Full Offline")
                        .into_any_element()
                } else {
                    div()
                        .id(format!("settings-dl-bible-{code}"))
                        .flex_shrink_0()
                        .cursor_pointer()
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.download_catalog_bible(code, cx);
                        }))
                        .child(sculpted_secondary_button_body(
                            Some(DeckIcon::BookOpenText),
                            if modal.busy {
                                "Working…"
                            } else {
                                "Download Free"
                            },
                            TOKENS.signal_preview,
                        ))
                        .into_any_element()
                }),
        );
    }

    section_card(
        "Free Full Offline Bibles (Verified Public Domain / CC0)",
        div()
            .flex()
            .flex_col()
            .gap(px(9.0))
            .child(
                div()
                    .font_family(FONT_BODY)
                    .text_color(TOKENS.ink_muted)
                    .text_size(px(11.5))
                    .child("BSB and KJV auto-sync in the background when online. Click any translation below to store all books permanently offline in SQLite."),
            )
            .child(rows),
    )
}
