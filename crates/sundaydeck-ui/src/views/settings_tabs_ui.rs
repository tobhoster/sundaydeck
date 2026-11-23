use gpui::{
    Context, Div, FontWeight, InteractiveElement, ParentElement, StatefulInteractiveElement,
    Styled, div, px,
};

use crate::design::icons::{DeckIcon, IconSize, deck_icon};
use crate::design::primitives::{neutral_option_chip, sculpted_secondary_button_body};
use crate::design::tokens::{FONT_BODY, FONT_MONO, RADIUS_CONTROL, TOKENS};
use crate::views::settings_modal::{
    LLM_PRESET_CHOICES, LOOP_INTERVAL_CHOICES, SettingsModal, VOICE_THRESHOLD_CHOICES,
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
            "Local AI Service Planner (Offline llama.cpp / Ollama)",
            div()
                .flex()
                .flex_col()
                .gap(px(11.0))
                .child(field_label(
                    "OpenAI-Compatible Local Endpoint",
                    modal.llm_url_input.clone(),
                ))
                .child(llm_chips),
        ))
        .child(section_card(
            "Voice-Synced Auto-Follow (Offline Whisper)",
            div()
                .flex()
                .flex_col()
                .gap(px(11.0))
                .child(field_label(
                    "Local Whisper HTTP Inference URL",
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

    div()
        .w_full()
        .flex()
        .flex_col()
        .gap(px(14.0))
        .child(section_card(
            "Offline 66-Book Bible Packs",
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
                                .child("Import OSIS, USFX, or Zefania .xml or .sqlite full Bibles for instant offline lookup."),
                        )
                        .child(
                            div()
                                .id("settings-import-bible")
                                .flex_shrink_0()
                                .cursor_pointer()
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.import_bible_pack(cx);
                                }))
                                .child(sculpted_secondary_button_body(
                                    Some(DeckIcon::BookOpenText),
                                    "Import Bible File",
                                    TOKENS.ink_primary,
                                )),
                        ),
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
