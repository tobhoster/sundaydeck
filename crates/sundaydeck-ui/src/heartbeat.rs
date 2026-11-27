//! Operator heartbeat: the background tick that drives voice telemetry, announcement loops and
//! timed alerts, and the [`HeartbeatKey`] that decides whether a tick changed anything visible.

use gpui::Context;
use sundaydeck_core::{
    AudienceRenderState, BackgroundSpec, PresentationCommand, PresentationSnapshot,
};

use crate::design::motion::MotionMode;
use crate::store::PresentationStore;

/// Interval between heartbeat ticks. It is also the motion-loop blend cadence (~6 Hz).
pub const HEARTBEAT_INTERVAL_MS: u64 = 160;
const MS_PER_SECOND: i64 = 1_000;
const MS_PER_MINUTE: i64 = 60_000;

/// Everything time-dependent that a window shows. Two ticks with equal keys render identically,
/// so the heartbeat notifies the store (and with it every window) only when the key changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HeartbeatKey {
    /// Elapsed whole seconds of a running service timer (Stage, Audience and header text).
    timer_second: Option<i64>,
    /// Remaining whole seconds of a timed audience alert.
    alert_second: Option<i64>,
    /// Wall-clock minute shown by the header and Stage clocks.
    clock_minute: i64,
    /// Heartbeat tick while a motion-loop background is live and motion is enabled.
    video_tick: Option<i64>,
}

/// Computes the visible time-dependent state of `snapshot` at `now_ms`.
pub fn heartbeat_key(
    snapshot: &PresentationSnapshot,
    motion: MotionMode,
    now_ms: i64,
) -> HeartbeatKey {
    let video_live = motion != MotionMode::Instant
        && matches!(
            &snapshot.audience,
            AudienceRenderState::Slide { theme, .. }
                if matches!(theme.background, BackgroundSpec::VideoLoop { .. })
        );
    HeartbeatKey {
        timer_second: snapshot
            .active_timer
            .as_ref()
            .filter(|timer| timer.running)
            .map(|timer| timer.elapsed_ms(now_ms) / MS_PER_SECOND),
        alert_second: snapshot
            .active_alert
            .as_ref()
            .and_then(|alert| alert.remaining_secs(now_ms))
            .map(i64::from),
        clock_minute: now_ms / MS_PER_MINUTE,
        video_tick: video_live.then_some(now_ms / HEARTBEAT_INTERVAL_MS as i64),
    }
}

impl PresentationStore {
    /// Runs one heartbeat tick: drains voice events, advances a due announcement loop and
    /// expires a due alert. Each of those notifies on its own when it changes state.
    pub fn run_heartbeat_tick(&mut self, cx: &mut Context<Self>) {
        self.poll_voice_events(cx);
        let now = self.now_epoch_ms();
        let _ = self.tick_announcement_loop(now, cx);
        let alert_due = self
            .snapshot()
            .active_alert
            .as_ref()
            .is_some_and(|alert| alert.is_expired(now));
        if alert_due {
            let _ = self.dispatch(PresentationCommand::ExpireAlertIfDue { now_ms: now }, cx);
        }
    }

    /// The [`HeartbeatKey`] of the store's current state at its current clock.
    pub fn current_heartbeat_key(&self) -> HeartbeatKey {
        heartbeat_key(&self.snapshot(), self.motion_mode, self.now_epoch_ms())
    }

    /// Runs one heartbeat tick, then notifies only when the [`HeartbeatKey`] moved away from
    /// `last_key` (which is updated to the new key).
    pub fn tick_heartbeat(&mut self, last_key: &mut HeartbeatKey, cx: &mut Context<Self>) {
        self.run_heartbeat_tick(cx);
        let key = self.current_heartbeat_key();
        if key != *last_key {
            *last_key = key;
            cx.notify();
        }
    }
}
