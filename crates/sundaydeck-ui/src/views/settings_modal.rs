use gpui::{
    AppContext as _, Context, Entity, EventEmitter, InteractiveElement as _, IntoElement,
    ParentElement as _, PathPromptOptions, Render, StatefulInteractiveElement as _, Styled as _,
    Window, div,
};
use sundaydeck_core::ServiceId;
use sundaydeck_db::DbWorker;

use crate::components::TextInput;
use crate::deck_import::copy_church_logo_file;
use crate::design::icons::DeckIcon;
use crate::design::primitives::sculpted_secondary_button_body;
use crate::design::tokens::TOKENS;
use crate::store::PresentationStore;
use crate::updater::{CURRENT_VERSION, UpdateCheckOutcome, check_for_updates_blocking};
use crate::views::quick_edit_modal::ModalClosed;
use crate::views::settings_ui::render_settings_modal;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsTab {
    ChurchDisplay,
    AiVoice,
    BiblesBundles,
}

pub const MOTION_CHOICES: &[(&str, &str)] = &[
    ("full", "Smooth Motion"),
    ("crossfade", "Calm Crossfade"),
    ("instant", "Instant Cut"),
];

pub const LLM_PRESET_CHOICES: &[(&str, &str)] = &[
    ("qwen3-4b", "Qwen3 4B"),
    ("qwen2.5-3b", "Qwen2.5 3B"),
    ("phi4-mini", "Phi-4 Mini"),
    ("gemma3-4b", "Gemma 3 4B"),
];

pub const VOICE_THRESHOLD_CHOICES: &[(u8, &str)] = &[
    (60, "60% · Responsive"),
    (70, "70% · Balanced"),
    (80, "80% · Strict"),
];

pub const LOOP_INTERVAL_CHOICES: &[u16] = &[5, 6, 8, 12];

pub const CHURCH_PROFILE_CHOICES: &[(&str, &str)] = &[
    ("contemporary", "Contemporary"),
    ("baptist_evangelical", "Baptist & Evangelical"),
    ("methodist_mainline", "Methodist / Mainline"),
    ("catholic_parish", "Catholic Parish (73-Book)"),
    ("spanish_bilingual", "Spanish / Portuguese"),
];

pub const BILINGUAL_MODE_CHOICES: &[(&str, &str)] = &[
    ("bilingual_stacked", "Bilingual Stacked"),
    ("primary_only", "Primary Only"),
    ("secondary_only", "Secondary Only"),
];

/// Full Church Branding, Display Motion, Service Assistant & Voice, Offline Bible Importer,
/// Global Church Style & `.sdeck-pack` Packs, and Portable `.sdeck` Service Bundles modal.
pub struct SettingsModal {
    pub(crate) store: Entity<PresentationStore>,
    pub(crate) db: Option<DbWorker>,
    pub(crate) active_tab: SettingsTab,
    pub(crate) church_name_input: Entity<TextInput>,
    pub(crate) logo_uri: Option<String>,
    pub(crate) motion_mode: String,
    pub(crate) direct_click_live: bool,
    pub(crate) llm_url_input: Entity<TextInput>,
    pub(crate) llm_model_preset: String,
    pub(crate) whisper_url_input: Entity<TextInput>,
    pub(crate) voice_threshold_pct: u8,
    pub(crate) voice_lock_during_override: bool,
    pub(crate) church_profile: String,
    pub(crate) bilingual_mode: String,
    pub(crate) enable_catholic_canon: bool,
    pub(crate) loop_interval_secs: u16,
    pub(crate) new_service_input: Entity<TextInput>,
    pub(crate) status_note: Option<String>,
    pub(crate) error_message: Option<String>,
    pub(crate) update_url: Option<(String, String)>,
    pub(crate) busy: bool,
}

impl EventEmitter<ModalClosed> for SettingsModal {}

impl SettingsModal {
    pub fn new(
        store: Entity<PresentationStore>,
        db: Option<DbWorker>,
        initial_tab: SettingsTab,
        cx: &mut Context<Self>,
    ) -> Self {
        let cur = store.read(cx).settings.clone();
        let church_name_input = cx.new(|cx| {
            TextInput::single_line(
                "settings-church-name",
                "Church or Ministry Name",
                &cur.church_name,
                cx,
            )
        });
        let llm_url_input = cx.new(|cx| {
            TextInput::single_line(
                "settings-llm-url",
                "http://127.0.0.1:8080/v1/chat/completions",
                &cur.local_llm_url,
                cx,
            )
        });
        let whisper_url_input = cx.new(|cx| {
            TextInput::single_line(
                "settings-whisper-url",
                "http://127.0.0.1:8081/inference",
                &cur.whisper_url,
                cx,
            )
        });
        let new_service_input = cx.new(|cx| {
            TextInput::single_line(
                "settings-new-service-title",
                "e.g. Next Sunday Morning — 10:30 AM",
                "",
                cx,
            )
        });

        Self {
            store,
            db,
            active_tab: initial_tab,
            church_name_input,
            logo_uri: cur.logo_uri,
            motion_mode: cur.motion_mode,
            direct_click_live: cur.direct_click_live,
            llm_url_input,
            llm_model_preset: cur.local_llm_model,
            whisper_url_input,
            voice_threshold_pct: cur.voice_advance_threshold_pct,
            voice_lock_during_override: cur.voice_lock_during_override,
            church_profile: cur.church_profile,
            bilingual_mode: cur.bilingual_mode,
            enable_catholic_canon: cur.enable_catholic_canon,
            loop_interval_secs: 6,
            new_service_input,
            status_note: None,
            error_message: None,
            update_url: None,
            busy: false,
        }
    }

    pub(crate) fn save_and_close(&mut self, cx: &mut Context<Self>) {
        let mut settings = self.store.read(cx).settings.clone();
        let church_name = self.church_name_input.read(cx).value().trim().to_owned();
        if !church_name.is_empty() {
            settings.church_name = church_name;
        }
        settings.logo_uri = self.logo_uri.clone();
        settings.motion_mode = self.motion_mode.clone();
        settings.direct_click_live = self.direct_click_live;
        let llm_url = self.llm_url_input.read(cx).value().trim().to_owned();
        if !llm_url.is_empty() {
            settings.local_llm_url = llm_url;
        }
        settings.local_llm_model = self.llm_model_preset.clone();
        let whisper_url = self.whisper_url_input.read(cx).value().trim().to_owned();
        if !whisper_url.is_empty() {
            settings.whisper_url = whisper_url;
        }
        settings.voice_advance_threshold_pct = self.voice_threshold_pct;
        settings.voice_lock_during_override = self.voice_lock_during_override;
        settings.church_profile = self.church_profile.clone();
        settings.bilingual_mode = self.bilingual_mode.clone();
        settings.enable_catholic_canon = self.enable_catholic_canon;

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

    pub(crate) fn browse_church_logo(&mut self, cx: &mut Context<Self>) {
        let rx = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Select Church Logo Image (.png, .jpg, .webp)".into()),
        });
        cx.spawn(async move |this, cx| {
            let Ok(Ok(Some(paths))) = rx.await else {
                return;
            };
            let Some(src) = paths.into_iter().next() else {
                return;
            };
            let result = cx
                .background_executor()
                .spawn(async move { copy_church_logo_file(&src) })
                .await;
            let _ = this.update(cx, |modal, cx| {
                match result {
                    Ok(saved) => {
                        modal.logo_uri = Some(saved.to_string_lossy().into_owned());
                        modal.error_message = None;
                        modal.status_note = Some(
                            "Logo loaded — click Save Settings to apply to Show Logo (F3)."
                                .to_owned(),
                        );
                    }
                    Err(err) => {
                        modal.error_message = Some(err);
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(crate) fn create_new_service(&mut self, cx: &mut Context<Self>) {
        let title = self.new_service_input.read(cx).value().trim().to_owned();
        let created = self
            .store
            .update(cx, |s, cx| s.create_new_service(title, cx));
        self.new_service_input
            .update(cx, |inp, cx| inp.set_value("", cx));
        self.status_note = Some(format!("Switched to new service \"{}\".", created.title));
        self.error_message = None;
        if let Some(db) = self.db.clone() {
            cx.spawn(async move |_, _| {
                let _ = db.save_service(created).await;
            })
            .detach();
        }
        cx.notify();
    }

    pub(crate) fn switch_service(&mut self, service_id: ServiceId, cx: &mut Context<Self>) {
        self.store
            .update(cx, |s, cx| s.switch_service(service_id, cx));
        let title = self.store.read(cx).service.title.clone();
        self.status_note = Some(format!("Active service: \"{title}\"."));
        self.error_message = None;
        cx.notify();
    }

    pub(crate) fn check_for_updates(&mut self, cx: &mut Context<Self>) {
        self.busy = true;
        self.error_message = None;
        self.status_note = Some("Checking GitHub Releases for updates…".to_owned());
        cx.notify();
        cx.spawn(async move |this, cx| {
            let outcome = cx
                .background_executor()
                .spawn(async move { check_for_updates_blocking() })
                .await;
            let _ = this.update(cx, |m, cx| {
                m.busy = false;
                match outcome {
                    UpdateCheckOutcome::UpToDate { current } => {
                        m.update_url = None;
                        m.status_note = Some(format!(
                            "Up to date — SundayDeck v{current} is the latest release."
                        ));
                    }
                    UpdateCheckOutcome::UpdateAvailable {
                        latest,
                        download_url,
                        ..
                    } => {
                        m.status_note = Some(format!(
                            "SundayDeck v{latest} is available — click Download Update."
                        ));
                        m.update_url = Some((latest, download_url));
                    }
                    UpdateCheckOutcome::OfflineOrError(msg) => {
                        m.status_note = Some(msg);
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(crate) fn render_update_action(&self, cx: &mut Context<Self>) -> impl IntoElement {
        if let Some((ver, url)) = self.update_url.clone() {
            div()
                .id("settings-download-update")
                .cursor_pointer()
                .on_click(cx.listener(move |_, _, _, cx| cx.open_url(&url)))
                .child(sculpted_secondary_button_body(
                    Some(DeckIcon::UploadSimple),
                    &format!("Download v{ver}"),
                    TOKENS.signal_voice,
                ))
        } else {
            div()
                .id("settings-check-updates")
                .cursor_pointer()
                .on_click(cx.listener(|this, _, _, cx| this.check_for_updates(cx)))
                .child(sculpted_secondary_button_body(
                    Some(DeckIcon::UploadSimple),
                    &format!("v{CURRENT_VERSION} · Check for Updates"),
                    TOKENS.ink_secondary,
                ))
        }
    }
}

impl Render for SettingsModal {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        render_settings_modal(self, cx)
    }
}
