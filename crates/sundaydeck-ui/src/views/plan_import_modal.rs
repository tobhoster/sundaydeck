use std::path::PathBuf;

use gpui::{
    AppContext as _, Context, Entity, EventEmitter, IntoElement, PathPromptOptions, Render, Window,
};
use sundaydeck_core::{GeneratedServicePlan, PlanApplyMode, sample_pastor_sermon_outline};
use sundaydeck_db::DbWorker;

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
/// Parses uploaded files off-thread on `DbWorker`, matches library songs & SQLite Bible
/// passages, auto-paginates dense sermon copy, and lets volunteers review before applying.
pub struct PlanImportModal {
    pub(crate) store: Entity<PresentationStore>,
    db: Option<DbWorker>,
    pub(crate) active_tab: PlanInputTab,
    pub(crate) path_input: Entity<TextInput>,
    pub(crate) outline_input: Entity<TextInput>,
    pub(crate) plan: Option<GeneratedServicePlan>,
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
            path_input,
            outline_input,
            plan: None,
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
        self.loading = true;
        self.error_message = None;
        cx.notify();

        let translation = self.store.read(cx).active_translation_code.clone();
        if let Some(db) = self.db.clone() {
            cx.spawn(async move |this, cx| {
                let res = db.generate_plan_from_file(path, &translation).await;
                let _ = this.update(cx, |modal, cx| {
                    modal.loading = false;
                    match res {
                        Ok(plan) => {
                            modal.plan = Some(plan);
                            modal.selected_item_idx = 0;
                            modal.error_message = None;
                        }
                        Err(err) => {
                            modal.error_message = Some(err.to_string());
                        }
                    }
                    cx.notify();
                });
            })
            .detach();
        } else {
            self.loading = false;
            match sundaydeck_core::extract_office_file(&path) {
                Ok(doc) => {
                    let store_ref = self.store.read(cx);
                    let themes: Vec<_> = store_ref.themes.iter().map(|t| (**t).clone()).collect();
                    let plan = sundaydeck_core::build_service_plan(
                        &doc,
                        &store_ref.songs,
                        &themes,
                        &translation,
                        |_| Vec::new(),
                    );
                    self.plan = Some(plan);
                    self.selected_item_idx = 0;
                }
                Err(err) => self.error_message = Some(err.to_string()),
            }
            cx.notify();
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
        self.loading = true;
        self.error_message = None;
        cx.notify();

        let translation = self.store.read(cx).active_translation_code.clone();
        if let Some(db) = self.db.clone() {
            cx.spawn(async move |this, cx| {
                let res = db
                    .generate_plan_from_text(&source_name, &text, &translation)
                    .await;
                let _ = this.update(cx, |modal, cx| {
                    modal.loading = false;
                    match res {
                        Ok(plan) => {
                            modal.plan = Some(plan);
                            modal.selected_item_idx = 0;
                            modal.error_message = None;
                        }
                        Err(err) => modal.error_message = Some(err.to_string()),
                    }
                    cx.notify();
                });
            })
            .detach();
        } else {
            self.loading = false;
            match self
                .store
                .read(cx)
                .build_plan_from_text_local(&source_name, &text)
            {
                Ok(plan) => {
                    self.plan = Some(plan);
                    self.selected_item_idx = 0;
                }
                Err(err) => self.error_message = Some(err.to_string()),
            }
            cx.notify();
        }
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
