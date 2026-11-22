use serde::{Deserialize, Serialize};

use crate::error::ValidationError;
use crate::ids::{ArrangementId, MediaAssetId, ServiceId, ServiceItemId, SongId, ThemeId};
use crate::song::ReflowMode;

/// Tagged payload per service item kind so variant-specific data cannot be half-set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ServiceItemPayload {
    Song {
        song_id: SongId,
        arrangement_id: Option<ArrangementId>,
        reflow_override: Option<ReflowMode>,
    },
    Scripture {
        translation_code: String,
        reference: String,
        /// Cached `(verse_label, verse_text)` pairs so saved services remain readable offline.
        cached_verses: Vec<(String, String)>,
    },
    Presentation {
        slide_asset_ids: Vec<MediaAssetId>,
    },
    AnnouncementLoop {
        slide_asset_ids: Vec<MediaAssetId>,
        interval_secs: u16,
    },
    CustomSlide {
        body: String,
        footer: Option<String>,
    },
}

impl ServiceItemPayload {
    pub const fn db_kind(&self) -> &'static str {
        match self {
            Self::Song { .. } => "song",
            Self::Scripture { .. } => "scripture",
            Self::Presentation { .. } => "presentation",
            Self::AnnouncementLoop { .. } => "announcement_loop",
            Self::CustomSlide { .. } => "custom_slide",
        }
    }

    pub const fn badge_label(&self) -> &'static str {
        match self {
            Self::Song { .. } => "SONG",
            Self::Scripture { .. } => "BIBLE",
            Self::Presentation { .. } => "DECK",
            Self::AnnouncementLoop { .. } => "LOOP",
            Self::CustomSlide { .. } => "SLIDE",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceItem {
    pub id: ServiceItemId,
    pub title: String,
    pub theme_override_id: Option<ThemeId>,
    pub payload: ServiceItemPayload,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceSchedule {
    pub id: ServiceId,
    pub title: String,
    pub service_date: Option<String>,
    pub notes: String,
    pub items: Vec<ServiceItem>,
}

impl ServiceSchedule {
    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.title.trim().is_empty() {
            return Err(ValidationError::EmptyField {
                field: "service.title",
            });
        }
        Ok(())
    }
}
