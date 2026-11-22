use gpui::{
    App, Context, Entity, FocusHandle, Focusable, InteractiveElement, IntoElement, ParentElement,
    Render, Styled, Subscription, Window, div,
};
use sundaydeck_core::{OutputOverride, PresentationCommand, ServiceItemId, SongId};
use sundaydeck_db::DbWorker;

use crate::actions::{
    ClearStageMessage, GoLive, NextLiveSlide, PrevLiveSlide, QuitApp, ToggleAudienceWindow,
    ToggleBlack, ToggleClearText, ToggleDirectClickLive, ToggleLogo, ToggleStageWindow,
};
use crate::palette::{BG_APP, color};
use crate::store::PresentationStore;
use crate::views::library_preview_pane::render_library_preview_pane;
use crate::views::live_pane::render_live_pane;
use crate::views::operator_header::render_operator_header;
use crate::views::schedule_pane::render_schedule_pane;
use crate::windows::WindowManager;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LibraryTab {
    Songs,
    Bible,
    Themes,
}

/// Primary 3-Pane Operator Workspace composing Service Schedule (Left),
/// Preview Grid & Library (Center), and Live Control & Overrides (Right).
pub struct OperatorWorkspace {
    pub store: Entity<PresentationStore>,
    pub windows: Entity<WindowManager>,
    pub db: Option<DbWorker>,
    pub active_library_tab: LibraryTab,
    focus_handle: FocusHandle,
    _store_sub: Subscription,
    _windows_sub: Subscription,
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
        Self {
            store,
            windows,
            db,
            active_library_tab: LibraryTab::Songs,
            focus_handle: cx.focus_handle(),
            _store_sub: store_sub,
            _windows_sub: windows_sub,
        }
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

    fn dispatch_cmd(&mut self, cmd: PresentationCommand, cx: &mut Context<Self>) {
        self.store.update(cx, |s, cx| {
            if let Err(err) = s.dispatch(cmd, cx) {
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
        let header = render_operator_header(self, cx);
        let left_schedule = render_schedule_pane(self, cx);
        let center_workspace = render_library_preview_pane(self, cx);
        let right_live = render_live_pane(self, cx);

        div()
            .key_context("OperatorWorkspace")
            .track_focus(&self.focus_handle)
            .size_full()
            .bg(color(BG_APP))
            .font_family(".SystemUIFont")
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
            )
    }
}
