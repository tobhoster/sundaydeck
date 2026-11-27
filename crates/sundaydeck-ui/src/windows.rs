use gpui::{
    App, AppContext as _, Bounds, Context, DisplayId, Entity, Focusable as _, Subscription,
    TitlebarOptions, WindowBounds, WindowHandle, WindowId, WindowKind, WindowOptions, point, px,
    size,
};

use crate::store::PresentationStore;
use crate::views::{AudienceOutputView, StageMonitorView};

const AUDIENCE_CLOSED_NOTICE: &str = "Audience output closed";
const STAGE_CLOSED_NOTICE: &str = "Stage monitor closed";

/// Explicit display targeting preference for Audience Projector or Stage Confidence Monitor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DisplayTarget {
    /// Automatically pick external display #2 (Audience) / #3 (Stage), falling back to a 16:9 rehearsal window.
    #[default]
    Auto,
    /// Target a specific connected OS display index in borderless fullscreen.
    Display(usize),
    /// Always open as a resizable 16:9 rehearsal window (ideal for single-screen testing).
    Windowed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectedDisplayInfo {
    pub index: usize,
    pub label: String,
    pub resolution: String,
}

/// Tracks live Audience and Stage output windows and manages multi-display targeting across
/// macOS, Linux, and Windows external monitors, projectors, and AirPlay displays.
pub struct WindowManager {
    audience: Option<OutputSlot<AudienceOutputView>>,
    stage: Option<OutputSlot<StageMonitorView>>,
    pub audience_target: DisplayTarget,
    pub stage_target: DisplayTarget,
    pub last_notice: Option<String>,
}

struct OutputSlot<V: 'static> {
    handle: WindowHandle<V>,
    display_label: String,
    /// Clears this slot when the window closes on its own (Esc, Cmd+W, the Exit Fullscreen
    /// badge, the OS close button). Dropping the slot unregisters it.
    _closed: Subscription,
}

impl<V: 'static> OutputSlot<V> {
    /// Whether this slot holds the window `id`.
    fn shows(&self, id: WindowId) -> bool {
        self.handle.window_id() == id
    }
}

impl Default for WindowManager {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowManager {
    pub fn new() -> Self {
        Self {
            audience: None,
            stage: None,
            audience_target: DisplayTarget::Auto,
            stage_target: DisplayTarget::Auto,
            last_notice: None,
        }
    }

    pub fn is_audience_open(&self) -> bool {
        self.audience.is_some()
    }

    pub fn is_stage_open(&self) -> bool {
        self.stage.is_some()
    }

    pub fn audience_label(&self) -> Option<&str> {
        self.audience
            .as_ref()
            .map(|slot| slot.display_label.as_str())
    }

    pub fn stage_label(&self) -> Option<&str> {
        self.stage.as_ref().map(|slot| slot.display_label.as_str())
    }

    pub fn display_count(cx: &App) -> usize {
        cx.displays().len()
    }

    /// Enumerates every connected physical display with index and pixel resolution.
    pub fn connected_displays(cx: &App) -> Vec<ConnectedDisplayInfo> {
        cx.displays()
            .iter()
            .enumerate()
            .map(|(index, d)| {
                let bounds = d.bounds();
                let w = f32::from(bounds.size.width).round() as i32;
                let h = f32::from(bounds.size.height).round() as i32;
                let role = if index == 0 { "Primary" } else { "External" };
                ConnectedDisplayInfo {
                    index,
                    label: format!("Display {} ({role})", index + 1),
                    resolution: format!("{w} × {h}"),
                }
            })
            .collect()
    }

    pub fn set_audience_target(
        &mut self,
        target: DisplayTarget,
        store: Entity<PresentationStore>,
        cx: &mut Context<Self>,
    ) {
        self.audience_target = target;
        if self.is_audience_open() {
            self.toggle_audience(store.clone(), cx);
            self.toggle_audience(store, cx);
        } else {
            cx.notify();
        }
    }

    pub fn set_stage_target(
        &mut self,
        target: DisplayTarget,
        store: Entity<PresentationStore>,
        cx: &mut Context<Self>,
    ) {
        self.stage_target = target;
        if self.is_stage_open() {
            self.toggle_stage(store.clone(), cx);
            self.toggle_stage(store, cx);
        } else {
            cx.notify();
        }
    }

    /// Toggles the Audience Projector window on the selected display target.
    pub fn toggle_audience(&mut self, store: Entity<PresentationStore>, cx: &mut Context<Self>) {
        if let Some(slot) = self.audience.take() {
            close_output_window(slot.handle, cx);
            self.last_notice = Some(AUDIENCE_CLOSED_NOTICE.to_owned());
            cx.notify();
            return;
        }

        let (options, label, fullscreen, single_screen_fs) = build_output_window_options(
            "SundayDeck — Audience Output",
            self.audience_target,
            1,
            point(px(120.0), px(90.0)),
            cx,
        );

        match cx.open_window(options, |window, cx| {
            let view =
                cx.new(|cx| AudienceOutputView::new(store, cx).with_escape_badge(single_screen_fs));
            let handle = view.read(cx).focus_handle(cx);
            window.focus(&handle, cx);
            view
        }) {
            Ok(handle) => {
                self.audience = Some(OutputSlot {
                    handle,
                    display_label: label.clone(),
                    _closed: Self::observe_output_closed(cx),
                });
                self.last_notice = Some(if single_screen_fs {
                    format!("Audience fullscreen on {label} — press Esc or Cmd+W to close")
                } else if fullscreen {
                    format!("Audience output active on {label} (Fullscreen)")
                } else {
                    "Audience output opened in a separate 16:9 window (Esc or Cmd+W to close)"
                        .to_owned()
                });
            }
            Err(err) => {
                self.last_notice = Some(format!("Failed to open Audience window: {err}"));
            }
        }
        cx.notify();
    }

    /// Toggles the Stage / Confidence Monitor window on the selected display target.
    pub fn toggle_stage(&mut self, store: Entity<PresentationStore>, cx: &mut Context<Self>) {
        if let Some(slot) = self.stage.take() {
            close_output_window(slot.handle, cx);
            self.last_notice = Some(STAGE_CLOSED_NOTICE.to_owned());
            cx.notify();
            return;
        }

        let (options, label, fullscreen, single_screen_fs) = build_output_window_options(
            "SundayDeck — Stage Monitor",
            self.stage_target,
            2,
            point(px(180.0), px(140.0)),
            cx,
        );

        match cx.open_window(options, |window, cx| {
            let view =
                cx.new(|cx| StageMonitorView::new(store, cx).with_escape_badge(single_screen_fs));
            let handle = view.read(cx).focus_handle(cx);
            window.focus(&handle, cx);
            view
        }) {
            Ok(handle) => {
                self.stage = Some(OutputSlot {
                    handle,
                    display_label: label.clone(),
                    _closed: Self::observe_output_closed(cx),
                });
                self.last_notice = Some(if single_screen_fs {
                    format!("Stage fullscreen on {label} — press Esc or Cmd+W to close")
                } else if fullscreen {
                    format!("Stage monitor active on {label} (Fullscreen)")
                } else {
                    "Stage monitor opened in a separate window (Esc or Cmd+W to close)".to_owned()
                });
            }
            Err(err) => {
                self.last_notice = Some(format!("Failed to open Stage window: {err}"));
            }
        }
        cx.notify();
    }

    /// Cleanly tears down auxiliary output windows before the Operator window exits
    /// so fullscreen projector windows are never left orphaned.
    pub fn close_all_outputs(&mut self, cx: &mut Context<Self>) {
        if let Some(slot) = self.audience.take() {
            close_output_window(slot.handle, cx);
        }
        if let Some(slot) = self.stage.take() {
            close_output_window(slot.handle, cx);
        }
        cx.notify();
    }

    /// Subscribes to window closes so a slot never outlives its window.
    fn observe_output_closed(cx: &mut Context<Self>) -> Subscription {
        let manager = cx.entity().downgrade();
        cx.on_window_closed(move |cx, closed_id| {
            let manager = manager.clone();
            // Window-closed observers run synchronously while the closing window's update
            // unwinds, possibly inside a `WindowManager` update (`close_all_outputs`), so the
            // slot is cleared on the next effect cycle instead of re-entering the manager.
            cx.defer(move |cx| {
                // `Err` only means the manager is already released (app shutdown).
                let _ = manager.update(cx, |wm, cx| wm.forget_output(closed_id, cx));
            });
        })
    }

    /// Clears the slot of an output window that closed on its own, so the header stops showing
    /// it as live and the next toggle opens a fresh window.
    fn forget_output(&mut self, closed_id: WindowId, cx: &mut Context<Self>) {
        let notice = if self.audience.as_ref().is_some_and(|s| s.shows(closed_id)) {
            self.audience = None;
            AUDIENCE_CLOSED_NOTICE
        } else if self.stage.as_ref().is_some_and(|s| s.shows(closed_id)) {
            self.stage = None;
            STAGE_CLOSED_NOTICE
        } else {
            return;
        };
        self.last_notice = Some(notice.to_owned());
        cx.notify();
    }
}

/// Closes an output window immediately, so a retarget never shows two fullscreen outputs at
/// once. From inside that same window's update (a shortcut or menu item used while the output
/// window is focused) the handle cannot be updated, so the close waits for that update to end.
fn close_output_window<V: 'static>(handle: WindowHandle<V>, cx: &mut App) {
    if handle
        .update(cx, |_, window, _| window.remove_window())
        .is_err()
    {
        cx.defer(move |cx| {
            // A second `Err` means the window is already gone.
            let _ = handle.update(cx, |_, window, _| window.remove_window());
        });
    }
}

fn build_output_window_options(
    title: &str,
    target: DisplayTarget,
    auto_display_index: usize,
    fallback_origin: gpui::Point<gpui::Pixels>,
    cx: &App,
) -> (WindowOptions, String, bool, bool) {
    let displays = cx.displays();
    let chosen_index = match target {
        DisplayTarget::Windowed => None,
        DisplayTarget::Display(idx) => displays.get(idx).map(|_| idx),
        DisplayTarget::Auto => displays.get(auto_display_index).map(|_| auto_display_index),
    };

    if let Some(idx) = chosen_index
        && let Some(disp) = displays.get(idx)
    {
        let display_id: DisplayId = disp.id();
        let bounds = disp.bounds();
        let label = format!("Display {}", idx + 1);
        let single_screen_fs = displays.len() <= 1 || idx == 0;
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Fullscreen(bounds)),
            titlebar: None,
            focus: single_screen_fs,
            show: true,
            kind: WindowKind::Normal,
            display_id: Some(display_id),
            ..Default::default()
        };
        return (options, label, true, single_screen_fs);
    }

    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: fallback_origin,
            size: size(px(960.0), px(540.0)),
        })),
        titlebar: Some(TitlebarOptions {
            title: Some(title.to_owned().into()),
            ..Default::default()
        }),
        focus: true,
        show: true,
        kind: WindowKind::Normal,
        ..Default::default()
    };
    (options, "Rehearsal".to_owned(), false, false)
}
