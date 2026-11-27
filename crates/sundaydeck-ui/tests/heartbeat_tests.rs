//! Heartbeat gate regressions (plan WP1 `heartbeat_key`): a heartbeat tick redraws only when
//! something time-dependent became visibly different.
use std::sync::Arc;

use gpui::{AppContext as _, Entity};
use gpui_screenshot::ScreenshotApp;
use sundaydeck_core::{
    AlertPosition, AlertTimeout, AudienceAlert, PresentationCommand, PresentationSnapshot,
    ServiceTimer,
};
use sundaydeck_ui::design::MotionMode;
use sundaydeck_ui::heartbeat::{HEARTBEAT_INTERVAL_MS, heartbeat_key};
use sundaydeck_ui::store::PresentationStore;

/// Exactly 20 minutes past the epoch: a minute boundary.
const T: i64 = 1_200_000;

fn seeded_store(app: &mut ScreenshotApp) -> Entity<PresentationStore> {
    let mut db = sundaydeck_db::Database::open_in_memory().expect("in-memory sqlite");
    sundaydeck_db::seed::seed_defaults_if_empty(db.conn_mut()).expect("seed defaults");
    let boot = sundaydeck_db::worker::load_bootstrap_sync(&db).expect("bootstrap");
    app.update(|cx| cx.new(|_| PresentationStore::from_bootstrap(boot)))
}

fn snapshot_after(commands: Vec<PresentationCommand>) -> Arc<PresentationSnapshot> {
    let mut app = ScreenshotApp::new().expect("headless app");
    let store = seeded_store(&mut app);
    app.update(|cx| {
        store.update(cx, |s, cx| {
            if let Some(id) = s.songs.first().map(|song| song.id) {
                s.stage_library_song(id, cx);
            }
            for command in commands {
                s.dispatch(command, cx).expect("fixture command applies");
            }
            s.snapshot()
        })
    })
}

fn same_key(snapshot: &PresentationSnapshot, motion: MotionMode, a_ms: i64, b_ms: i64) -> bool {
    heartbeat_key(snapshot, motion, a_ms) == heartbeat_key(snapshot, motion, b_ms)
}

#[test]
fn idle_key_changes_only_when_the_wall_clock_minute_changes() {
    let snapshot = snapshot_after(vec![PresentationCommand::GoLive]);
    assert!(same_key(&snapshot, MotionMode::Full, T, T + 59_999));
    assert!(
        !same_key(&snapshot, MotionMode::Full, T + 59_999, T + 60_000),
        "header and stage clocks must refresh on the minute"
    );
}

#[test]
fn running_timer_key_changes_once_per_elapsed_second() {
    let mut timer = ServiceTimer::countdown("Service", 600, false);
    timer.start(T);
    let running = snapshot_after(vec![PresentationCommand::SetTimer(Some(timer.clone()))]);
    assert!(same_key(&running, MotionMode::Instant, T + 1, T + 999));
    assert!(!same_key(&running, MotionMode::Instant, T + 999, T + 1_000));
    assert!(!same_key(
        &running,
        MotionMode::Instant,
        T + 1_000,
        T + 2_000
    ));

    timer.pause(T + 1_500);
    let paused = snapshot_after(vec![PresentationCommand::SetTimer(Some(timer))]);
    assert!(
        same_key(&paused, MotionMode::Instant, T + 2_000, T + 9_000),
        "a paused timer shows a fixed value"
    );
}

#[test]
fn timed_alert_key_tracks_its_remaining_seconds() {
    let alert = AudienceAlert::new(
        "Parent of child #12 to the nursery",
        AlertPosition::TopRight,
        AlertTimeout::Seconds { duration_secs: 30 },
        T,
    );
    let snapshot = snapshot_after(vec![PresentationCommand::SetAudienceAlert(Some(alert))]);
    // 30 s → 29 s remaining happens at T + 1_000.
    assert!(same_key(&snapshot, MotionMode::Instant, T + 100, T + 999));
    assert!(!same_key(
        &snapshot,
        MotionMode::Instant,
        T + 999,
        T + 1_001
    ));
}

#[test]
fn live_motion_loop_ticks_every_heartbeat_unless_motion_is_instant() {
    let mut app = ScreenshotApp::new().expect("headless app");
    let store = seeded_store(&mut app);
    let snapshot = app.update(|cx| {
        store.update(cx, |s, cx| {
            let motion_theme = s
                .themes
                .iter()
                .find(|t| t.name == "Golden Light Motion")
                .cloned()
                .expect("seeded Golden Light Motion theme");
            if let Some(id) = s.songs.first().map(|song| song.id) {
                s.stage_library_song(id, cx);
            }
            s.dispatch(PresentationCommand::GoLive, cx)
                .expect("go live");
            s.dispatch(PresentationCommand::ApplyLiveTheme(motion_theme), cx)
                .expect("apply motion theme");
            s.snapshot()
        })
    });
    let tick = i64::try_from(HEARTBEAT_INTERVAL_MS).expect("interval fits i64");
    for motion in [MotionMode::Full, MotionMode::CrossfadeOnly] {
        assert!(
            !same_key(&snapshot, motion, T, T + tick),
            "{motion:?}: the motion loop must keep its blend cadence"
        );
    }
    assert!(
        same_key(&snapshot, MotionMode::Instant, T, T + tick),
        "Instant motion freezes the loop, so ticks must not redraw"
    );
}
