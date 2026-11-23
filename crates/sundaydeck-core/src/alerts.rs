use serde::{Deserialize, Serialize};

use crate::ids::AlertId;

/// Configurable screen placement for discreet nursery / parent-paging alerts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlertPosition {
    TopRight,
    TopLeft,
    TopBar,
    BottomBar,
}

impl AlertPosition {
    pub const ALL: [Self; 4] = [Self::TopRight, Self::TopLeft, Self::TopBar, Self::BottomBar];

    pub const fn display_label(self) -> &'static str {
        match self {
            Self::TopRight => "Top Right",
            Self::TopLeft => "Top Left",
            Self::TopBar => "Top Banner",
            Self::BottomBar => "Bottom Ticker",
        }
    }
}

/// Auto-dismiss policy for an active audience alert.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum AlertTimeout {
    Manual,
    Seconds { duration_secs: u16 },
}

impl AlertTimeout {
    pub const PRESETS: [Self; 4] = [
        Self::Seconds { duration_secs: 15 },
        Self::Seconds { duration_secs: 30 },
        Self::Seconds { duration_secs: 60 },
        Self::Manual,
    ];

    pub const fn display_label(self) -> &'static str {
        match self {
            Self::Manual => "Manual",
            Self::Seconds { duration_secs: 15 } => "15s",
            Self::Seconds { duration_secs: 30 } => "30s",
            Self::Seconds { duration_secs: 60 } => "60s",
            Self::Seconds { .. } => "Timed",
        }
    }

    pub const fn expires_at(self, started_at_ms: i64) -> Option<i64> {
        match self {
            Self::Manual => None,
            Self::Seconds { duration_secs } => {
                Some(started_at_ms.saturating_add((duration_secs as i64) * 1000))
            }
        }
    }
}

/// Active discreet nursery or parent-paging alert overlay rendered on Audience output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AudienceAlert {
    pub id: AlertId,
    pub message: String,
    pub position: AlertPosition,
    pub expires_at_ms: Option<i64>,
}

impl AudienceAlert {
    pub fn new(
        message: impl Into<String>,
        position: AlertPosition,
        timeout: AlertTimeout,
        now_ms: i64,
    ) -> Self {
        Self {
            id: AlertId::new_v7(),
            message: message.into(),
            position,
            expires_at_ms: timeout.expires_at(now_ms),
        }
    }

    pub const fn is_expired(&self, now_ms: i64) -> bool {
        match self.expires_at_ms {
            Some(deadline) => now_ms >= deadline,
            None => false,
        }
    }

    pub const fn remaining_secs(&self, now_ms: i64) -> Option<u32> {
        match self.expires_at_ms {
            Some(deadline) => {
                let rem_ms = deadline.saturating_sub(now_ms);
                if rem_ms <= 0 {
                    Some(0)
                } else {
                    Some(((rem_ms + 999) / 1000) as u32)
                }
            }
            None => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timed_alert_tracks_remaining_seconds_and_expiry() {
        let alert = AudienceAlert::new(
            "Parent #142",
            AlertPosition::BottomBar,
            AlertTimeout::Seconds { duration_secs: 30 },
            10_000,
        );
        assert!(!alert.is_expired(10_000));
        assert_eq!(alert.remaining_secs(10_000), Some(30));
        assert_eq!(alert.remaining_secs(25_500), Some(15));
        assert!(alert.is_expired(40_000));
        assert_eq!(alert.remaining_secs(40_000), Some(0));
    }
}
