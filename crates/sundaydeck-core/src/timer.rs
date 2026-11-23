use serde::{Deserialize, Serialize};

use crate::ids::TimerId;

const COUNTDOWN_WARNING_SECS: i64 = 120;

/// Operating mode of a service or speaker timer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TimerKind {
    /// Counts down from `target_secs`, entering overtime (`-MM:SS`) past zero.
    Countdown { target_secs: u32 },
    /// Counts upward from `00:00` to track elapsed speaker time.
    Elapsed,
}

/// Urgency visual level computed deterministically for Stage and Audience rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerUrgency {
    Normal,
    Warning,
    Overtime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimerDisplay {
    pub formatted: String,
    pub urgency: TimerUrgency,
}

/// Active pre-service countdown or speaker elapsed/remaining timer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceTimer {
    pub id: TimerId,
    pub label: String,
    pub kind: TimerKind,
    pub running: bool,
    pub accumulated_ms: i64,
    pub started_at_ms: Option<i64>,
    /// When `true`, renders a clean countdown badge/card on Audience output as well as Stage.
    pub show_on_audience: bool,
}

impl ServiceTimer {
    pub fn countdown(label: impl Into<String>, target_secs: u32, show_on_audience: bool) -> Self {
        Self {
            id: TimerId::new_v7(),
            label: label.into(),
            kind: TimerKind::Countdown { target_secs },
            running: false,
            accumulated_ms: 0,
            started_at_ms: None,
            show_on_audience,
        }
    }

    pub fn elapsed(label: impl Into<String>) -> Self {
        Self {
            id: TimerId::new_v7(),
            label: label.into(),
            kind: TimerKind::Elapsed,
            running: false,
            accumulated_ms: 0,
            started_at_ms: None,
            show_on_audience: false,
        }
    }

    pub fn start(&mut self, now_ms: i64) {
        if !self.running {
            self.running = true;
            self.started_at_ms = Some(now_ms);
        }
    }

    pub fn pause(&mut self, now_ms: i64) {
        if self.running {
            self.accumulated_ms = self.elapsed_ms(now_ms);
            self.running = false;
            self.started_at_ms = None;
        }
    }

    pub fn reset(&mut self) {
        self.running = false;
        self.accumulated_ms = 0;
        self.started_at_ms = None;
    }

    pub fn elapsed_ms(&self, now_ms: i64) -> i64 {
        let running_delta = match (self.running, self.started_at_ms) {
            (true, Some(started)) => now_ms.saturating_sub(started).max(0),
            _ => 0,
        };
        self.accumulated_ms.saturating_add(running_delta)
    }

    pub fn display(&self, now_ms: i64) -> TimerDisplay {
        let elapsed_secs = self.elapsed_ms(now_ms) / 1000;
        match self.kind {
            TimerKind::Countdown { target_secs } => {
                let remaining = i64::from(target_secs) - elapsed_secs;
                if remaining < 0 {
                    TimerDisplay {
                        formatted: format!("-{}", format_mm_ss(remaining.unsigned_abs())),
                        urgency: TimerUrgency::Overtime,
                    }
                } else if remaining <= COUNTDOWN_WARNING_SECS {
                    TimerDisplay {
                        formatted: format_mm_ss(remaining as u64),
                        urgency: TimerUrgency::Warning,
                    }
                } else {
                    TimerDisplay {
                        formatted: format_mm_ss(remaining as u64),
                        urgency: TimerUrgency::Normal,
                    }
                }
            }
            TimerKind::Elapsed => TimerDisplay {
                formatted: format_mm_ss(elapsed_secs.max(0) as u64),
                urgency: TimerUrgency::Normal,
            },
        }
    }
}

fn format_mm_ss(total_secs: u64) -> String {
    let hours = total_secs / 3600;
    let mins = (total_secs % 3600) / 60;
    let secs = total_secs % 60;
    if hours > 0 {
        format!("{hours}:{mins:02}:{secs:02}")
    } else {
        format!("{mins:02}:{secs:02}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn countdown_transitions_from_normal_to_warning_to_overtime() {
        let mut timer = ServiceTimer::countdown("Pre-Service", 300, true);
        timer.start(1_000);

        let d0 = timer.display(1_000);
        assert_eq!(d0.formatted, "05:00");
        assert_eq!(d0.urgency, TimerUrgency::Normal);

        let d_warn = timer.display(201_000);
        assert_eq!(d_warn.formatted, "01:40");
        assert_eq!(d_warn.urgency, TimerUrgency::Warning);

        let d_over = timer.display(319_000);
        assert_eq!(d_over.formatted, "-00:18");
        assert_eq!(d_over.urgency, TimerUrgency::Overtime);

        timer.pause(319_000);
        assert_eq!(timer.display(999_000).formatted, "-00:18");
        timer.reset();
        assert_eq!(timer.display(999_000).formatted, "05:00");
    }

    #[test]
    fn elapsed_timer_counts_up_with_hour_rollover() {
        let mut timer = ServiceTimer::elapsed("Sermon");
        timer.start(0);
        assert_eq!(timer.display(3_665_000).formatted, "1:01:05");
    }
}
