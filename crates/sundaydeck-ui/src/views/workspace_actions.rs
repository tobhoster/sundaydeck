use std::time::Duration;

use gpui::{AppContext as _, Context, Div, Entity, InteractiveElement as _};
use sundaydeck_core::bible_defaults::starter_bibles;
use sundaydeck_core::{
    BibleVerse, CustomSlideSpec, OutputOverride, PresentationCommand, ScriptureRef, ServiceItemId,
    SongId, book_by_number,
};

use crate::actions::{
    CheckForUpdates, ClearStageMessage, CloseActiveWindow, DismissOverlay, GoLive, HardCutBlack,
    HardCutClearText, HardCutLogo, ImportPastorPlan, NewCustomSlide, NewService, NewSong,
    NextLiveSlide, OpenAlertsTimers, OpenBackgroundStudio, OpenBiblesBundles, OpenSettings,
    OpenWelcomeSetup, PrevLiveSlide, QuitApp, ResetWorkspaceLayout, SwitchTabBible,
    SwitchTabCustomSlides, SwitchTabSongs, SwitchTabThemes, ToggleAudienceWindow, ToggleBlack,
    ToggleClearText, ToggleDirectClickLive, ToggleLeftPane, ToggleLibraryDrawer, ToggleLogo,
    ToggleObsOverlay, ToggleQuickEdit, ToggleRightPane, ToggleStageWindow,
};
use crate::components::{TextInput, TextInputEvent};
use crate::updater::{UpdateCheckOutcome, check_for_updates_blocking};
use crate::views::custom_slide_modal::CustomSlideModal;
use crate::views::operator_workspace::{ActiveModal, LibraryTab, OperatorWorkspace};
use crate::views::quick_edit_modal::ModalClosed;
use crate::views::settings_modal::{SettingsModal, SettingsTab};
use crate::views::song_editor_modal::EditorTab;
use crate::views::welcome_modal::WelcomeModal;

/// Quiet period after the last keystroke before the song search hits SQLite FTS.
const SONG_SEARCH_DEBOUNCE_MS: u64 = 120;

impl OperatorWorkspace {
    /// Opens the structured Custom Slide Builder modal (6 curated layouts + live 16:9 preview).
    pub fn open_custom_slide_builder(
        &mut self,
        initial: Option<CustomSlideSpec>,
        cx: &mut Context<Self>,
    ) {
        let store = self.store.clone();
        let db = self.db.clone();
        let modal = cx.new(|cx| CustomSlideModal::new(store, db, initial, cx));
        self.modal_sub = Some(cx.subscribe(&modal, |this, _, _: &ModalClosed, cx| {
            this.close_modal(cx);
        }));
        self.active_modal = Some(ActiveModal::CustomSlideBuilder(modal));
        self.active_popover = None;
        self.active_context_menu = None;
        cx.notify();
    }

    /// 1-click staging from the Auto-Follow Scripture Queue (never auto-fires live).
    pub fn stage_queued_scripture(&mut self, reference: ScriptureRef, cx: &mut Context<Self>) {
        let translation = self.store.read(cx).active_translation_code.clone();
        if let Some(db) = self.db.clone() {
            let store = self.store.clone();
            cx.spawn(async move |_, cx| {
                if let Ok(verses) = db.get_passage(&translation, reference).await
                    && !verses.is_empty()
                {
                    store.update(cx, |s, cx| {
                        let _ = s.stage_scripture_verses(&verses, false, cx);
                        s.dismiss_queued_scripture(reference, cx);
                    });
                }
            })
            .detach();
            return;
        }
        let starters = starter_bibles();
        if let Some(bible) = starters
            .iter()
            .find(|b| b.code.eq_ignore_ascii_case(&translation))
            .or_else(|| starters.first())
        {
            let book_name = book_by_number(reference.book_number)
                .map_or("Book", |b| b.name)
                .to_owned();
            let verses: Vec<BibleVerse> = bible
                .verses
                .iter()
                .filter(|v| {
                    v.book_number == reference.book_number
                        && v.chapter == reference.chapter
                        && reference.verse_start.is_none_or(|s| v.verse >= s)
                        && reference.verse_end.is_none_or(|e| v.verse <= e)
                })
                .map(|v| BibleVerse {
                    translation_code: bible.code.clone(),
                    book_number: v.book_number,
                    book_name: book_name.clone(),
                    chapter: v.chapter,
                    verse: v.verse,
                    text: v.text.clone(),
                })
                .collect();
            if !verses.is_empty() {
                self.store.update(cx, |s, cx| {
                    let _ = s.stage_scripture_verses(&verses, false, cx);
                    s.dismiss_queued_scripture(reference, cx);
                });
            }
        }
    }

    /// Opens the SundayDeck Settings & Branding modal on the default `Church & Display` tab.
    pub fn open_settings(&mut self, cx: &mut Context<Self>) {
        self.open_settings_on_tab(SettingsTab::ChurchDisplay, cx);
    }

    /// Opens the SundayDeck Settings modal directly to a specific tab (e.g., `Bibles & Service Packs`).
    pub fn open_settings_on_tab(&mut self, tab: SettingsTab, cx: &mut Context<Self>) {
        let store = self.store.clone();
        let db = self.db.clone();
        let modal = cx.new(|cx| SettingsModal::new(store, db, tab, cx));
        self.modal_sub = Some(cx.subscribe(&modal, |this, _, _: &ModalClosed, cx| {
            this.close_modal(cx);
        }));
        self.active_modal = Some(ActiveModal::Settings(modal));
        self.active_popover = None;
        self.active_context_menu = None;
        cx.notify();
    }

    /// Opens the calm 30-second first-run Welcome Setup ("Choose Your Church Style").
    pub fn open_welcome_modal(&mut self, cx: &mut Context<Self>) {
        let store = self.store.clone();
        let db = self.db.clone();
        let modal = cx.new(|cx| WelcomeModal::new(store, db, cx));
        self.modal_sub = Some(cx.subscribe(&modal, |this, _, _: &ModalClosed, cx| {
            this.close_modal(cx);
        }));
        self.active_modal = Some(ActiveModal::Welcome(modal));
        self.active_popover = None;
        self.active_context_menu = None;
        cx.notify();
    }

    pub fn add_song_to_schedule(&mut self, song_id: SongId, cx: &mut Context<Self>) {
        let updated = self
            .store
            .update(cx, |s, cx| s.add_song_to_service(song_id, cx));
        if let (Some(schedule), Some(db)) = (updated, self.db.clone()) {
            cx.spawn(async move |_, _| {
                let _ = db.save_service(schedule).await;
            })
            .detach();
        }
    }

    pub fn reorder_service_item(
        &mut self,
        item_id: ServiceItemId,
        delta: isize,
        cx: &mut Context<Self>,
    ) {
        let reordered = self
            .store
            .update(cx, |s, cx| s.move_service_item(item_id, delta, cx));
        if let (Some((service_id, ordered_ids)), Some(db)) = (reordered, self.db.clone()) {
            cx.spawn(async move |_, _| {
                let _ = db.reorder_service_items(service_id, ordered_ids).await;
            })
            .detach();
        }
    }

    pub fn remove_service_item(&mut self, item_id: ServiceItemId, cx: &mut Context<Self>) {
        let updated = self
            .store
            .update(cx, |s, cx| s.remove_service_item(item_id, cx));
        if let (Some(schedule), Some(db)) = (updated, self.db.clone()) {
            cx.spawn(async move |_, _| {
                let _ = db.save_service(schedule).await;
            })
            .detach();
        }
    }

    pub(crate) fn on_song_search(
        &mut self,
        _: Entity<TextInput>,
        event: &TextInputEvent,
        cx: &mut Context<Self>,
    ) {
        let query = match event {
            TextInputEvent::Changed(q) | TextInputEvent::Submitted(q) => q.trim().to_owned(),
        };
        // A newer keystroke always cancels the pending search, so a stale query never lands.
        self.song_search_task = None;
        if query.is_empty() {
            self.store
                .update(cx, |s, cx| s.set_song_filter(String::new(), None, cx));
        } else if let Some(db) = self.db.clone() {
            self.song_search_task = Some(cx.spawn(async move |this, cx| {
                cx.background_executor()
                    .timer(Duration::from_millis(SONG_SEARCH_DEBOUNCE_MS))
                    .await;
                let Ok(hits) = db.search_songs(&query).await else {
                    return;
                };
                let ids: Vec<SongId> = hits.into_iter().map(|h| h.id).collect();
                // `Err` only means the workspace closed while the query ran.
                let _ = this.update(cx, |ws, cx| ws.apply_song_search_hits(query, ids, cx));
            }));
        } else {
            self.filter_songs_in_memory(query, cx);
        }
    }

    /// Applies search hits only if they still answer what the search box shows.
    fn apply_song_search_hits(&self, query: String, ids: Vec<SongId>, cx: &mut Context<Self>) {
        if self.song_search_input.read(cx).value().trim() != query {
            return;
        }
        self.store
            .update(cx, |s, cx| s.set_song_filter(query, Some(ids), cx));
    }

    /// Offline fallback without a database: substring match over title, author, CCLI and lyrics.
    fn filter_songs_in_memory(&self, query: String, cx: &mut Context<Self>) {
        let needle = query.to_ascii_lowercase();
        self.store.update(cx, |s, cx| {
            let ids: Vec<SongId> = s
                .songs
                .iter()
                .filter(|song| {
                    song.title.to_ascii_lowercase().contains(&needle)
                        || song.author.to_ascii_lowercase().contains(&needle)
                        || song.ccli_number.contains(&needle)
                        || song
                            .lyrics_search_text()
                            .to_ascii_lowercase()
                            .contains(&needle)
                })
                .map(|song| song.id)
                .collect();
            s.set_song_filter(query, Some(ids), cx);
        });
    }

    pub fn create_new_service_blank(&mut self, cx: &mut Context<Self>) {
        let created = self
            .store
            .update(cx, |s, cx| s.create_new_service(String::new(), cx));
        if let Some(db) = self.db.clone() {
            cx.spawn(async move |_, _| {
                let _ = db.save_service(created).await;
            })
            .detach();
        }
        self.active_modal = None;
        self.active_popover = None;
        self.active_context_menu = None;
        cx.notify();
    }

    pub fn switch_library_tab(&mut self, tab: LibraryTab, cx: &mut Context<Self>) {
        self.active_library_tab = tab;
        self.layout.library_collapsed = false;
        self.active_modal = None;
        self.active_popover = None;
        self.active_context_menu = None;
        cx.notify();
    }

    /// Queries GitHub Releases on a background thread without blocking Sunday morning operation.
    pub fn check_for_updates(&mut self, cx: &mut Context<Self>) {
        let store = self.store.clone();
        store.update(cx, |s, cx| {
            s.set_status("Checking GitHub Releases for SundayDeck updates…", cx);
        });
        cx.spawn(async move |this, cx| {
            let outcome = cx
                .background_executor()
                .spawn(async move { check_for_updates_blocking() })
                .await;
            let _ = this.update(cx, |ws, cx| {
                match outcome {
                    UpdateCheckOutcome::UpToDate { current } => {
                        ws.available_update = None;
                        ws.store.update(cx, |s, cx| {
                            s.set_status(
                                format!("You're up to date — SundayDeck v{current} is the latest release."),
                                cx,
                            );
                        });
                    }
                    UpdateCheckOutcome::UpdateAvailable {
                        latest,
                        download_url,
                        ..
                    } => {
                        ws.available_update = Some((latest.clone(), download_url));
                        ws.store.update(cx, |s, cx| {
                            s.set_status(format!("SundayDeck v{latest} is available!"), cx);
                        });
                    }
                    UpdateCheckOutcome::OfflineOrError(msg) => {
                        ws.store.update(cx, |s, cx| s.set_status(msg, cx));
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(crate) fn bind_workspace_actions(root: Div, cx: &mut Context<Self>) -> Div {
        root.on_action(
            cx.listener(|t, _: &GoLive, _, cx| t.dispatch_cmd(PresentationCommand::GoLive, cx)),
        )
        .on_action(cx.listener(|t, _: &NextLiveSlide, _, cx| {
            t.dispatch_cmd(PresentationCommand::NextLiveSlide, cx)
        }))
        .on_action(cx.listener(|t, _: &PrevLiveSlide, _, cx| {
            t.dispatch_cmd(PresentationCommand::PrevLiveSlide, cx)
        }))
        .on_action(cx.listener(|t, _: &ToggleClearText, _, cx| {
            t.dispatch_cmd(
                PresentationCommand::ToggleOverride(OutputOverride::ClearText),
                cx,
            )
        }))
        .on_action(cx.listener(|t, _: &ToggleBlack, _, cx| {
            t.dispatch_cmd(
                PresentationCommand::ToggleOverride(OutputOverride::Black),
                cx,
            )
        }))
        .on_action(cx.listener(|t, _: &ToggleLogo, _, cx| {
            t.dispatch_cmd(
                PresentationCommand::ToggleOverride(OutputOverride::Logo),
                cx,
            )
        }))
        .on_action(cx.listener(|t, _: &HardCutClearText, _, cx| {
            t.dispatch_cmd_cut(
                PresentationCommand::ToggleOverride(OutputOverride::ClearText),
                true,
                cx,
            )
        }))
        .on_action(cx.listener(|t, _: &HardCutBlack, _, cx| {
            t.dispatch_cmd_cut(
                PresentationCommand::ToggleOverride(OutputOverride::Black),
                true,
                cx,
            )
        }))
        .on_action(cx.listener(|t, _: &HardCutLogo, _, cx| {
            t.dispatch_cmd_cut(
                PresentationCommand::ToggleOverride(OutputOverride::Logo),
                true,
                cx,
            )
        }))
        .on_action(cx.listener(|t, _: &ToggleQuickEdit, _, cx| t.open_quick_edit(cx)))
        .on_action(cx.listener(|t, _: &ToggleAudienceWindow, _, cx| {
            let s = t.store.clone();
            t.windows.update(cx, |wm, cx| wm.toggle_audience(s, cx));
        }))
        .on_action(cx.listener(|t, _: &ToggleStageWindow, _, cx| {
            let s = t.store.clone();
            t.windows.update(cx, |wm, cx| wm.toggle_stage(s, cx));
        }))
        .on_action(cx.listener(|t, _: &ToggleObsOverlay, _, cx| {
            t.store.update(cx, |s, cx| s.toggle_obs_overlay(cx));
        }))
        .on_action(cx.listener(|t, _: &ToggleDirectClickLive, _, cx| {
            let next = !t.store.read(cx).snapshot().direct_click_live;
            t.dispatch_cmd(PresentationCommand::SetDirectClickLive(next), cx);
        }))
        .on_action(cx.listener(|t, _: &ClearStageMessage, _, cx| {
            t.dispatch_cmd(PresentationCommand::SetStageMessage(None), cx)
        }))
        .on_action(cx.listener(|t, _: &NewService, _, cx| t.create_new_service_blank(cx)))
        .on_action(
            cx.listener(|t, _: &NewSong, _, cx| t.open_song_editor(None, EditorTab::Edit, cx)),
        )
        .on_action(
            cx.listener(|t, _: &NewCustomSlide, _, cx| t.open_custom_slide_builder(None, cx)),
        )
        .on_action(cx.listener(|t, _: &ImportPastorPlan, _, cx| t.open_plan_importer(cx)))
        .on_action(cx.listener(|t, _: &OpenBiblesBundles, _, cx| {
            t.open_settings_on_tab(SettingsTab::BiblesBundles, cx)
        }))
        .on_action(cx.listener(|t, _: &OpenBackgroundStudio, _, cx| t.open_background_picker(cx)))
        .on_action(cx.listener(|t, _: &OpenAlertsTimers, _, cx| t.open_alerts_timers(cx)))
        .on_action(cx.listener(|t, _: &OpenSettings, _, cx| t.open_settings(cx)))
        .on_action(cx.listener(|t, _: &OpenWelcomeSetup, _, cx| t.open_welcome_modal(cx)))
        .on_action(cx.listener(|t, _: &CheckForUpdates, _, cx| t.check_for_updates(cx)))
        .on_action(cx.listener(|t, _: &ToggleLeftPane, _, cx| {
            t.layout.left_collapsed = !t.layout.left_collapsed;
            cx.notify();
        }))
        .on_action(cx.listener(|t, _: &ToggleRightPane, _, cx| {
            t.layout.right_collapsed = !t.layout.right_collapsed;
            cx.notify();
        }))
        .on_action(cx.listener(|t, _: &ToggleLibraryDrawer, _, cx| {
            t.layout.library_collapsed = !t.layout.library_collapsed;
            cx.notify();
        }))
        .on_action(cx.listener(|t, _: &ResetWorkspaceLayout, _, cx| {
            t.layout.reset();
            t.store.update(cx, |s, cx| {
                s.set_status("Workspace layout reset to default", cx)
            });
            cx.notify();
        }))
        .on_action(cx.listener(|t, _: &DismissOverlay, _, cx| {
            if t.active_context_menu.is_some() {
                t.close_context_menu(cx);
            } else if t.active_popover.is_some() {
                t.close_popover(cx);
            } else if t.active_modal.is_some() {
                t.close_modal(cx);
            }
        }))
        .on_action(cx.listener(|t, _: &CloseActiveWindow, window, cx| {
            if t.active_context_menu.is_some() {
                t.close_context_menu(cx);
            } else if t.active_modal.is_some() {
                t.close_modal(cx);
            } else if t.active_popover.is_some() {
                t.close_popover(cx);
            } else {
                // The Operator is the app's only main window: closing it quits, through the
                // same global `QuitApp` path as the menu; voice shutdown runs in the
                // `on_app_quit` hook registered in `main.rs`.
                window.dispatch_action(Box::new(QuitApp), cx);
            }
        }))
        .on_action(
            cx.listener(|t, _: &SwitchTabSongs, _, cx| t.switch_library_tab(LibraryTab::Songs, cx)),
        )
        .on_action(
            cx.listener(|t, _: &SwitchTabBible, _, cx| t.switch_library_tab(LibraryTab::Bible, cx)),
        )
        .on_action(cx.listener(|t, _: &SwitchTabCustomSlides, _, cx| {
            t.switch_library_tab(LibraryTab::CustomSlides, cx)
        }))
        .on_action(
            cx.listener(|t, _: &SwitchTabThemes, _, cx| {
                t.switch_library_tab(LibraryTab::Themes, cx)
            }),
        )
    }
}
