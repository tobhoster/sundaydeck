use gpui::{
    Context, Div, FontWeight, InteractiveElement, ParentElement, StatefulInteractiveElement,
    Styled, div, px,
};

use crate::design::icons::{DeckIcon, IconSize, deck_icon};
use crate::design::primitives::{neutral_option_chip, sculpted_secondary_button_body};
use crate::design::tokens::{FONT_BODY, FONT_MONO, RADIUS_CONTROL, TOKENS};
use crate::views::settings_modal::{
    BILINGUAL_MODE_CHOICES, CHURCH_PROFILE_CHOICES, LLM_PRESET_CHOICES, LOOP_INTERVAL_CHOICES,
    SettingsModal, VOICE_THRESHOLD_CHOICES,
};
use crate::views::settings_ui::{field_label, section_card};

pub(crate) fn render_ai_voice_tab(modal: &SettingsModal, cx: &mut Context<SettingsModal>) -> Div {
    let mut llm_chips = div().flex().flex_wrap().gap(px(7.0));
    for (id, label) in LLM_PRESET_CHOICES {
        let selected = modal.llm_model_preset == *id;
        let id_str = (*id).to_owned();
        llm_chips = llm_chips.child(
            neutral_option_chip(
                format!("settings-llm-{id}"),
                *label,
                selected,
                TOKENS.signal_preview,
            )
            .on_click(cx.listener(move |this, _, _, cx| {
                this.llm_model_preset = id_str.clone();
                cx.notify();
            })),
        );
    }

    let mut threshold_chips = div().flex().flex_wrap().gap(px(7.0));
    for (pct, label) in VOICE_THRESHOLD_CHOICES {
        let selected = modal.voice_threshold_pct == *pct;
        let p = *pct;
        threshold_chips = threshold_chips.child(
            neutral_option_chip(
                format!("settings-voice-pct-{p}"),
                *label,
                selected,
                TOKENS.signal_voice,
            )
            .on_click(cx.listener(move |this, _, _, cx| {
                this.voice_threshold_pct = p;
                cx.notify();
            })),
        );
    }

    div()
        .w_full()
        .flex()
        .flex_col()
        .gap(px(16.0))
        .child(section_card(
            "Offline Service Plan Assistant",
            div()
                .flex()
                .flex_col()
                .gap(px(11.0))
                .child(field_label(
                    "Local Companion Endpoint",
                    modal.llm_url_input.clone(),
                ))
                .child(llm_chips),
        ))
        .child(section_card(
            "Offline Microphone Auto-Follow",
            div()
                .flex()
                .flex_col()
                .gap(px(11.0))
                .child(field_label(
                    "Local Voice Engine URL",
                    modal.whisper_url_input.clone(),
                ))
                .child(threshold_chips)
                .child(
                    neutral_option_chip(
                        "settings-voice-lock",
                        if modal.voice_lock_during_override {
                            "Pause Auto-Follow during Black Screen / Show Logo: On"
                        } else {
                            "Pause Auto-Follow during Black Screen / Show Logo: Off"
                        },
                        modal.voice_lock_during_override,
                        TOKENS.signal_voice,
                    )
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.voice_lock_during_override = !this.voice_lock_during_override;
                        cx.notify();
                    })),
                ),
        ))
}

pub(crate) fn render_bibles_bundles_tab(
    modal: &SettingsModal,
    cx: &mut Context<SettingsModal>,
) -> Div {
    let store = modal.store.read(cx);
    let translations = store.bible_translations.clone();
    let active_code = store.active_translation_code.clone();

    let mut bible_list = div().flex().flex_wrap().gap(px(7.0));
    for tr in translations {
        let is_active = tr.code.eq_ignore_ascii_case(&active_code);
        bible_list = bible_list.child(
            div()
                .px(px(10.0))
                .py(px(5.0))
                .rounded(px(RADIUS_CONTROL))
                .bg(TOKENS.recessed)
                .border_1()
                .border_color(if is_active {
                    TOKENS.focus_border
                } else {
                    TOKENS.gridline
                })
                .flex()
                .items_center()
                .gap(px(7.0))
                .child(
                    div()
                        .font_family(FONT_MONO)
                        .text_color(TOKENS.ink_primary)
                        .text_size(px(11.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(tr.code),
                )
                .child(
                    div()
                        .font_family(FONT_BODY)
                        .text_color(TOKENS.ink_secondary)
                        .text_size(px(11.5))
                        .child(tr.name),
                ),
        );
    }

    let mut interval_chips = div().flex().items_center().gap(px(6.0));
    for secs in LOOP_INTERVAL_CHOICES {
        let s_val = *secs;
        let selected = modal.loop_interval_secs == s_val;
        interval_chips = interval_chips.child(
            neutral_option_chip(
                format!("settings-loop-sec-{s_val}"),
                format!("{s_val}s"),
                selected,
                TOKENS.signal_preview,
            )
            .on_click(cx.listener(move |this, _, _, cx| {
                this.loop_interval_secs = s_val;
                cx.notify();
            })),
        );
    }

    let mut profile_chips = div().flex().flex_wrap().gap(px(6.0));
    for (id, label) in CHURCH_PROFILE_CHOICES {
        let selected = modal.church_profile == *id;
        let id_str = (*id).to_owned();
        profile_chips = profile_chips.child(
            neutral_option_chip(
                format!("settings-profile-{id}"),
                *label,
                selected,
                TOKENS.signal_preview,
            )
            .on_click(cx.listener(move |this, _, _, cx| {
                this.church_profile = id_str.clone();
                if id_str == "catholic_parish" {
                    this.enable_catholic_canon = true;
                }
                cx.notify();
            })),
        );
    }

    let mut bilingual_chips = div().flex().flex_wrap().gap(px(6.0));
    for (id, label) in BILINGUAL_MODE_CHOICES {
        let selected = modal.bilingual_mode == *id;
        let id_str = (*id).to_owned();
        bilingual_chips = bilingual_chips.child(
            neutral_option_chip(
                format!("settings-bilingual-{id}"),
                *label,
                selected,
                TOKENS.signal_voice,
            )
            .on_click(cx.listener(move |this, _, _, cx| {
                this.bilingual_mode = id_str.clone();
                cx.notify();
            })),
        );
    }

    div()
        .w_full()
        .flex()
        .flex_col()
        .gap(px(14.0))
        .child(section_card(
            "Church Tradition & Bilingual Lyrics",
            div()
                .flex()
                .flex_col()
                .gap(px(9.0))
                .child(profile_chips)
                .child(bilingual_chips),
        ))
        .child(section_card(
            "Offline Bibles (66 & 73 Books) & Content Packs",
            div()
                .flex()
                .flex_col()
                .gap(px(10.0))
                .child(bible_list)
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(px(10.0))
                        .child(
                            div()
                                .font_family(FONT_BODY)
                                .text_color(TOKENS.ink_muted)
                                .text_size(px(11.5))
                                .child("Import USFM, OSIS, USFX, Zefania, SQLite Bibles, or .sdeck-pack hymnals & liturgy."),
                        )
                        .child(
                            div()
                                .flex_shrink_0()
                                .flex()
                                .items_center()
                                .gap(px(7.0))
                                .child(
                                    div()
                                        .id("settings-import-bible")
                                        .cursor_pointer()
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.import_bible_pack(cx);
                                        }))
                                        .child(sculpted_secondary_button_body(
                                            Some(DeckIcon::BookOpenText),
                                            "Import Bible",
                                            TOKENS.ink_primary,
                                        )),
                                )
                                .child(
                                    div()
                                        .id("settings-import-pack")
                                        .cursor_pointer()
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.import_community_pack(cx);
                                        }))
                                        .child(sculpted_secondary_button_body(
                                            Some(DeckIcon::Stack),
                                            "Import .sdeck-pack",
                                            TOKENS.signal_preview,
                                        )),
                                ),
                        ),
                ),
        ))
        .child(section_card(
            "Migrate Songs From Another App",
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap(px(10.0))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .font_family(FONT_BODY)
                        .text_color(TOKENS.ink_secondary)
                        .text_size(px(11.5))
                        .child("Import EasyWorship (Songs.db), OpenLP (songs.sqlite), ProPresenter (.pro6), FreeShow, VideoPsalm, ChordPro, or SongSelect files."),
                )
                .child(
                    div()
                        .id("settings-migrate-songs")
                        .flex_shrink_0()
                        .cursor_pointer()
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.migrate_external_songs(cx);
                        }))
                        .child(sculpted_secondary_button_body(
                            Some(DeckIcon::UploadSimple),
                            "Migrate Library",
                            TOKENS.ink_primary,
                        )),
                ),
        ))
        .child(section_card(
            "Portable .sdeck Service Bundles",
            div()
                .flex()
                .flex_col()
                .gap(px(10.0))
                .child(
                    div()
                        .font_family(FONT_BODY)
                        .text_color(TOKENS.ink_secondary)
                        .text_size(px(12.0))
                        .child("Package this Sunday's service order, songs, custom themes, and local media into a single offline .sdeck file to move between laptops."),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .child(
                            div()
                                .id("settings-export-sdeck")
                                .cursor_pointer()
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.export_sdeck(cx);
                                }))
                                .child(sculpted_secondary_button_body(
                                    Some(DeckIcon::UploadSimple),
                                    "Export .sdeck Bundle",
                                    TOKENS.ink_primary,
                                )),
                        )
                        .child(
                            div()
                                .id("settings-import-sdeck")
                                .cursor_pointer()
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.import_sdeck(cx);
                                }))
                                .child(sculpted_secondary_button_body(
                                    Some(DeckIcon::Stack),
                                    "Import .sdeck Bundle",
                                    TOKENS.ink_primary,
                                )),
                        ),
                ),
        ))
        .child(section_card(
            "PDF / Image Slide Decks & Pre-Service Announcement Loops",
            div()
                .flex()
                .flex_col()
                .gap(px(10.0))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(px(8.0))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(6.0))
                                .child(deck_icon(DeckIcon::Timer, IconSize::Sm, TOKENS.ink_muted))
                                .child(
                                    div()
                                        .font_family(FONT_BODY)
                                        .text_color(TOKENS.ink_secondary)
                                        .text_size(px(12.0))
                                        .child("Auto-advance loop interval:"),
                                ),
                        )
                        .child(interval_chips),
                )
                .child(
                    div()
                        .flex()
                        .flex_wrap()
                        .items_center()
                        .gap(px(8.0))
                        .child(
                            div()
                                .id("settings-import-pdf-deck")
                                .cursor_pointer()
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.import_deck_files(false, cx);
                                }))
                                .child(sculpted_secondary_button_body(
                                    Some(DeckIcon::Image),
                                    "Import PDF / Images",
                                    TOKENS.ink_primary,
                                )),
                        )
                        .child(
                            div()
                                .id("settings-import-loop")
                                .cursor_pointer()
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.import_deck_files(true, cx);
                                }))
                                .child(sculpted_secondary_button_body(
                                    Some(DeckIcon::Megaphone),
                                    "Import Announcement Loop",
                                    TOKENS.ink_primary,
                                )),
                        )
                        .child(
                            div()
                                .id("settings-sample-loop")
                                .cursor_pointer()
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.add_sample_loop(cx);
                                }))
                                .child(sculpted_secondary_button_body(
                                    Some(DeckIcon::PlayFill),
                                    "Add Sample 3-Slide Loop",
                                    TOKENS.signal_preview,
                                )),
                        ),
                ),
        ))
}
