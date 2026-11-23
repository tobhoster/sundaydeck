use std::sync::Arc;

use gpui::Context;
use sundaydeck_core::{
    BibleTranslation, OutputOverride, PresentationCommand, SdeckBundle, ServiceId, ServiceItem,
    ServiceItemId, ServiceSchedule, Song, Theme,
};
use sundaydeck_db::AppSettings;
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
        if let Some(engine) = &self.voice_engine {
            engine.send(VoiceCommand::UpdateConfig {
                asr_endpoint_url: settings.whisper_url.clone(),
                advance_confidence_threshold: f32::from(settings.voice_advance_threshold_pct)
                    / 100.0,
                lock_during_override: settings.voice_lock_during_override,
            });
        }
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
        let imported_songs = bundle.songs.clone();
        let imported_themes = bundle.themes.clone();

        for song in &imported_songs {
            if let Some(existing) = self.songs.iter_mut().find(|s| s.id == song.id) {
                *existing = song.clone();
            } else {
                self.songs.push(song.clone());
            }
        }
        self.songs.sort_by_key(|s| s.title.to_lowercase());

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

    fn sync_active_service_in_list(&mut self) {
        if let Some(existing) = self.services.iter_mut().find(|s| s.id == self.service.id) {
            *existing = self.service.clone();
        } else {
            self.services.insert(0, self.service.clone());
        }
    }
}
