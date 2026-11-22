use gpui::{
    App, AppContext as _, Bounds, Context, DisplayId, Entity, TitlebarOptions, WindowBounds,
    WindowHandle, WindowKind, WindowOptions, point, px, size,
};

use crate::store::PresentationStore;
use crate::views::{AudienceOutputView, StageMonitorView};

/// Tracks live Audience and Stage output windows and manages multi-display targeting.
pub struct WindowManager {
    audience: Option<OutputSlot<AudienceOutputView>>,
    stage: Option<OutputSlot<StageMonitorView>>,
    pub last_notice: Option<String>,
}

struct OutputSlot<V: 'static> {
    handle: WindowHandle<V>,
    display_label: String,
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

    /// Toggles the Audience Projector window. Targets external display #2 in borderless
    /// fullscreen when connected, or opens a 16:9 rehearsal window on single-display laptops.
    pub fn toggle_audience(&mut self, store: Entity<PresentationStore>, cx: &mut Context<Self>) {
        if let Some(slot) = self.audience.take() {
            let _ = slot
                .handle
                .update(cx, |_, window, _| window.remove_window());
            self.last_notice = Some("Audience output closed".to_owned());
            cx.notify();
            return;
        }

        let (options, label, fullscreen) = build_output_window_options(
            "SundayDeck — Audience Output",
            1,
            point(px(120.0), px(90.0)),
            cx,
        );

        let manager = cx.entity().downgrade();
        match cx.open_window(options, |window, cx| {
            window.on_window_should_close(cx, move |_, cx| {
                let _ = manager.update(cx, |this, cx| {
                    this.audience = None;
                    cx.notify();
                });
                true
            });
            cx.new(|cx| AudienceOutputView::new(store, cx))
        }) {
            Ok(handle) => {
                self.audience = Some(OutputSlot {
                    handle,
                    display_label: label.clone(),
                });
                self.last_notice = Some(format!(
                    "Audience output active on {label}{}",
                    if fullscreen {
                        " (Fullscreen)"
                    } else {
                        " (Windowed)"
                    }
                ));
            }
            Err(err) => {
                self.last_notice = Some(format!("Failed to open Audience window: {err}"));
            }
        }
        cx.notify();
    }

    /// Toggles the Stage / Confidence Monitor window (external display #3 when available).
    pub fn toggle_stage(&mut self, store: Entity<PresentationStore>, cx: &mut Context<Self>) {
        if let Some(slot) = self.stage.take() {
            let _ = slot
                .handle
                .update(cx, |_, window, _| window.remove_window());
            self.last_notice = Some("Stage monitor closed".to_owned());
            cx.notify();
            return;
        }

        let (options, label, fullscreen) = build_output_window_options(
            "SundayDeck — Stage Monitor",
            2,
            point(px(180.0), px(140.0)),
            cx,
        );

        let manager = cx.entity().downgrade();
        match cx.open_window(options, |window, cx| {
            window.on_window_should_close(cx, move |_, cx| {
                let _ = manager.update(cx, |this, cx| {
                    this.stage = None;
                    cx.notify();
                });
                true
            });
            cx.new(|cx| StageMonitorView::new(store, cx))
        }) {
            Ok(handle) => {
                self.stage = Some(OutputSlot {
                    handle,
                    display_label: label.clone(),
                });
                self.last_notice = Some(format!(
                    "Stage monitor active on {label}{}",
                    if fullscreen {
                        " (Fullscreen)"
                    } else {
                        " (Windowed)"
                    }
                ));
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
            let _ = slot
                .handle
                .update(cx, |_, window, _| window.remove_window());
        }
        if let Some(slot) = self.stage.take() {
            let _ = slot
                .handle
                .update(cx, |_, window, _| window.remove_window());
        }
        cx.notify();
    }
}

fn build_output_window_options(
    title: &str,
    preferred_display_index: usize,
    fallback_origin: gpui::Point<gpui::Pixels>,
    cx: &App,
) -> (WindowOptions, String, bool) {
    let displays = cx.displays();
    if let Some(target) = displays.get(preferred_display_index) {
        let display_id: DisplayId = target.id();
        let bounds = target.bounds();
        let label = format!("Display {}", preferred_display_index + 1);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Fullscreen(bounds)),
            titlebar: None,
            focus: false,
            show: true,
            kind: WindowKind::Normal,
            display_id: Some(display_id),
            ..Default::default()
        };
        return (options, label, true);
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
        focus: false,
        show: true,
        kind: WindowKind::Normal,
        ..Default::default()
    };
    (options, "Rehearsal".to_owned(), false)
}
