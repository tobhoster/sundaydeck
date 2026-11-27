//! Output-window lifecycle regressions (plan C4, C5, C6): closing an Audience or Stage window by
//! any route must resync `WindowManager`, toggles fired from inside an output window must not
//! orphan it, and every quit route must reach the single global `QuitApp` handler.
mod support;

use std::cell::Cell;
use std::rc::Rc;

use gpui::{AnyWindowHandle, App};
use sundaydeck_ui::actions::QuitApp;
use sundaydeck_ui::store::AppServices;
use sundaydeck_ui::views::{AudienceOutputView, StageMonitorView};
use sundaydeck_ui::windows::DisplayTarget;
use support::{Rig, rig, windows_of};

#[derive(Debug, Clone, Copy)]
enum Output {
    Audience,
    Stage,
}

impl Output {
    fn open_windows(self, cx: &App) -> Vec<AnyWindowHandle> {
        match self {
            Output::Audience => windows_of::<AudienceOutputView>(cx),
            Output::Stage => windows_of::<StageMonitorView>(cx),
        }
    }

    fn toggle(self, r: &mut Rig) {
        match self {
            Output::Audience => r.toggle_audience(),
            Output::Stage => r.toggle_stage(),
        }
    }

    fn global_toggle_shortcut(self) -> &'static str {
        match self {
            Output::Audience => "ctrl-shift-a",
            Output::Stage => "ctrl-shift-s",
        }
    }
}

/// `(output window count, WindowManager thinks it is open)`.
fn output_state(r: &mut Rig, output: Output) -> (usize, bool) {
    r.app.update(|cx| {
        let wm = r.windows.read(cx);
        let tracked = match output {
            Output::Audience => wm.is_audience_open(),
            Output::Stage => wm.is_stage_open(),
        };
        (output.open_windows(cx).len(), tracked)
    })
}

fn the_output_window(r: &mut Rig, output: Output) -> AnyWindowHandle {
    let windows = r.app.update(|cx| output.open_windows(cx));
    assert_eq!(windows.len(), 1, "{output:?}: expected exactly one window");
    windows[0]
}

#[derive(Debug, Clone, Copy)]
enum CloseRoute {
    /// What the "Exit Fullscreen (Esc)" badge does.
    RemoveWindow,
    Escape,
    CmdW,
}

#[test]
fn closing_an_output_window_by_any_route_resyncs_the_window_manager() {
    for output in [Output::Audience, Output::Stage] {
        for route in [
            CloseRoute::RemoveWindow,
            CloseRoute::Escape,
            CloseRoute::CmdW,
        ] {
            let mut r = rig();
            output.toggle(&mut r);
            let window = the_output_window(&mut r, output);
            match route {
                CloseRoute::RemoveWindow => {
                    let _ = r
                        .app
                        .update_window(window, |_, window, _| window.remove_window());
                    r.app.run_until_parked();
                }
                CloseRoute::Escape => r.press(window, "escape"),
                CloseRoute::CmdW => r.press(window, "ctrl-w"),
            }
            assert_eq!(
                output_state(&mut r, output),
                (0, false),
                "{output:?} closed via {route:?}: window gone but manager still tracks it"
            );

            output.toggle(&mut r);
            assert_eq!(
                output_state(&mut r, output),
                (1, true),
                "{output:?} after {route:?}: the next toggle must reopen exactly one window"
            );
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum ToggleOrigin {
    /// A menu item or action handler running inside the output window's own update.
    OutputWindowUpdate,
    /// The real global shortcut pressed while the output window has focus.
    GlobalShortcut,
}

fn install_app_menus(r: &mut Rig) {
    let services = AppServices {
        store: r.store.clone(),
        windows: r.windows.clone(),
        db: None,
    };
    r.app.update(|cx| {
        cx.set_global(services);
        sundaydeck_ui::menus::register_app_menus(cx);
    });
}

#[test]
fn toggling_from_a_focused_output_window_closes_it_without_orphans() {
    for output in [Output::Audience, Output::Stage] {
        for origin in [
            ToggleOrigin::OutputWindowUpdate,
            ToggleOrigin::GlobalShortcut,
        ] {
            let mut r = rig();
            install_app_menus(&mut r);
            output.toggle(&mut r);
            let window = the_output_window(&mut r, output);
            match origin {
                ToggleOrigin::OutputWindowUpdate => {
                    let (windows, store) = (r.windows.clone(), r.store.clone());
                    let _ = r.app.update_window(window, |_, _, cx| {
                        windows.update(cx, |wm, cx| match output {
                            Output::Audience => wm.toggle_audience(store, cx),
                            Output::Stage => wm.toggle_stage(store, cx),
                        })
                    });
                    r.app.run_until_parked();
                }
                ToggleOrigin::GlobalShortcut => r.press(window, output.global_toggle_shortcut()),
            }
            assert_eq!(
                output_state(&mut r, output),
                (0, false),
                "{output:?} toggled off via {origin:?} must actually close the window"
            );

            output.toggle(&mut r);
            assert_eq!(
                output_state(&mut r, output),
                (1, true),
                "{output:?} after {origin:?}: re-toggle must leave exactly one window"
            );
        }
    }
}

#[test]
fn retargeting_an_open_output_keeps_exactly_one_window() {
    for output in [Output::Audience, Output::Stage] {
        let mut r = rig();
        output.toggle(&mut r);
        for target in [
            DisplayTarget::Windowed,
            DisplayTarget::Display(0),
            DisplayTarget::Windowed,
        ] {
            let (windows, store) = (r.windows.clone(), r.store.clone());
            r.app.update(|cx| {
                windows.update(cx, |wm, cx| match output {
                    Output::Audience => wm.set_audience_target(target, store, cx),
                    Output::Stage => wm.set_stage_target(target, store, cx),
                })
            });
            r.app.run_until_parked();
            assert_eq!(
                output_state(&mut r, output),
                (1, true),
                "{output:?} retargeted to {target:?}"
            );
        }
    }
}

fn count_quit_requests(r: &mut Rig) -> Rc<Cell<usize>> {
    let quits = Rc::new(Cell::new(0usize));
    let sink = quits.clone();
    // Registered last, so it is the global `QuitApp` handler that runs in the bubble phase.
    r.app.update(|cx| {
        cx.on_action(move |_: &QuitApp, _| sink.set(sink.get() + 1));
    });
    quits
}

#[derive(Debug, Clone, Copy)]
enum QuitOrigin {
    Operator,
    Audience,
    Stage,
}

#[test]
fn every_quit_route_reaches_the_global_quit_handler_once() {
    let cases = [
        (QuitOrigin::Operator, "ctrl-w"),
        (QuitOrigin::Operator, "ctrl-q"),
        (QuitOrigin::Audience, "ctrl-q"),
        (QuitOrigin::Stage, "ctrl-q"),
    ];
    for (origin, keystroke) in cases {
        let mut r = rig();
        r.toggle_audience();
        r.toggle_stage();
        let quits = count_quit_requests(&mut r);
        let window = match origin {
            QuitOrigin::Operator => {
                let (operator, workspace) = (r.operator, r.workspace.clone());
                r.focus(operator, &workspace);
                operator
            }
            QuitOrigin::Audience => the_output_window(&mut r, Output::Audience),
            QuitOrigin::Stage => the_output_window(&mut r, Output::Stage),
        };
        r.press(window, keystroke);
        assert_eq!(
            quits.get(),
            1,
            "{keystroke} from {origin:?} must reach the global QuitApp handler exactly once"
        );
    }
}

#[test]
fn cmd_w_with_a_modal_open_closes_only_the_modal() {
    let mut r = rig();
    let quits = count_quit_requests(&mut r);
    let (operator, workspace) = (r.operator, r.workspace.clone());
    r.app
        .update(|cx| workspace.update(cx, |ws, cx| ws.open_alerts_timers(cx)));
    r.app.run_until_parked();
    r.focus(operator, &workspace);
    r.press(operator, "ctrl-w");

    let modal_open = r
        .app
        .update(|cx| r.workspace.read(cx).active_modal.is_some());
    let operator_alive = r.app.update(|cx| {
        cx.windows()
            .iter()
            .any(|w| w.window_id() == operator.window_id())
    });
    assert!(!modal_open, "Cmd+W must dismiss the open modal");
    assert_eq!(quits.get(), 0, "Cmd+W on a modal must not quit");
    assert!(
        operator_alive,
        "Cmd+W on a modal must keep the operator window"
    );
}
