use std::path::PathBuf;

use gpui::{
    AppContext as _, Context, Entity, EventEmitter, IntoElement, PathPromptOptions, Render, Window,
};
use sundaydeck_core::{
    ExtractedDocument, GeneratedServicePlan, PlanApplyMode, extract_office_file,
    extract_plain_outline, sample_pastor_sermon_outline,
};
use sundaydeck_db::DbWorker;
use sundaydeck_planner::benchmark_corpus::lookup_starter_web;
use sundaydeck_planner::{
    LocalLlmConfig, LocalLlmPreset, PlanRunMetrics, PlannerConfig, PlannerEngineKind, run_planner,
};

use crate::components::TextInput;
use crate::store::PresentationStore;
use crate::views::plan_import_ui::render_plan_import_modal;
use crate::views::quick_edit_modal::ModalClosed;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanInputTab {
    FileUpload,
    PasteOutline,
}

/// Offline Pastor PowerPoint (`.pptx`) / Word (`.docx`) / Outline Auto-Planner modal.
/// Supports switching head-to-head between pure-Rust No-LLM rules, a Local LLM sidecar,
/// and an Embedded in-process GGUF model, always running off-thread so live playback stays 60fps.
pub struct PlanImportModal {
    pub(crate) store: Entity<PresentationStore>,
    db: Option<DbWorker>,
    pub(crate) active_tab: PlanInputTab,
    pub(crate) engine: PlannerEngineKind,
    pub(crate) local_model_preset: String,
    pub(crate) path_input: Entity<TextInput>,
    pub(crate) outline_input: Entity<TextInput>,
    pub(crate) last_doc: Option<ExtractedDocument>,
    pub(crate) plan: Option<GeneratedServicePlan>,
    pub(crate) last_metrics: Option<PlanRunMetrics>,
    pub(crate) selected_item_idx: usize,
    pub(crate) apply_mode: PlanApplyMode,
    pub(crate) loading: bool,
    pub(crate) error_message: Option<String>,
}

impl EventEmitter<ModalClosed> for PlanImportModal {}

impl PlanImportModal {
    pub fn new(
        store: Entity<PresentationStore>,
        db: Option<DbWorker>,
        cx: &mut Context<Self>,
    ) -> Self {
        let path_input = cx.new(|cx| {
            TextInput::single_line(
                "plan-file-path",
                "/path/to/sunday-sermon.pptx or pastor-notes.docx",
                "",
                cx,
            )
        });
        let outline_input = cx.new(|cx| {
            TextInput::multiline(
                "plan-outline-text",
                "Paste sermon outline (# Headings, Song: Title, Romans 8:28, Note: Stage cue)...",
                "",
                cx,
            )
        });
        Self {
            store,
            db,
            active_tab: PlanInputTab::FileUpload,
            engine: PlannerEngineKind::Deterministic,
            local_model_preset: LocalLlmPreset::ALL[0].model_name.to_owned(),
            path_input,
            outline_input,
            last_doc: None,
            plan: None,
            last_metrics: None,
            selected_item_idx: 0,
            apply_mode: PlanApplyMode::ReplaceSchedule,
            loading: false,
            error_message: None,
        }
    }

    pub fn set_plan_for_preview(&mut self, plan: GeneratedServicePlan, cx: &mut Context<Self>) {
        self.plan = Some(plan);
        self.selected_item_idx = 0;
        self.loading = false;
        self.error_message = None;
        cx.notify();
    }

    pub fn set_engine_for_preview(&mut self, engine: PlannerEngineKind, cx: &mut Context<Self>) {
        self.engine = engine;
        cx.notify();
    }

    pub fn set_plan_with_metrics(
        &mut self,
        plan: GeneratedServicePlan,
        metrics: PlanRunMetrics,
        cx: &mut Context<Self>,
    ) {
        self.engine = metrics.engine;
        self.last_metrics = Some(metrics);
        self.set_plan_for_preview(plan, cx);
    }

    pub(crate) fn select_local_model(&mut self, model: String, cx: &mut Context<Self>) {
        self.local_model_preset = model;
        if self.engine == PlannerEngineKind::LocalLlm
            && let Some(doc) = self.last_doc.clone()
        {
            self.run_engine_on_doc(doc, cx);
        } else {
            cx.notify();
        }
    }

    pub(crate) fn select_engine(&mut self, engine: PlannerEngineKind, cx: &mut Context<Self>) {
        if self.engine == engine {
            return;
        }
        self.engine = engine;
        if let Some(doc) = self.last_doc.clone() {
            self.run_engine_on_doc(doc, cx);
        } else {
            cx.notify();
        }
    }

    pub(crate) fn browse_office_file(&mut self, cx: &mut Context<Self>) {
        let rx = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Select Pastor PowerPoint (.pptx) or Word (.docx)".into()),
        });
        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(paths))) = rx.await
                && let Some(path) = paths.into_iter().next()
            {
                let path_str = path.display().to_string();
                let _ = this.update(cx, |modal, cx| {
                    modal
                        .path_input
                        .update(cx, |inp, cx| inp.set_value(&path_str, cx));
                    modal.parse_file_path(path, cx);
                });
            }
        })
        .detach();
    }

    pub(crate) fn parse_typed_path(&mut self, cx: &mut Context<Self>) {
        let raw = self.path_input.read(cx).value().trim().to_owned();
        if raw.is_empty() {
            self.error_message = Some("Enter or browse for a .pptx, .docx, or .md file".to_owned());
            cx.notify();
            return;
        }
        self.parse_file_path(PathBuf::from(raw), cx);
    }

    pub(crate) fn parse_file_path(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        match extract_office_file(&path) {
            Ok(doc) => {
                self.last_doc = Some(doc.clone());
                self.run_engine_on_doc(doc, cx);
            }
            Err(err) => {
                self.error_message = Some(err.to_string());
                cx.notify();
            }
        }
    }

    pub(crate) fn load_sample_sermon(&mut self, cx: &mut Context<Self>) {
        let sample = sample_pastor_sermon_outline();
        self.outline_input
            .update(cx, |inp, cx| inp.set_value(sample, cx));
        self.parse_outline_text("walking-in-peace.docx".to_owned(), sample.to_owned(), cx);
    }

    pub(crate) fn parse_pasted_outline(&mut self, cx: &mut Context<Self>) {
        let text = self.outline_input.read(cx).value().to_owned();
        self.parse_outline_text("sermon-outline.md".to_owned(), text, cx);
    }

    fn parse_outline_text(&mut self, source_name: String, text: String, cx: &mut Context<Self>) {
        match extract_plain_outline(&source_name, &text) {
            Ok(doc) => {
                self.last_doc = Some(doc.clone());
                self.run_engine_on_doc(doc, cx);
            }
            Err(err) => {
                self.error_message = Some(err.to_string());
                cx.notify();
            }
        }
    }

    fn run_engine_on_doc(&mut self, doc: ExtractedDocument, cx: &mut Context<Self>) {
        self.loading = true;
        self.error_message = None;
        cx.notify();

        let engine = self.engine;
        let local_model = self.local_model_preset.clone();
        let (songs, themes, translation) = {
            let store_ref = self.store.read(cx);
            (
                store_ref.songs.clone(),
                store_ref
                    .themes
                    .iter()
                    .map(|t| (**t).clone())
                    .collect::<Vec<_>>(),
                store_ref.active_translation_code.clone(),
            )
        };

        let (tx, rx) = async_channel::bounded(1);
        std::thread::spawn(move || {
            let cfg = PlannerConfig {
                engine,
                local_llm: LocalLlmConfig {
                    model: local_model,
                    ..LocalLlmConfig::default()
                },
                ..PlannerConfig::default()
            };
            let res = run_planner(
                &cfg,
                &doc,
                &songs,
                &themes,
                &translation,
                lookup_starter_web,
            );
            let _ = tx.send_blocking(res);
        });

        cx.spawn(async move |this, cx| {
            let res = rx.recv().await;
            let _ = this.update(cx, |modal, cx| {
                modal.loading = false;
                match res {
                    Ok(Ok(outcome)) => {
                        modal.plan = Some(outcome.plan);
                        modal.last_metrics = Some(outcome.metrics);
                        modal.selected_item_idx = 0;
                        modal.error_message = None;
                    }
                    Ok(Err(err)) => modal.error_message = Some(err.to_string()),
                    Err(_) => modal.error_message = Some("Planner worker terminated".to_owned()),
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(crate) fn toggle_item_enabled(&mut self, idx: usize, cx: &mut Context<Self>) {
        if let Some(plan) = &mut self.plan
            && let Some(item) = plan.items.get_mut(idx)
        {
            item.enabled = !item.enabled;
            cx.notify();
        }
    }

    pub(crate) fn apply_plan(&mut self, cx: &mut Context<Self>) {
        let Some(plan) = self.plan.clone() else {
            return;
        };
        let mode = self.apply_mode;
        let saved = self
            .store
            .update(cx, |s, cx| s.apply_generated_plan(&plan, mode, cx));

        if let (Some(schedule), Some(db)) = (saved, self.db.clone()) {
            cx.spawn(async move |_, _| {
                let _ = db.save_service(schedule).await;
            })
            .detach();
            cx.emit(ModalClosed);
        }
    }
}

impl Render for PlanImportModal {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        render_plan_import_modal(self, cx)
    }
}
