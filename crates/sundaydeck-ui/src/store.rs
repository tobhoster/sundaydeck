use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use gpui::{Context, Entity, EventEmitter, Global};
use sundaydeck_core::{
    BibleTranslation, BibleVerse, BilingualDisplayMode, CustomSlideSpec, DeckCursor,
    PresentationCommand, PresentationEngine, PresentationError, PresentationSnapshot, ReflowMode,
    ScriptureRef, ServiceId, ServiceItem, ServiceItemId, ServiceItemPayload, ServiceSchedule,
    SlideId, Song, SongId, Theme, ThemeId, ThemeTarget, default_custom_slide_templates,
    reflow_song_bilingual,
};
use sundaydeck_db::{AppSettings, BootstrapData, DbWorker};
use sundaydeck_overlay::{OverlayPayload, OverlayServerHandle};
use sundaydeck_voice::{AudioInputDevice, VoiceEngineHandle, VoiceTelemetry};

use crate::deck_builder::build_deck_for_service_item;
use crate::design::motion::MotionMode;
use crate::design::primitives::VoiceSyncMode;
use crate::windows::WindowManager;

#[derive(Debug, Clone, Copy)]
pub struct StoreEvent {
    pub kind: sundaydeck_core::TransitionKind,
    pub version: u64,
    pub hard_cut: bool,
}

pub struct AppServices {
    pub store: Entity<PresentationStore>,
    pub windows: Entity<WindowManager>,
    pub db: Option<DbWorker>,
}

impl Global for AppServices {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BibleBrowseMode {
    Chapter { book_number: u8, chapter: u16 },
    Search { query: String },
}

/// Single source of truth for live presentation state, library cache, offline Bible browser,
/// active service schedule, alerts/timers, and non-blocking operator status messages.
pub struct PresentationStore {
    engine: PresentationEngine,
    pub settings: AppSettings,
    pub themes: Vec<Arc<Theme>>,
    pub songs: Vec<Song>,
    pub custom_slides: Vec<CustomSlideSpec>,
    pub filtered_song_ids: Option<Vec<SongId>>,
    pub song_search_query: String,
    pub services: Vec<ServiceSchedule>,
    pub service: ServiceSchedule,
    pub selected_service_item_id: Option<ServiceItemId>,
    pub preview_reflow_mode: ReflowMode,
    pub bible_translations: Vec<BibleTranslation>,
    pub active_translation_code: String,
    pub bible_browse_mode: BibleBrowseMode,
    pub bible_verses: Vec<BibleVerse>,
    pub selected_verse_range: Option<(usize, usize)>,
    pub queued_scriptures: Vec<ScriptureRef>,
    pub status_banner: Option<String>,
    pub clock_override_ms: Option<i64>,
    pub fixed_clock_label: Option<String>,
    pub motion_mode: MotionMode,
    pub voice_sync_mode: VoiceSyncMode,
    pub voice_telemetry: VoiceTelemetry,
    pub voice_devices: Vec<AudioInputDevice>,
    pub(crate) voice_engine: Option<VoiceEngineHandle>,
    pub overlay_server: Option<OverlayServerHandle>,
    pub(crate) last_loop_advance_ms: i64,
}

impl EventEmitter<StoreEvent> for PresentationStore {}

impl PresentationStore {
    pub fn from_bootstrap(data: BootstrapData) -> Self {
        let mut engine = PresentationEngine::new(
            data.settings.logo_asset_id,
            data.settings.church_name.clone(),
        );
        let _ = engine.dispatch(PresentationCommand::SetLogoUri(
            data.settings.logo_uri.clone(),
        ));
        let _ = engine.dispatch(PresentationCommand::SetDirectClickLive(
            data.settings.direct_click_live,
        ));

        let motion_mode = match data.settings.motion_mode.as_str() {
            "crossfade" => MotionMode::CrossfadeOnly,
            "instant" => MotionMode::Instant,
            _ => MotionMode::Full,
        };
        let themes: Vec<Arc<Theme>> = data.themes.into_iter().map(Arc::new).collect();
        let mut services = data.services;
        if services.is_empty() {
            services.push(sundaydeck_core::defaults::default_sample_service());
        }
        let service = services[0].clone();
        let active_translation_code = data
            .bible_translations
            .iter()
            .find(|t| t.code.eq_ignore_ascii_case("WEB"))
            .or_else(|| data.bible_translations.first())
            .map(|t| t.code.clone())
            .unwrap_or_else(|| "WEB".to_owned());

        let mut store = Self {
            engine,
            settings: data.settings,
            themes,
            songs: data.loaded_songs,
            custom_slides: default_custom_slide_templates(),
            filtered_song_ids: None,
            song_search_query: String::new(),
            services,
            service,
            selected_service_item_id: None,
            preview_reflow_mode: ReflowMode::ByStanza,
            bible_translations: data.bible_translations,
            active_translation_code,
            bible_browse_mode: BibleBrowseMode::Chapter {
                book_number: 19,
                chapter: 23,
            },
            bible_verses: data.initial_chapter_verses,
            selected_verse_range: Some((0, 0)),
            queued_scriptures: Vec::new(),
            status_banner: None,
            clock_override_ms: None,
            fixed_clock_label: None,
            motion_mode,
            voice_sync_mode: VoiceSyncMode::Off,
            voice_telemetry: VoiceTelemetry::default(),
            voice_devices: Vec::new(),
            voice_engine: None,
            overlay_server: None,
            last_loop_advance_ms: 0,
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

    pub fn now_epoch_ms(&self) -> i64 {
        if let Some(fixed) = self.clock_override_ms {
            return fixed;
        }
        let millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        i64::try_from(millis).unwrap_or(i64::MAX)
    }

    pub fn snapshot(&self) -> Arc<PresentationSnapshot> {
        self.engine.snapshot()
    }

    pub fn service_filename(&self) -> String {
        let slug: String = self
            .service
            .title
            .to_ascii_lowercase()
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .collect();
        let trimmed = slug.trim_matches('-');
        if trimmed.is_empty() {
            "service.sdeck".to_owned()
        } else {
            format!("{trimmed}.sdeck")
        }
    }

    pub fn dispatch(
        &mut self,
        command: PresentationCommand,
        cx: &mut Context<Self>,
    ) -> Result<(), PresentationError> {
        self.dispatch_with_cut(command, false, cx)
    }

    pub fn dispatch_with_cut(
        &mut self,
        command: PresentationCommand,
        hard_cut: bool,
        cx: &mut Context<Self>,
    ) -> Result<(), PresentationError> {
        let transition = self.engine.dispatch(command)?;
        self.status_banner = None;
        if matches!(
            transition.kind,
            sundaydeck_core::TransitionKind::LiveCrossfade
                | sundaydeck_core::TransitionKind::OverrideChanged
        ) {
            self.last_loop_advance_ms = self.now_epoch_ms();
            self.voice_telemetry.suggested_slide_index = None;
            self.voice_telemetry.active_slide_progress = 0.0;
            self.sync_voice_engine_state();
        }
        if let Some(overlay) = &self.overlay_server {
            overlay.publish(&OverlayPayload::from_snapshot(&transition.snapshot));
        }
        cx.emit(StoreEvent {
            kind: transition.kind,
            version: transition.snapshot.version,
            hard_cut,
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

    pub fn visible_songs(&self) -> Vec<Song> {
        match &self.filtered_song_ids {
            None => self.songs.clone(),
            Some(ids) => ids
                .iter()
                .filter_map(|id| self.song_by_id(*id).cloned())
                .collect(),
        }
    }

    pub fn upsert_song_local(&mut self, song: Song, cx: &mut Context<Self>) {
        let song_id = song.id;
        if let Some(existing) = self.songs.iter_mut().find(|s| s.id == song_id) {
            *existing = song;
        } else {
            self.songs.push(song);
            self.songs.sort_by_key(|a| a.title.to_lowercase());
        }
        self.filtered_song_ids = None;
        self.song_search_query.clear();
        self.stage_library_song(song_id, cx);
    }

    pub fn patch_live_slide_and_song(
        &mut self,
        slide_id: SlideId,
        new_body: String,
        persist_to_song: bool,
        cx: &mut Context<Self>,
    ) -> Option<Song> {
        let source_section_id = self
            .snapshot()
            .live
            .as_ref()
            .and_then(|cursor| cursor.slides.iter().find(|s| s.id == slide_id))
            .and_then(|s| s.source_section_id);

        if let Err(err) = self.dispatch(
            PresentationCommand::PatchLiveSlideText {
                slide_id,
                new_body: new_body.clone(),
            },
            cx,
        ) {
            self.set_status(err.to_string(), cx);
            return None;
        }

        if persist_to_song
            && let Some(sec_id) = source_section_id
            && let Some(song) = self
                .songs
                .iter_mut()
                .find(|s| s.sections.iter().any(|sec| sec.id == sec_id))
        {
            if let Some(sec) = song.sections.iter_mut().find(|sec| sec.id == sec_id) {
                sec.body = new_body;
            }
            return Some(song.clone());
        }
        None
    }

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
        let slides = reflow_song_bilingual(
            &song,
            song.default_arrangement_id,
            Some(self.preview_reflow_mode),
            self.active_bilingual_mode(),
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
            let slides = reflow_song_bilingual(
                &song,
                song.default_arrangement_id,
                Some(mode),
                self.active_bilingual_mode(),
            );
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

    pub fn active_bilingual_mode(&self) -> BilingualDisplayMode {
        match self.settings.bilingual_mode.as_str() {
            "primary_only" => BilingualDisplayMode::PrimaryOnly,
            "secondary_only" => BilingualDisplayMode::SecondaryOnly,
            _ => BilingualDisplayMode::BilingualStacked,
        }
    }

    fn build_deck_for_item(
        &self,
        item: &ServiceItem,
        reflow_override: Option<ReflowMode>,
    ) -> Result<DeckCursor, PresentationError> {
        build_deck_for_service_item(
            item,
            reflow_override,
            self.active_bilingual_mode(),
            &self.songs,
            &self.themes,
            self.active_theme(),
        )
    }
}
