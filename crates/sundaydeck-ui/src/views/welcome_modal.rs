use gpui::{
    AppContext as _, Context, Entity, EventEmitter, FontWeight, InteractiveElement, IntoElement,
    ParentElement, PathPromptOptions, Render, StatefulInteractiveElement, Styled, Window, div, px,
};
use sundaydeck_core::import_sdeck_pack;
use sundaydeck_db::DbWorker;

use crate::components::TextInput;
use crate::deck_import::copy_church_logo_file;
use crate::design::icons::{DeckIcon, IconSize, deck_icon};
use crate::design::primitives::{
    primary_ivory_button_body, sculpted_secondary_button_body, status_dot_label,
};
use crate::design::tokens::{
    FONT_BODY, FONT_DISPLAY, RADIUS_CONTROL, RADIUS_MODAL, RADIUS_TILE, TOKENS,
};
use crate::store::PresentationStore;
use crate::views::quick_edit_modal::ModalClosed;
use crate::views::settings_migration::import_external_song_files;

const WELCOME_PROFILES: &[(&str, &str, &str)] = &[
    (
        "contemporary",
        "Contemporary",
        "Non-denominational & modern worship lyrics, sermon points, and motion backdrops.",
    ),
    (
        "baptist_evangelical",
        "Baptist & Evangelical",
        "Instant Scripture lookup, Baptist Hymnal jump (#330), and sermon outlines.",
    ),
    (
        "methodist_mainline",
        "Methodist / Mainline",
        "Classic hymnbook numbering (UMH), Apostles' Creed, and responsive readings.",
    ),
    (
        "catholic_parish",
        "Catholic Parish (73-Book)",
        "Enables the 73-book Deuterocanonical Bible and Order of Mass call-and-response.",
    ),
    (
        "spanish_bilingual",
        "Spanish / Portuguese",
        "Stacked bilingual lyrics, accent-tolerant Scripture search, and Himnario / Hinário.",
    ),
];

/// Calm 30-second first-run onboarding modal ("Welcome to SundayDeck — Choose Your Church Style").
pub struct WelcomeModal {
    store: Entity<PresentationStore>,
    db: Option<DbWorker>,
    church_name_input: Entity<TextInput>,
    selected_profile: String,
    logo_uri: Option<String>,
    status_note: Option<String>,
    error_message: Option<String>,
}

impl EventEmitter<ModalClosed> for WelcomeModal {}

impl WelcomeModal {
    pub fn new(
        store: Entity<PresentationStore>,
        db: Option<DbWorker>,
        cx: &mut Context<Self>,
    ) -> Self {
        let cur = store.read(cx).settings.clone();
        let church_name_input = cx.new(|cx| {
            TextInput::single_line(
                "welcome-church-name",
                "Enter your church or ministry name",
                &cur.church_name,
                cx,
            )
        });
        Self {
            store,
            db,
            church_name_input,
            selected_profile: cur.church_profile,
            logo_uri: cur.logo_uri,
            status_note: None,
            error_message: None,
        }
    }

    fn finish(&mut self, apply_choices: bool, cx: &mut Context<Self>) {
        let mut settings = self.store.read(cx).settings.clone();
        settings.onboarding_completed = true;
        if apply_choices {
            let name = self.church_name_input.read(cx).value().trim().to_owned();
            if !name.is_empty() {
                settings.church_name = name;
            }
            settings.church_profile = self.selected_profile.clone();
            settings.enable_catholic_canon = self.selected_profile == "catholic_parish";
            if self.selected_profile == "spanish_bilingual" {
                settings.bilingual_mode = "bilingual_stacked".to_owned();
            }
            settings.logo_uri = self.logo_uri.clone();
        }
        let saved = self
            .store
            .update(cx, |s, cx| s.apply_settings(settings, cx));
        if let Some(db) = self.db.clone() {
            cx.spawn(async move |_, _| {
                let _ = db.save_settings(saved).await;
            })
            .detach();
        }
        cx.emit(ModalClosed);
    }

    fn choose_logo(&mut self, cx: &mut Context<Self>) {
        let rx = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Select Church Logo (.png, .jpg, .webp)".into()),
        });
        cx.spawn(async move |this, cx| {
            let Ok(Ok(Some(paths))) = rx.await else { return };
            let Some(src) = paths.into_iter().next() else { return };
            let res = cx
                .background_executor()
                .spawn(async move { copy_church_logo_file(&src) })
                .await;
            let _ = this.update(cx, |m, cx| {
                match res {
                    Ok(saved) => {
                        m.logo_uri = Some(saved.to_string_lossy().into_owned());
                        m.status_note = Some("Church logo ready for Show Logo (F3).".to_owned());
                        m.error_message = None;
                    }
                    Err(err) => m.error_message = Some(err),
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn import_starter_pack(&mut self, cx: &mut Context<Self>) {
        let Some(db) = self.db.clone() else { return };
        let rx = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Select Starter or Church Pack (.sdeck-pack)".into()),
        });
        let store = self.store.clone();
        cx.spawn(async move |this, cx| {
            let Ok(Ok(Some(paths))) = rx.await else { return };
            let Some(src) = paths.into_iter().next() else { return };
            let loaded = cx
                .background_executor()
                .spawn(async move { import_sdeck_pack(&src).map_err(|e| e.to_string()) })
                .await;
            match loaded {
                Ok(pack) => {
                    let name = pack.manifest.name.clone();
                    let songs = pack.songs.clone();
                    let res = db.install_sdeck_pack(pack).await.map_err(|e| e.to_string());
                    let _ = this.update(cx, |m, cx| {
                        match res {
                            Ok(r) => {
                                for song in songs {
                                    store.update(cx, |s, cx| s.upsert_song_local(song, cx));
                                }
                                m.status_note = Some(format!(
                                    "Installed \"{name}\" ({} Bibles, {} songs).",
                                    r.bibles_installed, r.songs_installed
                                ));
                                m.error_message = None;
                            }
                            Err(e) => m.error_message = Some(e),
                        }
                        cx.notify();
                    });
                }
                Err(e) => {
                    let _ = this.update(cx, |m, cx| {
                        m.error_message = Some(e);
                        cx.notify();
                    });
                }
            }
        })
        .detach();
    }

    fn migrate_songs(&mut self, cx: &mut Context<Self>) {
        let rx = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: true,
            prompt: Some(
                "Import Songs From EasyWorship, OpenLP, ProPresenter, FreeShow, VideoPsalm, ChordPro, or SongSelect"
                    .into(),
            ),
        });
        let store = self.store.clone();
        let db = self.db.clone();
        cx.spawn(async move |this, cx| {
            let Ok(Ok(Some(paths))) = rx.await else { return };
            if paths.is_empty() { return; }
            let res = cx
                .background_executor()
                .spawn(async move { import_external_song_files(&paths) })
                .await;
            match res {
                Ok((songs, src)) => {
                    let n = songs.len();
                    for song in songs {
                        store.update(cx, |s, cx| s.upsert_song_local(song.clone(), cx));
                        if let Some(worker) = db.as_ref() {
                            let _ = worker.upsert_song(song).await;
                        }
                    }
                    let _ = this.update(cx, |m, cx| {
                        m.status_note = Some(format!("Migrated {n} song(s) from {src}."));
                        m.error_message = None;
                        cx.notify();
                    });
                }
                Err(e) => {
                    let _ = this.update(cx, |m, cx| {
                        m.error_message = Some(e);
                        cx.notify();
                    });
                }
            }
        })
        .detach();
    }
}

fn field_header(label: &'static str) -> gpui::Div {
    div()
        .font_family(FONT_BODY)
        .text_color(TOKENS.ink_secondary)
        .text_size(px(11.5))
        .font_weight(FontWeight::MEDIUM)
        .child(label)
}

impl Render for WelcomeModal {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut profiles_grid = div().w_full().flex().flex_col().gap(px(7.0));
        for (id, title, desc) in WELCOME_PROFILES {
            let active = self.selected_profile == *id;
            let id_owned = (*id).to_owned();
            profiles_grid = profiles_grid.child(
                div()
                    .id(format!("welcome-prof-{id}"))
                    .w_full()
                    .px(px(12.0))
                    .py(px(9.0))
                    .rounded(px(RADIUS_CONTROL))
                    .cursor_pointer()
                    .bg(if active { TOKENS.elevated } else { TOKENS.recessed })
                    .border_1()
                    .border_color(if active {
                        TOKENS.signal_preview
                    } else {
                        TOKENS.gridline
                    })
                    .hover(|s| s.bg(TOKENS.elevated_hover))
                    .flex()
                    .items_center()
                    .gap(px(10.0))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.selected_profile = id_owned.clone();
                        cx.notify();
                    }))
                    .child(
                        div()
                            .w(px(7.0))
                            .h(px(7.0))
                            .flex_shrink_0()
                            .rounded(px(99.0))
                            .bg(if active {
                                TOKENS.signal_preview
                            } else {
                                TOKENS.gridline
                            }),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap(px(1.5))
                            .child(
                                div()
                                    .font_family(FONT_DISPLAY)
                                    .text_color(TOKENS.ink_primary)
                                    .text_size(px(12.5))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(*title),
                            )
                            .child(
                                div()
                                    .font_family(FONT_BODY)
                                    .text_color(TOKENS.ink_secondary)
                                    .text_size(px(11.0))
                                    .child(*desc),
                            ),
                    ),
            );
        }

        let footer_msg = if let Some(err) = &self.error_message {
            status_dot_label(err, TOKENS.signal_live_dot, TOKENS.ink_primary)
        } else if let Some(note) = &self.status_note {
            status_dot_label(note, TOKENS.signal_voice, TOKENS.ink_secondary)
        } else {
            div()
                .font_family(FONT_BODY)
                .text_color(TOKENS.ink_muted)
                .text_size(px(11.5))
                .child("Free forever · 100% offline · You can change any of this later in Settings")
        };

        div()
            .w(px(680.0))
            .max_h(px(760.0))
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
                    .px(px(22.0))
                    .py(px(16.0))
                    .border_b_1()
                    .border_color(TOKENS.hairline)
                    .flex()
                    .items_center()
                    .gap(px(10.0))
                    .child(deck_icon(DeckIcon::Monitor, IconSize::Md, TOKENS.signal_preview))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(2.0))
                            .child(
                                div()
                                    .font_family(FONT_DISPLAY)
                                    .text_color(TOKENS.ink_primary)
                                    .text_size(px(16.0))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child("Welcome to SundayDeck"),
                            )
                            .child(
                                div()
                                    .font_family(FONT_BODY)
                                    .text_color(TOKENS.ink_secondary)
                                    .text_size(px(12.0))
                                    .child("Let's set up your sanctuary in 30 seconds."),
                            ),
                    ),
            )
            .child(
                div()
                    .w_full()
                    .p(px(20.0))
                    .flex()
                    .flex_col()
                    .gap(px(14.0))
                    .child(
                        div()
                            .w_full()
                            .flex()
                            .flex_col()
                            .gap(px(5.0))
                            .child(field_header("Church or Ministry Name"))
                            .child(self.church_name_input.clone()),
                    )
                    .child(
                        div()
                            .w_full()
                            .flex()
                            .flex_col()
                            .gap(px(6.0))
                            .child(field_header("Choose Your Church Style"))
                            .child(profiles_grid),
                    )
                    .child(
                        div()
                            .w_full()
                            .p(px(12.0))
                            .rounded(px(RADIUS_TILE))
                            .bg(TOKENS.slide_card)
                            .border_1()
                            .border_color(TOKENS.gridline)
                            .flex()
                            .flex_wrap()
                            .items_center()
                            .gap(px(8.0))
                            .child(
                                div()
                                    .id("welcome-logo-btn")
                                    .cursor_pointer()
                                    .on_click(cx.listener(|this, _, _, cx| this.choose_logo(cx)))
                                    .child(sculpted_secondary_button_body(
                                        Some(DeckIcon::Image),
                                        if self.logo_uri.is_some() {
                                            "Logo Ready"
                                        } else {
                                            "Add Church Logo"
                                        },
                                        TOKENS.ink_primary,
                                    )),
                            )
                            .child(
                                div()
                                    .id("welcome-pack-btn")
                                    .cursor_pointer()
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.import_starter_pack(cx);
                                    }))
                                    .child(sculpted_secondary_button_body(
                                        Some(DeckIcon::Stack),
                                        "Import .sdeck-pack",
                                        TOKENS.ink_primary,
                                    )),
                            )
                            .child(
                                div()
                                    .id("welcome-migrate-btn")
                                    .cursor_pointer()
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.migrate_songs(cx);
                                    }))
                                    .child(sculpted_secondary_button_body(
                                        Some(DeckIcon::UploadSimple),
                                        "Migrate Existing Songs",
                                        TOKENS.signal_preview,
                                    )),
                            ),
                    ),
            )
            .child(
                div()
                    .w_full()
                    .px(px(20.0))
                    .py(px(13.0))
                    .bg(TOKENS.recessed)
                    .border_t_1()
                    .border_color(TOKENS.hairline)
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(12.0))
                    .child(div().flex_1().min_w_0().child(footer_msg))
                    .child(
                        div()
                            .flex_shrink_0()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .child(
                                div()
                                    .id("welcome-skip")
                                    .cursor_pointer()
                                    .on_click(cx.listener(|this, _, _, cx| this.finish(false, cx)))
                                    .child(sculpted_secondary_button_body(
                                        None,
                                        "Skip for now",
                                        TOKENS.ink_secondary,
                                    )),
                            )
                            .child(
                                div()
                                    .id("welcome-start")
                                    .cursor_pointer()
                                    .on_click(cx.listener(|this, _, _, cx| this.finish(true, cx)))
                                    .child(primary_ivory_button_body(
                                        Some(DeckIcon::Check),
                                        "Start Using SundayDeck",
                                        true,
                                    )),
                            ),
                    ),
            )
    }
}
