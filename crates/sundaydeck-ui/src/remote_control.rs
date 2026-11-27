use gpui::{Context, Div, Entity, InteractiveElement as _};
use sundaydeck_core::{
    AlertPosition, AlertTimeout, AudienceAlert, OutputOverride, PresentationCommand,
};
use sundaydeck_overlay::{OverlayServerHandle, RemoteControlCommand};

use crate::actions::{
    GoLive, HardCutBlack, HardCutClearText, HardCutLogo, NextLiveSlide, PrevLiveSlide, ToggleBlack,
    ToggleClearText, ToggleLogo,
};
use crate::store::PresentationStore;

/// Formats a short nursery/parent security code (e.g. `"142"` or `"#142"`) into a clean
/// discreet sanctuary pager alert in the top-right corner with a 45-second auto-dismiss.
pub fn build_parent_pager_alert(code_raw: &str, now_ms: i64) -> Option<AudienceAlert> {
    let clean = code_raw.trim().trim_start_matches('#').trim();
    if clean.is_empty() {
        return None;
    }
    let msg = format!("Parent #{clean}");
    Some(AudienceAlert::new(
        msg,
        AlertPosition::TopRight,
        AlertTimeout::Seconds { duration_secs: 45 },
        now_ms,
    ))
}

/// Spawns a zero-polling async listener draining incoming Stream Deck / Companion / Booth Remote
/// commands from `handle` until the overlay server shuts down.
pub fn spawn_remote_control_listener(
    handle: &OverlayServerHandle,
    cx: &mut Context<PresentationStore>,
) {
    let rx = handle.control_receiver();
    cx.spawn(async move |this, cx| {
        while let Ok(cmd) = rx.recv().await {
            let res = this.update(cx, |store, cx| apply_remote_command(store, cmd, cx));
            if res.is_err() {
                break;
            }
        }
    })
    .detach();
}

fn apply_remote_command(
    store: &mut PresentationStore,
    cmd: RemoteControlCommand,
    cx: &mut Context<PresentationStore>,
) {
    let now_ms = store.now_epoch_ms();
    let pres_cmd = match cmd {
        RemoteControlCommand::GoLive => PresentationCommand::GoLive,
        RemoteControlCommand::NextSlide => PresentationCommand::NextLiveSlide,
        RemoteControlCommand::PrevSlide => PresentationCommand::PrevLiveSlide,
        RemoteControlCommand::ToggleClearText => {
            PresentationCommand::ToggleOverride(OutputOverride::ClearText)
        }
        RemoteControlCommand::ToggleBlack => {
            PresentationCommand::ToggleOverride(OutputOverride::Black)
        }
        RemoteControlCommand::ToggleLogo => {
            PresentationCommand::ToggleOverride(OutputOverride::Logo)
        }
        RemoteControlCommand::ClearOverrides => {
            PresentationCommand::SetOverride(OutputOverride::None)
        }
        RemoteControlCommand::ShowAlert(msg) => {
            let alert = AudienceAlert::new(
                msg,
                AlertPosition::BottomBar,
                AlertTimeout::Seconds { duration_secs: 30 },
                now_ms,
            );
            PresentationCommand::SetAudienceAlert(Some(alert))
        }
        RemoteControlCommand::PageParent(code) => {
            let Some(alert) = build_parent_pager_alert(&code, now_ms) else {
                return;
            };
            PresentationCommand::SetAudienceAlert(Some(alert))
        }
        RemoteControlCommand::ClearAlert => PresentationCommand::SetAudienceAlert(None),
    };
    let _ = store.dispatch(pres_cmd, cx);
}

/// Attaches `OutputWindow` key context and wireless PowerPoint presenter clicker / recovery
/// action handlers to Audience Projector and Stage Monitor windows so a clicker advances slides
/// or toggles blackout even when an output window has OS focus.
pub fn bind_output_clicker_actions(root: Div, store: Entity<PresentationStore>) -> Div {
    let s1 = store.clone();
    let s2 = store.clone();
    let s3 = store.clone();
    let s4 = store.clone();
    let s5 = store.clone();
    let s6 = store.clone();
    let s7 = store.clone();
    let s8 = store.clone();
    let s9 = store;
    root.key_context("OutputWindow")
        .on_action(move |_: &GoLive, _, cx| {
            s1.update(cx, |s, cx| {
                let _ = s.dispatch(PresentationCommand::GoLive, cx);
            });
        })
        .on_action(move |_: &NextLiveSlide, _, cx| {
            s2.update(cx, |s, cx| {
                let _ = s.dispatch(PresentationCommand::NextLiveSlide, cx);
            });
        })
        .on_action(move |_: &PrevLiveSlide, _, cx| {
            s3.update(cx, |s, cx| {
                let _ = s.dispatch(PresentationCommand::PrevLiveSlide, cx);
            });
        })
        .on_action(move |_: &ToggleClearText, _, cx| {
            s4.update(cx, |s, cx| {
                let _ = s.dispatch(
                    PresentationCommand::ToggleOverride(OutputOverride::ClearText),
                    cx,
                );
            });
        })
        .on_action(move |_: &ToggleBlack, _, cx| {
            s5.update(cx, |s, cx| {
                let _ = s.dispatch(
                    PresentationCommand::ToggleOverride(OutputOverride::Black),
                    cx,
                );
            });
        })
        .on_action(move |_: &ToggleLogo, _, cx| {
            s6.update(cx, |s, cx| {
                let _ = s.dispatch(
                    PresentationCommand::ToggleOverride(OutputOverride::Logo),
                    cx,
                );
            });
        })
        .on_action(move |_: &HardCutClearText, _, cx| {
            s7.update(cx, |s, cx| {
                let _ = s.dispatch_with_cut(
                    PresentationCommand::ToggleOverride(OutputOverride::ClearText),
                    true,
                    cx,
                );
            });
        })
        .on_action(move |_: &HardCutBlack, _, cx| {
            s8.update(cx, |s, cx| {
                let _ = s.dispatch_with_cut(
                    PresentationCommand::ToggleOverride(OutputOverride::Black),
                    true,
                    cx,
                );
            });
        })
        .on_action(move |_: &HardCutLogo, _, cx| {
            s9.update(cx, |s, cx| {
                let _ = s.dispatch_with_cut(
                    PresentationCommand::ToggleOverride(OutputOverride::Logo),
                    true,
                    cx,
                );
            });
        })
}
