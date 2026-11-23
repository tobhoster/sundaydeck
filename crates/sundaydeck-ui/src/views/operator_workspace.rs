use std::time::Duration;

use gpui::{
    AnyElement, App, AppContext as _, Context, Entity, FocusHandle, Focusable, InteractiveElement,
    IntoElement, ParentElement, Render, Styled, Subscription, Window, div, rgba,
};
use sundaydeck_core::{
    GeneratedServicePlan, OutputOverride, PresentationCommand, ServiceItemId, SlideContent, Song,
    SongId,
};
use sundaydeck_db::DbWorker;

use crate::actions::{
    ClearStageMessage, GoLive, HardCutBlack, HardCutClearText, HardCutLogo, NextLiveSlide,
    PrevLiveSlide, QuitApp, ToggleAudienceWindow, ToggleBlack, ToggleClearText,
    ToggleDirectClickLive, ToggleLogo, ToggleQuickEdit, ToggleStageWindow,
};
use crate::components::{TextInput, TextInputEvent};
use crate::design::motion::modal_enter;
use crate::design::tokens::{FONT_BODY, TOKENS};
use crate::store::PresentationStore;
use crate::views::alerts_timers_modal::AlertsTimersModal;
use crate::views::bible_browser::BibleBrowserView;
use crate::views::library_preview_pane::render_library_preview_pane;
use crate::views::live_pane::render_live_pane;
use crate::views::operator_header::render_operator_header;
use crate::views::plan_import_modal::PlanImportModal;
use crate::views::quick_edit_modal::{ModalClosed, QuickEditModal};
use crate::views::schedule_pane::render_schedule_pane;
use crate::views::song_editor_modal::{EditorTab, SongEditorModal};
use crate::windows::WindowManager;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LibraryTab {
    Songs,
    Bible,
    Themes,
}

pub enum ActiveModal {
    SongEditor(Entity<SongEditorModal>),
    QuickEdit(Entity<QuickEditModal>),
    AlertsTimers(Entity<AlertsTimersModal>),
    PlanImporter(Entity<PlanImportModal>),
}

/// Primary 3-Pane Operator Workspace composing Service Schedule (Left),
/// Preview Grid & Library/Bible (Center), Live Control & Overrides (Right),
/// and non-blocking overlay modals for Song Editing, Quick Live Edit, and Alerts/Timers.
pub struct OperatorWorkspace {
    pub store: Entity<PresentationStore>,
    pub windows: Entity<WindowManager>,
    pub db: Option<DbWorker>,
    pub active_library_tab: LibraryTab,
    pub song_search_input: Entity<TextInput>,
    pub bible_browser: Entity<BibleBrowserView>,
    pub active_modal: Option<ActiveModal>,
    modal_sub: Option<Subscription>,
    focus_handle: FocusHandle,
    _store_sub: Subscription,
    _windows_sub: Subscription,
    _song_search_sub: Subscription,
}

impl OperatorWorkspace {
    pub fn new(
        store: Entity<PresentationStore>,
        windows: Entity<WindowManager>,
        db: Option<DbWorker>,
        cx: &mut Context<Self>,
    ) -> Self {
        let store_sub = cx.observe(&store, |_, _, cx| cx.notify());
        let windows_sub = cx.observe(&windows, |_, _, cx| cx.notify());

        let song_search_input = cx.new(|cx| {
            TextInput::single_line(
                "songs-fts-search",
                "Search songs by title, lyrics, author, or CCLI #...",
                "",
                cx,
            )
        });
        let song_search_sub = cx.subscribe(&song_search_input, Self::on_song_search);

        let bible_store = store.clone();
        let bible_db = db.clone();
        let bible_browser = cx.new(|cx| BibleBrowserView::new(bible_store, bible_db, cx));

        // 1-second background heartbeat for live timer display updates & auto-dismissing alerts.
        let tick_store = store.downgrade();
        cx.spawn(async move |_, cx| {
            loop {
                cx.background_executor().timer(Duration::from_secs(1)).await;
                let keep_going = tick_store
                    .update(cx, |s, cx| {
                        let snap = s.snapshot();
                        let now = s.now_epoch_ms();
                        if snap
                            .active_alert
                            .as_ref()
                            .is_some_and(|a| a.is_expired(now))
                        {
                            let _ = s.dispatch(
                                PresentationCommand::ExpireAlertIfDue { now_ms: now },
                                cx,
                            );
                        } else if snap.active_timer.as_ref().is_some_and(|t| t.running)
                            || snap
                                .active_alert
                                .as_ref()
                                .and_then(|a| a.expires_at_ms)
                                .is_some()
                        {
                            cx.notify();
                        }
                    })
                    .is_ok();
                if !keep_going {
                    break;
                }
            }
        })
        .detach();

        Self {
            store,
            windows,
            db,
            active_library_tab: LibraryTab::Songs,
            song_search_input,
            bible_browser,
            active_modal: None,
            modal_sub: None,
            focus_handle: cx.focus_handle(),
            _store_sub: store_sub,
            _windows_sub: windows_sub,
            _song_search_sub: song_search_sub,
        }
    }

    pub fn open_song_editor(&mut self, song: Option<Song>, tab: EditorTab, cx: &mut Context<Self>) {
        let store = self.store.clone();
        let db = self.db.clone();
        let modal = cx.new(|cx| SongEditorModal::open_for_song(store, db, song, tab, cx));
        self.modal_sub = Some(cx.subscribe(&modal, |this, _, _: &ModalClosed, cx| {
            this.close_modal(cx);
        }));
        self.active_modal = Some(ActiveModal::SongEditor(modal));
        cx.notify();
    }

    pub fn open_quick_edit(&mut self, cx: &mut Context<Self>) {
        let Some((slide_id, label, body)) = self
            .store
            .read(cx)
            .snapshot()
            .live
            .as_ref()
            .and_then(|cursor| cursor.active_slide())
            .and_then(|slide| match &slide.content {
                SlideContent::Text { body, .. } => {
                    Some((slide.id, slide.label.clone(), body.clone()))
                }
                SlideContent::Image { .. } | SlideContent::Blank => None,
            })
        else {
            self.store.update(cx, |s, cx| {
                s.set_status("No live text slide to quick-edit", cx)
            });
            return;
        };

        let store = self.store.clone();
        let db = self.db.clone();
        let modal = cx.new(|cx| QuickEditModal::new(store, db, slide_id, label, body, cx));
        self.modal_sub = Some(cx.subscribe(&modal, |this, _, _: &ModalClosed, cx| {
            this.close_modal(cx);
        }));
        self.active_modal = Some(ActiveModal::QuickEdit(modal));
        cx.notify();
    }

    pub fn open_alerts_timers(&mut self, cx: &mut Context<Self>) {
        let store = self.store.clone();
        let modal = cx.new(|cx| AlertsTimersModal::new(store, cx));
        self.modal_sub = Some(cx.subscribe(&modal, |this, _, _: &ModalClosed, cx| {
            this.close_modal(cx);
        }));
        self.active_modal = Some(ActiveModal::AlertsTimers(modal));
        cx.notify();
    }

    pub fn open_plan_importer(&mut self, cx: &mut Context<Self>) {
        self.open_plan_importer_with_plan(None, cx);
    }

    pub fn open_plan_importer_with_plan(
        &mut self,
        initial_plan: Option<(GeneratedServicePlan, usize)>,
        cx: &mut Context<Self>,
    ) {
        let store = self.store.clone();
        let db = self.db.clone();
        let modal = cx.new(|cx| {
            let mut m = PlanImportModal::new(store, db, cx);
            if let Some((plan, selected)) = initial_plan {
                m.set_plan_for_preview(plan, cx);
                m.selected_item_idx = selected;
            }
            m
        });
        self.modal_sub = Some(cx.subscribe(&modal, |this, _, _: &ModalClosed, cx| {
            this.close_modal(cx);
        }));
        self.active_modal = Some(ActiveModal::PlanImporter(modal));
        cx.notify();
    }

    pub fn close_modal(&mut self, cx: &mut Context<Self>) {
        self.active_modal = None;
        self.modal_sub = None;
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

    fn on_song_search(
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

    fn dispatch_cmd(&mut self, cmd: PresentationCommand, cx: &mut Context<Self>) {
        self.dispatch_cmd_cut(cmd, false, cx);
    }

    fn dispatch_cmd_cut(
        &mut self,
        cmd: PresentationCommand,
        hard_cut: bool,
        cx: &mut Context<Self>,
    ) {
        self.store.update(cx, |s, cx| {
            if let Err(err) = s.dispatch_with_cut(cmd, hard_cut, cx) {
                s.set_status(err.to_string(), cx);
            }
        });
    }
}

impl Focusable for OperatorWorkspace {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for OperatorWorkspace {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let motion_mode = self.store.read(cx).motion_mode;
        let header = render_operator_header(self, cx);
        let left_schedule = render_schedule_pane(self, cx);
        let center_workspace = render_library_preview_pane(self, cx);
        let right_live = render_live_pane(self, cx);

        let mut root = div()
            .relative()
            .key_context("OperatorWorkspace")
            .track_focus(&self.focus_handle)
            .size_full()
            .bg(TOKENS.canvas)
            .font_family(FONT_BODY)
            .flex()
            .flex_col()
            .on_action(cx.listener(|this, _: &GoLive, _, cx| {
                this.dispatch_cmd(PresentationCommand::GoLive, cx);
            }))
            .on_action(cx.listener(|this, _: &NextLiveSlide, _, cx| {
                this.dispatch_cmd(PresentationCommand::NextLiveSlide, cx);
            }))
            .on_action(cx.listener(|this, _: &PrevLiveSlide, _, cx| {
                this.dispatch_cmd(PresentationCommand::PrevLiveSlide, cx);
            }))
            .on_action(cx.listener(|this, _: &ToggleClearText, _, cx| {
                this.dispatch_cmd(
                    PresentationCommand::ToggleOverride(OutputOverride::ClearText),
                    cx,
                );
            }))
            .on_action(cx.listener(|this, _: &ToggleBlack, _, cx| {
                this.dispatch_cmd(
                    PresentationCommand::ToggleOverride(OutputOverride::Black),
                    cx,
                );
            }))
            .on_action(cx.listener(|this, _: &ToggleLogo, _, cx| {
                this.dispatch_cmd(
                    PresentationCommand::ToggleOverride(OutputOverride::Logo),
                    cx,
                );
            }))
            .on_action(cx.listener(|this, _: &HardCutClearText, _, cx| {
                this.dispatch_cmd_cut(
                    PresentationCommand::ToggleOverride(OutputOverride::ClearText),
                    true,
                    cx,
                );
            }))
            .on_action(cx.listener(|this, _: &HardCutBlack, _, cx| {
                this.dispatch_cmd_cut(
                    PresentationCommand::ToggleOverride(OutputOverride::Black),
                    true,
                    cx,
                );
            }))
            .on_action(cx.listener(|this, _: &HardCutLogo, _, cx| {
                this.dispatch_cmd_cut(
                    PresentationCommand::ToggleOverride(OutputOverride::Logo),
                    true,
                    cx,
                );
            }))
            .on_action(cx.listener(|this, _: &ToggleQuickEdit, _, cx| {
                this.open_quick_edit(cx);
            }))
            .on_action(cx.listener(|this, _: &ToggleAudienceWindow, _, cx| {
                let store = this.store.clone();
                this.windows
                    .update(cx, |wm, cx| wm.toggle_audience(store, cx));
            }))
            .on_action(cx.listener(|this, _: &ToggleStageWindow, _, cx| {
                let store = this.store.clone();
                this.windows.update(cx, |wm, cx| wm.toggle_stage(store, cx));
            }))
            .on_action(cx.listener(|this, _: &ToggleDirectClickLive, _, cx| {
                let next = !this.store.read(cx).snapshot().direct_click_live;
                this.dispatch_cmd(PresentationCommand::SetDirectClickLive(next), cx);
            }))
            .on_action(cx.listener(|this, _: &ClearStageMessage, _, cx| {
                this.dispatch_cmd(PresentationCommand::SetStageMessage(None), cx);
            }))
            .on_action(cx.listener(|this, _: &QuitApp, _, cx| {
                this.windows.update(cx, |wm, cx| wm.close_all_outputs(cx));
                cx.quit();
            }))
            .child(header)
            .child(
                div()
                    .flex_1()
                    .w_full()
                    .overflow_hidden()
                    .flex()
                    .flex_row()
                    .child(left_schedule)
                    .child(center_workspace)
                    .child(right_live),
            );

        if let Some(modal) = &self.active_modal {
            let (modal_id, content): (&'static str, AnyElement) = match modal {
                ActiveModal::SongEditor(m) => ("modal-song-editor", m.clone().into_any_element()),
                ActiveModal::QuickEdit(m) => ("modal-quick-edit", m.clone().into_any_element()),
                ActiveModal::AlertsTimers(m) => {
                    ("modal-alerts-timers", m.clone().into_any_element())
                }
                ActiveModal::PlanImporter(m) => {
                    ("modal-plan-importer", m.clone().into_any_element())
                }
            };
            root = root.child(
                div()
                    .absolute()
                    .inset_0()
                    .bg(rgba(0x0506_08D6))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(modal_enter(modal_id, content, motion_mode)),
            );
        }

        root
    }
}
