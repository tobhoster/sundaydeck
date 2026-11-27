use std::collections::HashMap;
use std::sync::Arc;

use gpui::Context;
use sundaydeck_core::{
    BibleTranslation, CustomSlideSpec, DeckCursor, OutputOverride, PresentationCommand,
    ScriptureRef, SdeckBundle, ServiceId, ServiceItem, ServiceItemId, ServiceItemPayload,
    ServiceSchedule, Slide, SlideContent, SlideId, Song, SongId, Theme, ThemeTarget,
};
use sundaydeck_db::{AppSettings, CcliProjectionInput};
use sundaydeck_overlay::{DEFAULT_OVERLAY_BIND_ADDR, OverlayPayload, OverlayServerHandle};
use sundaydeck_voice::VoiceCommand;

use crate::design::motion::MotionMode;
use crate::store::PresentationStore;

impl PresentationStore {
    /// Applies updated Church Branding, Display, Local AI, and Auto-Follow settings live
    /// and returns a clone ready for off-thread SQLite persistence.
    pub fn apply_settings(&mut self, settings: AppSettings, cx: &mut Context<Self>) -> AppSettings {
        self.motion_mode = match settings.motion_mode.as_str() {
            "crossfade" => MotionMode::CrossfadeOnly,
            "instant" => MotionMode::Instant,
            _ => MotionMode::Full,
        };
        let _ = self.dispatch(
            PresentationCommand::SetChurchName(settings.church_name.clone()),
            cx,
        );
        let _ = self.dispatch(
            PresentationCommand::SetLogoUri(settings.logo_uri.clone()),
            cx,
        );
        let _ = self.dispatch(
            PresentationCommand::SetDirectClickLive(settings.direct_click_live),
            cx,
        );
        self.forward_to_voice(VoiceCommand::UpdateConfig {
            asr_endpoint_url: settings.whisper_url.clone(),
            advance_confidence_threshold: f32::from(settings.voice_advance_threshold_pct) / 100.0,
            lock_during_override: settings.voice_lock_during_override,
        });
        self.settings = settings.clone();
        self.set_status("Saved SundayDeck settings", cx);
        settings
    }

    /// Advances an active timed Pre-Service Announcement Loop when its interval has elapsed.
    /// Automatically pauses while any screen override (`Black`, `Logo`, `ClearText`) is latched.
    pub fn tick_announcement_loop(&mut self, now_ms: i64, cx: &mut Context<Self>) -> bool {
        let snap = self.snapshot();
        if snap.output_override != OutputOverride::None {
            return false;
        }
        let Some(live) = snap.live.as_ref() else {
            return false;
        };
        let Some(interval_secs) = live.loop_interval_secs else {
            return false;
        };
        if live.slides.len() <= 1 {
            return false;
        }
        if self.last_loop_advance_ms == 0 {
            self.last_loop_advance_ms = now_ms;
            return false;
        }
        let due_ms = i64::from(interval_secs) * 1_000;
        if now_ms.saturating_sub(self.last_loop_advance_ms) >= due_ms {
            self.last_loop_advance_ms = now_ms;
            let _ = self.dispatch(PresentationCommand::AdvanceLiveLoop, cx);
            return true;
        }
        false
    }

    /// Appends a [`ServiceItem`] (e.g., PDF slide deck or Announcement Loop) to the active
    /// service order, optionally staging it immediately into Preview.
    pub fn add_service_item(
        &mut self,
        item: ServiceItem,
        stage_immediately: bool,
        cx: &mut Context<Self>,
    ) -> ServiceSchedule {
        let item_id = item.id;
        let title = item.title.clone();
        self.service.items.push(item);
        self.sync_active_service_in_list();
        if stage_immediately {
            self.activate_service_item(item_id, cx);
        }
        self.set_status(format!("Added \"{title}\" to Service Order"), cx);
        self.service.clone()
    }

    /// Removes a non-live item from the Service Order and returns the updated schedule.
    /// Protects operators from accidentally deleting the item currently on the projector.
    pub fn remove_service_item(
        &mut self,
        item_id: ServiceItemId,
        cx: &mut Context<Self>,
    ) -> Option<ServiceSchedule> {
        let live_item_id = self.snapshot().live.as_ref().and_then(|c| c.item_id);
        if live_item_id == Some(item_id) {
            self.set_status("Cannot remove the item currently live on screen", cx);
            return None;
        }
        let idx = self.service.items.iter().position(|i| i.id == item_id)?;
        let removed = self.service.items.remove(idx);
        if self.selected_service_item_id == Some(item_id) {
            self.selected_service_item_id = self.service.items.first().map(|i| i.id);
        }
        self.sync_active_service_in_list();
        self.set_status(
            format!("Removed \"{}\" from Service Order", removed.title),
            cx,
        );
        Some(self.service.clone())
    }

    /// Switches to another saved Sunday service schedule in memory and stages its first item.
    pub fn switch_service(&mut self, service_id: ServiceId, cx: &mut Context<Self>) {
        let Some(target) = self.services.iter().find(|s| s.id == service_id).cloned() else {
            return;
        };
        let title = target.title.clone();
        self.service = target;
        self.selected_service_item_id = self.service.items.first().map(|i| i.id);
        if let Some(first_id) = self.selected_service_item_id {
            self.activate_service_item(first_id, cx);
        }
        self.set_status(format!("Loaded service \"{title}\""), cx);
    }

    /// Creates a blank named Sunday service, selects it, and returns it for SQLite persistence.
    pub fn create_new_service(&mut self, title: String, cx: &mut Context<Self>) -> ServiceSchedule {
        let clean = title.trim();
        let name = if clean.is_empty() {
            format!("Sunday Service {}", self.services.len() + 1)
        } else {
            clean.to_owned()
        };
        let schedule = ServiceSchedule {
            id: ServiceId::new_v7(),
            title: name.clone(),
            service_date: None,
            notes: String::new(),
            items: Vec::new(),
        };
        self.services.insert(0, schedule.clone());
        self.service = schedule.clone();
        self.selected_service_item_id = None;
        self.set_status(format!("Created new service \"{name}\""), cx);
        schedule
    }

    /// Merges an extracted portable `.sdeck` service bundle into local memory (songs, themes,
    /// and service order) and returns the records to persist asynchronously in SQLite.
    pub fn apply_imported_sdeck(
        &mut self,
        bundle: SdeckBundle,
        cx: &mut Context<Self>,
    ) -> (Vec<Song>, Vec<Theme>, ServiceSchedule) {
        let imported_songs = bundle.songs;
        let imported_themes = bundle.themes;

        self.upsert_songs_sorted(imported_songs.iter().cloned());

        for theme in &imported_themes {
            let arc_t = Arc::new(theme.clone());
            if let Some(existing) = self.themes.iter_mut().find(|t| t.id == theme.id) {
                *existing = arc_t;
            } else {
                self.themes.push(arc_t);
            }
        }

        let service = bundle.service;
        self.service = service.clone();
        self.sync_active_service_in_list();
        if let Some(first) = self.service.items.first().map(|i| i.id) {
            self.activate_service_item(first, cx);
        }
        self.set_status(
            format!(
                "Imported .sdeck \"{}\" ({} items, {} songs, {} media)",
                service.title,
                service.items.len(),
                imported_songs.len(),
                bundle.extracted_media_files.len(),
            ),
            cx,
        );
        (imported_songs, imported_themes, service)
    }

    /// Merges a bulk song import (migration, Service Pack, welcome setup) into the library with a
    /// single re-sort and a single notify. Unlike [`Self::upsert_song_local`] it never stages a
    /// song, so a bulk import can never replace what is on the projector.
    pub fn merge_imported_songs(&mut self, songs: Vec<Song>, cx: &mut Context<Self>) {
        self.upsert_songs_sorted(songs);
        self.clear_song_filter();
        cx.notify();
    }

    /// The current song search hits (in rank order), or `None` when no search is active.
    pub fn filtered_song_ids(&self) -> Option<&[SongId]> {
        self.filtered_song_ids.as_deref()
    }

    /// Drops any song search so the library shows every song again.
    pub(crate) fn clear_song_filter(&mut self) {
        self.filtered_song_ids = None;
        self.filtered_song_indices = None;
        self.song_search_query.clear();
    }

    /// Shows `hit_ids` (in rank order) as the song library, or every song for `None`. Hits for
    /// songs that are not in memory (e.g. rows skipped at startup) are dropped.
    pub fn set_song_filter(
        &mut self,
        query: String,
        hit_ids: Option<Vec<SongId>>,
        cx: &mut Context<Self>,
    ) {
        self.filtered_song_ids = hit_ids;
        self.reindex_song_filter();
        self.song_search_query = query;
        cx.notify();
    }

    /// Resolves the search hits to positions in `songs`, dropping hits no longer in memory, so
    /// the virtualized list reads each visible row in O(1). Call after `songs` is reordered.
    fn reindex_song_filter(&mut self) {
        let Some(ids) = self.filtered_song_ids.as_mut() else {
            self.filtered_song_indices = None;
            return;
        };
        let index_by_id: HashMap<SongId, usize> = self
            .songs
            .iter()
            .enumerate()
            .map(|(ix, s)| (s.id, ix))
            .collect();
        ids.retain(|id| index_by_id.contains_key(id));
        self.filtered_song_indices = Some(
            ids.iter()
                .filter_map(|id| index_by_id.get(id).copied())
                .collect(),
        );
    }

    /// Inserts or replaces one song. An edit that keeps the title replaces it in place (no map,
    /// no re-sort); anything else goes through [`Self::upsert_songs_sorted`].
    pub(crate) fn upsert_song_sorted(&mut self, song: Song) {
        let same_title_slot = self
            .songs
            .iter_mut()
            .find(|s| s.id == song.id)
            .filter(|s| s.title == song.title);
        match same_title_slot {
            Some(existing) => *existing = song,
            None => self.upsert_songs_sorted([song]),
        }
    }

    /// Replaces songs with the same id, appends new ones, then sorts the library by title once
    /// (an edited title moves to its new place too).
    pub(crate) fn upsert_songs_sorted(&mut self, songs: impl IntoIterator<Item = Song>) {
        let mut index_by_id: HashMap<SongId, usize> = self
            .songs
            .iter()
            .enumerate()
            .map(|(ix, s)| (s.id, ix))
            .collect();
        for song in songs {
            match index_by_id
                .get(&song.id)
                .and_then(|&ix| self.songs.get_mut(ix))
            {
                Some(existing) => *existing = song,
                None => {
                    index_by_id.insert(song.id, self.songs.len());
                    self.songs.push(song);
                }
            }
        }
        self.songs.sort_by_cached_key(|s| s.title.to_lowercase());
        self.reindex_song_filter();
    }

    /// Registers a newly imported 66-book Bible translation (`.xml` or `.sqlite`) in memory
    /// and switches the Bible browser to it immediately.
    pub fn register_imported_bible(
        &mut self,
        translation: BibleTranslation,
        cx: &mut Context<Self>,
    ) {
        let label = format!("{} ({})", translation.name, translation.code);
        let code = translation.code.clone();
        if let Some(existing) = self
            .bible_translations
            .iter_mut()
            .find(|t| t.code.eq_ignore_ascii_case(&code))
        {
            *existing = translation;
        } else {
            self.bible_translations.push(translation);
        }
        self.active_translation_code = code;
        self.set_status(format!("Imported offline Bible: {label}"), cx);
    }

    /// Updates or adds a Bible translation installed by background auto-sync without switching
    /// away from the operator's currently selected translation.
    pub fn register_synced_bible(&mut self, translation: BibleTranslation, cx: &mut Context<Self>) {
        let code = translation.code.clone();
        if let Some(existing) = self
            .bible_translations
            .iter_mut()
            .find(|t| t.code.eq_ignore_ascii_case(&code))
        {
            *existing = translation;
        } else {
            self.bible_translations.push(translation);
        }
        if self.active_translation_code.is_empty() {
            self.active_translation_code = code;
        }
        cx.notify();
    }

    /// Starts or stops the lightweight local OBS Browser Source HTTP + WebSocket overlay server.
    pub fn toggle_obs_overlay(&mut self, cx: &mut Context<Self>) {
        if let Some(handle) = self.overlay_server.take() {
            handle.shutdown();
            self.set_status("OBS Broadcast Overlay stopped", cx);
            return;
        }
        match OverlayServerHandle::start(DEFAULT_OVERLAY_BIND_ADDR) {
            Ok(handle) => {
                let url = handle.overlay_url();
                let remote = handle.remote_url();
                handle.publish(&OverlayPayload::from_snapshot(&self.snapshot()));
                crate::remote_control::spawn_remote_control_listener(&handle, cx);
                self.overlay_server = Some(handle);
                self.set_status(format!("OBS Overlay ({url}) & Remote ({remote}) live"), cx);
            }
            Err(err) => {
                self.set_status(format!("Could not start OBS Overlay: {err}"), cx);
            }
        }
    }

    /// Saves a structured Custom Slide into the reusable library, stages it in Preview,
    /// and optionally appends it to the active Service Order.
    pub fn upsert_custom_slide(
        &mut self,
        spec: CustomSlideSpec,
        add_to_service: bool,
        auto_advance_secs: Option<u16>,
        cx: &mut Context<Self>,
    ) -> Option<ServiceSchedule> {
        let id = spec.id;
        if let Some(existing) = self.custom_slides.iter_mut().find(|s| s.id == id) {
            *existing = spec.clone();
        } else {
            self.custom_slides.insert(0, spec.clone());
        }
        if add_to_service {
            let item = ServiceItem {
                id: ServiceItemId::new_v7(),
                title: spec.label.clone(),
                theme_override_id: None,
                payload: ServiceItemPayload::CustomDeck {
                    slides: vec![spec],
                    auto_advance_secs,
                },
            };
            return Some(self.add_service_item(item, true, cx));
        }
        self.stage_custom_slide(id, cx);
        None
    }

    /// The library's custom slide with `id`, if any.
    pub fn custom_slide_by_id(&self, id: SlideId) -> Option<&CustomSlideSpec> {
        self.custom_slides.iter().find(|s| s.id == id)
    }

    /// Stages a reusable structured Custom Slide directly into Preview (or Live if direct-click is on).
    pub fn stage_custom_slide(&mut self, slide_id: SlideId, cx: &mut Context<Self>) {
        let Some(spec) = self.custom_slide_by_id(slide_id).cloned() else {
            return;
        };
        let theme = self
            .default_theme_for(ThemeTarget::General)
            .unwrap_or_else(|| self.active_theme());
        let slide = Slide {
            id: spec.id,
            source_section_id: None,
            label: spec.label.clone(),
            content: SlideContent::Custom {
                revealed_steps: spec.layout.total_reveal_steps().map(|_| 1),
                layout: spec.layout,
            },
            transition_override: spec.transition_override,
        };
        if let Ok(deck) = DeckCursor::try_new(None, spec.label, vec![slide], theme) {
            let cmd = if self.snapshot().direct_click_live {
                PresentationCommand::GoLiveDirect(deck)
            } else {
                PresentationCommand::StagePreview(deck)
            };
            let _ = self.dispatch(cmd, cx);
        }
    }

    /// Removes a custom slide from memory by id and returns whether one was removed.
    pub fn remove_custom_slide(&mut self, id: SlideId, cx: &mut Context<Self>) -> bool {
        let before = self.custom_slides.len();
        self.custom_slides.retain(|s| s.id != id);
        let removed = self.custom_slides.len() != before;
        if removed {
            self.set_status("Removed custom slide from library", cx);
        }
        removed
    }

    /// Generates a safe `.sdeck` export filename for the active Sunday service.
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

    /// Builds a [`CcliProjectionInput`] if the currently live deck corresponds to a song.
    pub fn live_song_ccli_entry(&self) -> Option<CcliProjectionInput> {
        let live = self.snapshot().live.as_ref()?.clone();
        let song = if let Some(item_id) = live.item_id {
            let item = self.service.items.iter().find(|i| i.id == item_id)?;
            match item.payload {
                ServiceItemPayload::Song { song_id, .. } => self.song_by_id(song_id)?,
                _ => return None,
            }
        } else {
            self.songs.iter().find(|s| s.title == live.title)?
        };
        Some(CcliProjectionInput {
            song_id: Some(song.id),
            title: song.title.clone(),
            author: song.author.clone(),
            ccli_number: song.ccli_number.clone(),
            copyright: song.copyright.clone(),
            projected_at: self.now_epoch_ms(),
        })
    }

    /// Removes a spoken Bible reference chip from the 1-Click Auto-Follow Scripture Queue.
    pub fn dismiss_queued_scripture(&mut self, reference: ScriptureRef, cx: &mut Context<Self>) {
        self.queued_scriptures.retain(|r| *r != reference);
        cx.notify();
    }

    fn sync_active_service_in_list(&mut self) {
        if let Some(existing) = self.services.iter_mut().find(|s| s.id == self.service.id) {
            *existing = self.service.clone();
        } else {
            self.services.insert(0, self.service.clone());
        }
    }
}
