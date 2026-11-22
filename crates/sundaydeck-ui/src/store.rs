use std::sync::Arc;

use gpui::{Context, Entity, EventEmitter, Global};
use sundaydeck_core::{
    DeckCursor, PresentationCommand, PresentationEngine, PresentationError, PresentationSnapshot,
    ReflowMode, ServiceId, ServiceItem, ServiceItemId, ServiceItemPayload, ServiceSchedule, Song,
    SongId, Theme, ThemeId, ThemeTarget, reflow_song,
};
use sundaydeck_db::{AppSettings, BootstrapData, DbWorker};

use crate::deck_builder::build_deck_for_service_item;
use crate::windows::WindowManager;

/// Event emitted whenever [`PresentationStore`] updates state, telling subscribing
/// windows whether to crossfade or update in place.
#[derive(Debug, Clone, Copy)]
pub struct StoreEvent {
    pub kind: sundaydeck_core::TransitionKind,
    pub version: u64,
}

/// Application-wide service handles stored as a GPUI [`Global`].
pub struct AppServices {
    pub store: Entity<PresentationStore>,
    pub windows: Entity<WindowManager>,
    pub db: Option<DbWorker>,
}

impl Global for AppServices {}

/// Single source of truth for live presentation state, library cache, active service,
/// and non-blocking operator status messages.
pub struct PresentationStore {
    engine: PresentationEngine,
    pub settings: AppSettings,
    pub themes: Vec<Arc<Theme>>,
    pub songs: Vec<Song>,
    pub service: ServiceSchedule,
    pub selected_service_item_id: Option<ServiceItemId>,
    pub preview_reflow_mode: ReflowMode,
    pub status_banner: Option<String>,
}

impl EventEmitter<StoreEvent> for PresentationStore {}

impl PresentationStore {
    pub fn from_bootstrap(data: BootstrapData) -> Self {
        let mut engine = PresentationEngine::new(
            data.settings.logo_asset_id,
            data.settings.church_name.clone(),
        );
        let _ = engine.dispatch(PresentationCommand::SetDirectClickLive(
            data.settings.direct_click_live,
        ));

        let themes: Vec<Arc<Theme>> = data.themes.into_iter().map(Arc::new).collect();
        let service = data
            .services
            .into_iter()
            .next()
            .unwrap_or_else(sundaydeck_core::defaults::default_sample_service);

        let mut store = Self {
            engine,
            settings: data.settings,
            themes,
            songs: data.loaded_songs,
            service,
            selected_service_item_id: None,
            preview_reflow_mode: ReflowMode::ByStanza,
            status_banner: None,
        };

        if let Some(first_item) = store.service.items.first().cloned() {
            store.selected_service_item_id = Some(first_item.id);
            if let Ok(deck) = store.build_deck_for_item(&first_item, None) {
                let _ = store
                    .engine
                    .dispatch(PresentationCommand::StagePreview(deck));
            }
        }
        store
    }

    pub fn snapshot(&self) -> Arc<PresentationSnapshot> {
        self.engine.snapshot()
    }

    pub fn dispatch(
        &mut self,
        command: PresentationCommand,
        cx: &mut Context<Self>,
    ) -> Result<(), PresentationError> {
        let transition = self.engine.dispatch(command)?;
        self.status_banner = None;
        cx.emit(StoreEvent {
            kind: transition.kind,
            version: transition.snapshot.version,
        });
        cx.notify();
        Ok(())
    }

    pub fn set_status(&mut self, message: impl Into<String>, cx: &mut Context<Self>) {
        self.status_banner = Some(message.into());
        cx.notify();
    }

    pub fn clear_status(&mut self, cx: &mut Context<Self>) {
        self.status_banner = None;
        cx.notify();
    }

    pub fn active_theme(&self) -> Arc<Theme> {
        self.snapshot()
            .live
            .as_ref()
            .or(self.snapshot().preview.as_ref())
            .map(|cursor| Arc::clone(&cursor.theme))
            .or_else(|| self.default_theme_for(ThemeTarget::Song))
            .unwrap_or_else(|| Arc::new(sundaydeck_core::defaults::default_themes().remove(0)))
    }

    pub fn default_theme_for(&self, target: ThemeTarget) -> Option<Arc<Theme>> {
        self.themes
            .iter()
            .find(|t| t.target == target && t.is_default)
            .or_else(|| self.themes.iter().find(|t| t.target == target))
            .or_else(|| self.themes.first())
            .cloned()
    }

    pub fn theme_by_id(&self, id: ThemeId) -> Option<Arc<Theme>> {
        self.themes.iter().find(|t| t.id == id).cloned()
    }

    pub fn song_by_id(&self, id: SongId) -> Option<&Song> {
        self.songs.iter().find(|s| s.id == id)
    }

    /// Stages a service item into Preview (or straight to Live if Direct-Click is enabled).
    pub fn activate_service_item(&mut self, item_id: ServiceItemId, cx: &mut Context<Self>) {
        let Some(item_idx) = self.service.items.iter().position(|i| i.id == item_id) else {
            return;
        };
        let item = self.service.items[item_idx].clone();
        self.selected_service_item_id = Some(item_id);

        if let ServiceItemPayload::Song {
            reflow_override, ..
        } = &item.payload
        {
            self.preview_reflow_mode = reflow_override.unwrap_or(ReflowMode::ByStanza);
        }

        let deck = match self.build_deck_for_item(&item, None) {
            Ok(deck) => deck,
            Err(err) => {
                self.set_status(format!("Could not load '{}': {err}", item.title), cx);
                return;
            }
        };

        let next_slide_peek = self
            .service
            .items
            .get(item_idx + 1)
            .and_then(|next_item| self.build_deck_for_item(next_item, None).ok())
            .and_then(|next_deck| next_deck.slides.first().cloned());

        let cmd = if self.snapshot().direct_click_live {
            PresentationCommand::GoLiveDirect(deck)
        } else {
            PresentationCommand::StagePreview(deck)
        };
        let _ = self.dispatch(cmd, cx);
        let _ = self.dispatch(
            PresentationCommand::SetNextItemFirstSlide(next_slide_peek),
            cx,
        );
    }

    /// Stages a library song into Preview without adding it to the service schedule.
    pub fn stage_library_song(&mut self, song_id: SongId, cx: &mut Context<Self>) {
        let Some(song) = self.song_by_id(song_id).cloned() else {
            self.set_status("Song no longer exists in library", cx);
            return;
        };
        self.selected_service_item_id = None;
        self.preview_reflow_mode = song.default_reflow_mode;

        let theme = song
            .default_theme_id
            .and_then(|id| self.theme_by_id(id))
            .or_else(|| self.default_theme_for(ThemeTarget::Song))
            .unwrap_or_else(|| self.active_theme());
        let slides = reflow_song(
            &song,
            song.default_arrangement_id,
            Some(self.preview_reflow_mode),
        );

        match DeckCursor::try_new(None, song.title, slides, theme) {
            Ok(deck) => {
                let cmd = if self.snapshot().direct_click_live {
                    PresentationCommand::GoLiveDirect(deck)
                } else {
                    PresentationCommand::StagePreview(deck)
                };
                let _ = self.dispatch(cmd, cx);
            }
            Err(err) => self.set_status(err.to_string(), cx),
        }
    }

    /// Re-paginates the currently staged Preview song under a new [`ReflowMode`].
    pub fn set_preview_reflow_mode(&mut self, mode: ReflowMode, cx: &mut Context<Self>) {
        self.preview_reflow_mode = mode;

        if let Some(item_id) = self.selected_service_item_id
            && let Some(item) = self.service.items.iter().find(|i| i.id == item_id).cloned()
            && let Ok(deck) = self.build_deck_for_item(&item, Some(mode))
        {
            let _ = self.dispatch(PresentationCommand::StagePreview(deck), cx);
            return;
        }

        if let Some(preview) = self.snapshot().preview.as_ref()
            && let Some(song) = self
                .songs
                .iter()
                .find(|s| s.title == preview.title)
                .cloned()
        {
            let slides = reflow_song(&song, song.default_arrangement_id, Some(mode));
            if let Ok(deck) =
                DeckCursor::try_new(None, song.title, slides, Arc::clone(&preview.theme))
            {
                let _ = self.dispatch(PresentationCommand::StagePreview(deck), cx);
            }
        }
    }

    pub fn add_song_to_service(
        &mut self,
        song_id: SongId,
        cx: &mut Context<Self>,
    ) -> Option<ServiceSchedule> {
        let song = self.song_by_id(song_id)?.clone();
        self.service.items.push(ServiceItem {
            id: ServiceItemId::new_v7(),
            title: song.title.clone(),
            theme_override_id: song.default_theme_id,
            payload: ServiceItemPayload::Song {
                song_id: song.id,
                arrangement_id: song.default_arrangement_id,
                reflow_override: None,
            },
        });
        cx.notify();
        Some(self.service.clone())
    }

    pub fn move_service_item(
        &mut self,
        item_id: ServiceItemId,
        delta: isize,
        cx: &mut Context<Self>,
    ) -> Option<(ServiceId, Vec<ServiceItemId>)> {
        let idx = self.service.items.iter().position(|i| i.id == item_id)?;
        let target = if delta < 0 {
            idx.checked_sub(delta.unsigned_abs())?
        } else {
            let next = idx.checked_add(delta as usize)?;
            if next >= self.service.items.len() {
                return None;
            }
            next
        };
        self.service.items.swap(idx, target);
        let ordered_ids: Vec<ServiceItemId> = self.service.items.iter().map(|i| i.id).collect();
        cx.notify();
        Some((self.service.id, ordered_ids))
    }

    fn build_deck_for_item(
        &self,
        item: &ServiceItem,
        reflow_override: Option<ReflowMode>,
    ) -> Result<DeckCursor, PresentationError> {
        build_deck_for_service_item(
            item,
            reflow_override,
            &self.songs,
            &self.themes,
            self.active_theme(),
        )
    }
}
