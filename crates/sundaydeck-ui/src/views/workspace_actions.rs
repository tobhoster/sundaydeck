use gpui::{AppContext as _, Context, Entity};
use sundaydeck_core::bible_defaults::starter_bibles;
use sundaydeck_core::{
    BibleVerse, CustomSlideSpec, ScriptureRef, ServiceItemId, SongId, book_by_number,
};

use crate::components::{TextInput, TextInputEvent};
use crate::views::custom_slide_modal::CustomSlideModal;
use crate::views::operator_workspace::{ActiveModal, OperatorWorkspace};
use crate::views::quick_edit_modal::ModalClosed;
use crate::views::settings_modal::{SettingsModal, SettingsTab};
use crate::views::welcome_modal::WelcomeModal;

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
        if query.is_empty() {
            self.store.update(cx, |s, cx| {
                s.song_search_query.clear();
                s.filtered_song_ids = None;
                cx.notify();
            });
            return;
        }

        if let Some(db) = self.db.clone() {
            let store = self.store.clone();
            cx.spawn(async move |_, cx| {
                if let Ok(hits) = db.search_songs(&query).await {
                    let ids: Vec<SongId> = hits.into_iter().map(|h| h.id).collect();
                    store.update(cx, |s, cx| {
                        s.song_search_query = query;
                        s.filtered_song_ids = Some(ids);
                        cx.notify();
                    });
                }
            })
            .detach();
        } else {
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
                s.song_search_query = query;
                s.filtered_song_ids = Some(ids);
                cx.notify();
            });
        }
    }
}
