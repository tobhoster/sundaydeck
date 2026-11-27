//! Shared headless rig for the SundayDeck UI regression tests (redraw cadence, window lifecycle,
//! keyboard routing). Each test binary uses a different subset, hence the `dead_code` allowance.
#![allow(dead_code)]

use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use gpui::{
    AnyWindowHandle, App, AppContext as _, Context, Entity, IntoElement, Keystroke, Render, Window,
    px, size,
};
use gpui_screenshot::ScreenshotApp;
use sundaydeck_core::{PresentationCommand, SongId};
use sundaydeck_db::{BootstrapData, Database, DbWorker};
use sundaydeck_ui::design::{DeckAssetSource, register_fonts};
use sundaydeck_ui::store::PresentationStore;
use sundaydeck_ui::views::OperatorWorkspace;
use sundaydeck_ui::windows::WindowManager;

/// One simulated 60 Hz frame.
pub const FRAME_US: u64 = 16_667;
pub const FRAMES_PER_SECOND: u32 = 60;
/// Fixed wall-clock origin for tests that drive `clock_override_ms` (≈ 16 min past epoch, so a
/// few simulated seconds never cross a minute boundary).
pub const CLOCK_ORIGIN_MS: i64 = 1_000_000;

pub fn seeded_bootstrap() -> BootstrapData {
    let mut db = Database::open_in_memory().expect("in-memory sqlite");
    sundaydeck_db::seed::seed_defaults_if_empty(db.conn_mut()).expect("seed defaults");
    sundaydeck_db::worker::load_bootstrap_sync(&db).expect("load bootstrap")
}

/// Root view that counts its own renders: the root re-renders on every window draw.
struct DrawCounter {
    inner: Entity<OperatorWorkspace>,
    draws: Rc<Cell<usize>>,
}

impl Render for DrawCounter {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.draws.set(self.draws.get() + 1);
        self.inner.clone()
    }
}

pub struct Rig {
    pub app: ScreenshotApp,
    pub store: Entity<PresentationStore>,
    pub windows: Entity<WindowManager>,
    pub workspace: Entity<OperatorWorkspace>,
    pub operator: AnyWindowHandle,
    draws: Rc<Cell<usize>>,
    simulated_us: u64,
}

pub struct RigOptions {
    /// Synthetic songs appended to the seeded library (copies of the first seeded song).
    pub extra_songs: usize,
    /// Drive `PresentationStore::clock_override_ms` in lock-step with the simulated frames.
    pub simulated_clock: bool,
    /// Attach an in-memory `DbWorker` (needed for FTS song search).
    pub db: bool,
}

impl Default for RigOptions {
    fn default() -> Self {
        Self {
            extra_songs: 0,
            simulated_clock: false,
            db: true,
        }
    }
}

pub fn rig() -> Rig {
    rig_with(RigOptions::default())
}

pub fn rig_with(options: RigOptions) -> Rig {
    let mut app = ScreenshotApp::with_assets(DeckAssetSource).expect("headless lavapipe app");
    app.update(|cx| {
        register_fonts(cx).expect("register bundled fonts");
        sundaydeck_ui::actions::register_keybindings(cx);
    });
    let db = options
        .db
        .then(|| DbWorker::spawn_in_memory().expect("spawn in-memory db worker"));
    let boot = seeded_bootstrap();
    let (store, windows) = app.update(|cx| {
        let store = cx.new(|_| {
            let mut s = PresentationStore::from_bootstrap(boot);
            append_synthetic_songs(&mut s, options.extra_songs);
            if options.simulated_clock {
                s.clock_override_ms = Some(CLOCK_ORIGIN_MS);
            }
            s
        });
        (store, cx.new(|_| WindowManager::new()))
    });
    let draws = Rc::new(Cell::new(0));
    let workspace_slot: Rc<Cell<Option<Entity<OperatorWorkspace>>>> = Rc::new(Cell::new(None));
    let (ws_store, ws_windows, ws_draws, ws_slot) = (
        store.clone(),
        windows.clone(),
        draws.clone(),
        workspace_slot.clone(),
    );
    let operator = app
        .open_window(size(px(1440.0), px(900.0)), 1.0, move |_, cx| {
            let workspace = cx.new(|cx| OperatorWorkspace::new(ws_store, ws_windows, db, cx));
            ws_slot.set(Some(workspace.clone()));
            cx.new(|_| DrawCounter {
                inner: workspace,
                draws: ws_draws,
            })
        })
        .expect("open operator window");
    let workspace = workspace_slot.take().expect("operator workspace entity");
    app.update(|cx| cx.set_reduce_motion(false));
    app.run_until_parked();
    Rig {
        app,
        store,
        windows,
        workspace,
        operator: operator.into(),
        draws,
        simulated_us: 0,
    }
}

fn append_synthetic_songs(store: &mut PresentationStore, count: usize) {
    let Some(template) = store.songs.first().cloned() else {
        return;
    };
    for i in 0..count {
        let mut song = template.clone();
        song.id = SongId::new_v7();
        song.title = format!("Synthetic Hymn {i:05}");
        store.songs.push(song);
    }
}

impl Rig {
    pub fn draws(&self) -> usize {
        self.draws.get()
    }

    /// Simulates `frames` 60 Hz frames and returns how many Operator window draws happened.
    pub fn draws_over_frames(&mut self, frames: u32) -> usize {
        self.app.run_until_parked();
        let start = self.draws.get();
        for _ in 0..frames {
            self.step_frame();
        }
        self.draws.get() - start
    }

    pub fn draws_over_secs(&mut self, secs: u32) -> usize {
        self.draws_over_frames(secs * FRAMES_PER_SECOND)
    }

    fn step_frame(&mut self) {
        self.simulated_us += FRAME_US;
        let simulated_ms = i64::try_from(self.simulated_us / 1_000).expect("simulated ms fits");
        self.app.update(|cx| {
            self.store.update(cx, |s, _| {
                if s.clock_override_ms.is_some() {
                    s.clock_override_ms = Some(CLOCK_ORIGIN_MS + simulated_ms);
                }
            })
        });
        self.app.advance_clock(Duration::from_micros(FRAME_US));
        let _ = self.app.update_window(self.operator, |_, window, cx| {
            window.simulate_next_frame(cx)
        });
        self.app.run_until_parked();
    }

    pub fn dispatch(&mut self, command: PresentationCommand) {
        self.app.update(|cx| {
            self.store.update(cx, |s, cx| {
                // Commands that are invalid in the current state return Err by design.
                let _ = s.dispatch(command, cx);
            })
        });
        self.app.run_until_parked();
    }

    /// Stages the first seeded song and takes it live.
    pub fn go_live_with_first_song(&mut self) {
        self.app.update(|cx| {
            self.store.update(cx, |s, cx| {
                if let Some(id) = s.songs.first().map(|song| song.id) {
                    s.stage_library_song(id, cx);
                }
                let _ = s.dispatch(PresentationCommand::GoLive, cx);
            })
        });
        self.app.run_until_parked();
    }

    pub fn live_index(&mut self) -> Option<usize> {
        self.app.update(|cx| {
            self.store
                .read(cx)
                .snapshot()
                .live
                .as_ref()
                .map(|l| l.active_index)
        })
    }

    pub fn live_title(&mut self) -> Option<String> {
        self.app.update(|cx| {
            self.store
                .read(cx)
                .snapshot()
                .live
                .as_ref()
                .map(|l| l.title.clone())
        })
    }

    /// Counts store notifications until the returned counter is dropped.
    pub fn count_store_notifies(&mut self) -> (Rc<Cell<usize>>, gpui::Subscription) {
        let count = Rc::new(Cell::new(0usize));
        let sink = count.clone();
        let sub = self
            .app
            .update(|cx| cx.observe(&self.store, move |_, _| sink.set(sink.get() + 1)));
        (count, sub)
    }

    pub fn toggle_audience(&mut self) {
        let store = self.store.clone();
        self.app.update(|cx| {
            self.windows
                .update(cx, |wm, cx| wm.toggle_audience(store, cx))
        });
        self.app.run_until_parked();
    }

    pub fn toggle_stage(&mut self) {
        let store = self.store.clone();
        self.app
            .update(|cx| self.windows.update(cx, |wm, cx| wm.toggle_stage(store, cx)));
        self.app.run_until_parked();
    }

    /// Focuses the given entity's focus handle inside `window`.
    pub fn focus<V: gpui::Focusable>(&mut self, window: AnyWindowHandle, view: &Entity<V>) {
        let view = view.clone();
        let _ = self.app.update_window(window, |_, window, cx| {
            let handle = view.read(cx).focus_handle(cx);
            window.focus(&handle, cx);
        });
        self.app.run_until_parked();
    }

    /// Dispatches a keystroke such as `"space"`, `"ctrl-w"` or `"escape"` to `window`.
    pub fn press(&mut self, window: AnyWindowHandle, keystroke: &str) {
        let keystroke = Keystroke::parse(keystroke).expect("valid keystroke");
        self.press_keystroke(window, keystroke);
    }

    /// Types one already-composed character (accents, emoji, combining marks) into `window`.
    pub fn type_char(&mut self, window: AnyWindowHandle, text: &str) {
        let mut keystroke = Keystroke::parse("a").expect("valid keystroke");
        keystroke.key = text.to_owned();
        keystroke.key_char = Some(text.to_owned());
        self.press_keystroke(window, keystroke);
    }

    fn press_keystroke(&mut self, window: AnyWindowHandle, keystroke: Keystroke) {
        let _ = self.app.update_window(window, |_, window, cx| {
            window.dispatch_keystroke(keystroke, cx);
        });
        self.app.run_until_parked();
        let _ = self.app.update_window(window, |_, window, cx| {
            window.refresh();
            window.draw(cx).clear(cx);
        });
        self.app.run_until_parked();
    }
}

/// All open windows whose root view is `V`.
pub fn windows_of<V: 'static>(cx: &App) -> Vec<AnyWindowHandle> {
    cx.windows()
        .into_iter()
        .filter(|w| w.downcast::<V>().is_some())
        .collect()
}
