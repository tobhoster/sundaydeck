use std::time::Duration;

use gpui::{
    AnyElement, App, AppContext as _, Context, Entity, FocusHandle, Focusable, InteractiveElement,
    IntoElement, MouseButton, MouseMoveEvent, ParentElement, Render, Styled, Subscription, Task,
    WeakEntity, Window, div, rgba,
};
use sundaydeck_core::{GeneratedServicePlan, PresentationCommand, SlideContent, Song};
use sundaydeck_db::DbWorker;

use crate::components::TextInput;
use crate::design::icons::DeckIcon;
use crate::design::motion::modal_enter;
use crate::design::tokens::{FONT_BODY, TOKENS};
use crate::heartbeat::{HEARTBEAT_INTERVAL_MS, HeartbeatKey};
use crate::store::PresentationStore;
use crate::views::alerts_timers_modal::AlertsTimersModal;
use crate::views::background_picker_modal::BackgroundPickerModal;
use crate::views::bible_browser::BibleBrowserView;
use crate::views::custom_slide_modal::CustomSlideModal;
use crate::views::header_popovers::render_header_popover;
use crate::views::layout_handles::{
    DragResizeAxis, WorkspaceLayout, render_collapsed_rail_tab, render_vertical_resize_handle,
};
use crate::views::library_preview_pane::render_library_preview_pane;
use crate::views::live_pane::render_live_pane;
use crate::views::operator_header::render_operator_header;
use crate::views::plan_import_modal::PlanImportModal;
use crate::views::quick_edit_modal::{ModalClosed, QuickEditModal};
use crate::views::schedule_pane::render_schedule_pane;
use crate::views::settings_modal::SettingsModal;
use crate::views::song_editor_modal::{EditorTab, SongEditorModal};
use crate::views::welcome_modal::WelcomeModal;
use crate::windows::WindowManager;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeaderPopover {
    AutoFollow,
    Projector,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LibraryTab {
    Songs,
    Bible,
    CustomSlides,
    Themes,
}

pub enum ActiveModal {
    SongEditor(Entity<SongEditorModal>),
    QuickEdit(Entity<QuickEditModal>),
    AlertsTimers(Entity<AlertsTimersModal>),
    PlanImporter(Entity<PlanImportModal>),
    BackgroundPicker(Entity<BackgroundPickerModal>),
    CustomSlideBuilder(Entity<CustomSlideModal>),
    Settings(Entity<SettingsModal>),
    Welcome(Entity<WelcomeModal>),
}

/// Primary 3-Pane Operator Workspace composing Service Schedule (Left),
/// Preview Grid & Library/Bible (Center), Live Control & Overrides (Right),
/// and non-blocking overlay modals for Song Editing, Quick Live Edit, Alerts/Timers, and Settings.
pub struct OperatorWorkspace {
    pub store: Entity<PresentationStore>,
    pub windows: Entity<WindowManager>,
    pub db: Option<DbWorker>,
    pub active_library_tab: LibraryTab,
    pub song_search_input: Entity<TextInput>,
    pub bible_browser: Entity<BibleBrowserView>,
    pub active_modal: Option<ActiveModal>,
    pub active_popover: Option<HeaderPopover>,
    pub layout: WorkspaceLayout,
    pub available_update: Option<(String, String)>,
    pub(crate) modal_sub: Option<Subscription>,
    /// Pending debounced song search; replacing it cancels the older query.
    pub(crate) song_search_task: Option<Task<()>>,
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

        let first_key = store.read(cx).current_heartbeat_key();
        Self::spawn_heartbeat(store.downgrade(), first_key, cx);

        Self {
            store,
            windows,
            db,
            active_library_tab: LibraryTab::Songs,
            song_search_input,
            bible_browser,
            active_modal: None,
            active_popover: None,
            layout: WorkspaceLayout::default(),
            available_update: None,
            modal_sub: None,
            song_search_task: None,
            focus_handle: cx.focus_handle(),
            _store_sub: store_sub,
            _windows_sub: windows_sub,
            _song_search_sub: song_search_sub,
        }
    }

    /// Background heartbeat for live timers, voice telemetry, expiring alerts, and motion video.
    /// It notifies the store (and with it every window) only when the visible time-dependent
    /// state changed, e.g. once per second for a running timer instead of on every tick.
    fn spawn_heartbeat(
        store: WeakEntity<PresentationStore>,
        mut last_key: HeartbeatKey,
        cx: &mut Context<Self>,
    ) {
        cx.spawn(async move |_, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(HEARTBEAT_INTERVAL_MS))
                    .await;
                let ticked = store.update(cx, |s, cx| s.tick_heartbeat(&mut last_key, cx));
                if ticked.is_err() {
                    break;
                }
            }
        })
        .detach();
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
                SlideContent::Custom { layout, .. } => Some((
                    slide.id,
                    slide.label.clone(),
                    layout.primary_str().to_owned(),
                )),
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

    pub fn open_background_picker(&mut self, cx: &mut Context<Self>) {
        let store = self.store.clone();
        let db = self.db.clone();
        let modal = cx.new(|cx| BackgroundPickerModal::new(store, db, cx));
        self.modal_sub = Some(cx.subscribe(&modal, |this, _, _: &ModalClosed, cx| {
            this.close_modal(cx);
        }));
        self.active_modal = Some(ActiveModal::BackgroundPicker(modal));
        cx.notify();
    }

    pub fn close_modal(&mut self, cx: &mut Context<Self>) {
        self.active_modal = None;
        self.modal_sub = None;
        cx.notify();
    }

    pub fn toggle_popover(&mut self, target: HeaderPopover, cx: &mut Context<Self>) {
        self.active_popover = if self.active_popover == Some(target) {
            None
        } else {
            Some(target)
        };
        cx.notify();
    }

    pub fn close_popover(&mut self, cx: &mut Context<Self>) {
        self.active_popover = None;
        cx.notify();
    }

    pub(crate) fn dispatch_cmd(&mut self, cmd: PresentationCommand, cx: &mut Context<Self>) {
        self.dispatch_cmd_cut(cmd, false, cx);
    }

    pub(crate) fn dispatch_cmd_cut(
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
        let active_axis = self.layout.active_drag.map(|(a, _, _)| a);
        let left_collapsed = self.layout.left_collapsed;
        let right_collapsed = self.layout.right_collapsed;

        let header = render_operator_header(self, cx);
        let center_workspace = render_library_preview_pane(self, cx);

        let mut body_row = div().flex_1().w_full().overflow_hidden().flex().flex_row();

        if left_collapsed {
            body_row = body_row.child(render_collapsed_rail_tab(
                "expand-left-rail",
                "Plan",
                DeckIcon::ListNumbers,
                true,
                cx,
            ));
        } else {
            body_row = body_row.child(render_schedule_pane(self, cx)).child(
                render_vertical_resize_handle(
                    "resize-left-rail",
                    DragResizeAxis::LeftRail,
                    active_axis == Some(DragResizeAxis::LeftRail),
                    cx,
                ),
            );
        }

        body_row = body_row.child(center_workspace);

        if right_collapsed {
            body_row = body_row.child(render_collapsed_rail_tab(
                "expand-right-rail",
                "Live",
                DeckIcon::MonitorFill,
                false,
                cx,
            ));
        } else {
            body_row = body_row
                .child(render_vertical_resize_handle(
                    "resize-right-deck",
                    DragResizeAxis::RightDeck,
                    active_axis == Some(DragResizeAxis::RightDeck),
                    cx,
                ))
                .child(render_live_pane(self, cx));
        }

        let mut root = Self::bind_workspace_actions(
            div()
                .relative()
                .key_context("OperatorWorkspace")
                .track_focus(&self.focus_handle)
                .size_full()
                .bg(TOKENS.canvas)
                .font_family(FONT_BODY)
                .flex()
                .flex_col()
                .on_mouse_move(cx.listener(|this, ev: &MouseMoveEvent, _, cx| {
                    if this
                        .layout
                        .apply_drag(f32::from(ev.position.x), f32::from(ev.position.y))
                    {
                        cx.notify();
                    }
                }))
                .on_mouse_up(
                    MouseButton::Left,
                    cx.listener(|this, _, _, cx| {
                        if this.layout.end_drag() {
                            cx.notify();
                        }
                    }),
                )
                .on_mouse_up_out(
                    MouseButton::Left,
                    cx.listener(|this, _, _, cx| {
                        if this.layout.end_drag() {
                            cx.notify();
                        }
                    }),
                ),
            cx,
        )
        .child(header)
        .child(body_row);

        if let Some(popover) = self.active_popover {
            let mut slot = div().absolute().top(gpui::px(48.0));
            slot = match popover {
                HeaderPopover::AutoFollow => slot.left_0().right_0().flex().justify_center(),
                HeaderPopover::Projector => slot.right(gpui::px(18.0)),
            };
            root = root.child(slot.child(render_header_popover(self, popover, cx)));
        }

        if let Some(modal) = &self.active_modal {
            let (id, el): (&'static str, AnyElement) = match modal {
                ActiveModal::SongEditor(m) => ("modal-song-editor", m.clone().into_any_element()),
                ActiveModal::QuickEdit(m) => ("modal-quick-edit", m.clone().into_any_element()),
                ActiveModal::AlertsTimers(m) => ("modal-alerts", m.clone().into_any_element()),
                ActiveModal::PlanImporter(m) => ("modal-plan", m.clone().into_any_element()),
                ActiveModal::BackgroundPicker(m) => ("modal-bg", m.clone().into_any_element()),
                ActiveModal::CustomSlideBuilder(m) => {
                    ("modal-custom-slide", m.clone().into_any_element())
                }
                ActiveModal::Settings(m) => ("modal-settings", m.clone().into_any_element()),
                ActiveModal::Welcome(m) => ("modal-welcome", m.clone().into_any_element()),
            };
            root = root.child(
                div()
                    .absolute()
                    .inset_0()
                    .bg(rgba(0x0506_08D6))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(modal_enter(id, el, motion_mode)),
            );
        }

        root
    }
}
