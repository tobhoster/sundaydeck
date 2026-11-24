use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::alerts::AudienceAlert;
use crate::custom_slide::{SlideTransition, TransitionDirection};
use crate::error::PresentationError;
use crate::ids::{MediaAssetId, ServiceItemId};
use crate::slide::Slide;
use crate::theme::Theme;
use crate::timer::ServiceTimer;

/// Instant recovery override applied to the audience projector output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputOverride {
    None,
    /// Keeps theme background visible; removes foreground lyrics and audience alerts.
    ClearText,
    /// Fades entire audience output to black.
    Black,
    /// Displays the church's configured splash/logo slide.
    Logo,
}

/// Active deck loaded into either Preview or Live.
#[derive(Debug, Clone, PartialEq)]
pub struct DeckCursor {
    pub item_id: Option<ServiceItemId>,
    pub title: String,
    pub slides: Arc<[Slide]>,
    pub active_index: usize,
    pub theme: Arc<Theme>,
    /// Auto-advance interval in seconds for timed Pre-Service Announcement Loops.
    pub loop_interval_secs: Option<u16>,
}

impl DeckCursor {
    /// Constructs a validated cursor, guaranteeing at least one slide and a valid index.
    pub fn try_new(
        item_id: Option<ServiceItemId>,
        title: impl Into<String>,
        slides: Vec<Slide>,
        theme: Arc<Theme>,
    ) -> Result<Self, PresentationError> {
        let title = title.into();
        if slides.is_empty() {
            return Err(PresentationError::EmptyDeck { title });
        }
        Ok(Self {
            item_id,
            title,
            slides: Arc::from(slides),
            active_index: 0,
            theme,
            loop_interval_secs: None,
        })
    }

    /// Attaches a timed auto-loop interval in seconds (clamped to >= 2s when present).
    pub fn with_loop_interval(mut self, interval_secs: Option<u16>) -> Self {
        self.loop_interval_secs = interval_secs.map(|s| s.max(2));
        self
    }

    pub fn active_slide(&self) -> Option<&Slide> {
        self.slides.get(self.active_index)
    }

    pub fn next_slide(&self) -> Option<&Slide> {
        self.slides.get(self.active_index.saturating_add(1))
    }

    pub fn len(&self) -> usize {
        self.slides.len()
    }

    pub fn is_empty(&self) -> bool {
        self.slides.is_empty()
    }

    pub(crate) fn select(&mut self, index: usize) -> Result<(), PresentationError> {
        if index >= self.slides.len() {
            return Err(PresentationError::SlideOutOfBounds {
                index,
                len: self.slides.len(),
            });
        }
        self.active_index = index;
        Ok(())
    }
}

/// Final render state for the Audience Projector window.
/// Private stage messages are excluded from this type so they can never leak to the audience.
#[derive(Debug, Clone, PartialEq)]
pub enum AudienceRenderState {
    Black {
        transition_ms: u16,
    },
    Logo {
        asset_id: Option<MediaAssetId>,
        logo_uri: Option<String>,
        fallback_title: String,
        audience_timer: Option<ServiceTimer>,
        transition_ms: u16,
    },
    Slide {
        slide: Slide,
        theme: Arc<Theme>,
        show_text: bool,
        alert: Option<AudienceAlert>,
        audience_timer: Option<ServiceTimer>,
        transition_ms: u16,
        active_transition: SlideTransition,
        transition_direction: TransitionDirection,
    },
}

/// Final render state for the Stage / Confidence Monitor window.
#[derive(Debug, Clone, PartialEq)]
pub struct StageRenderState {
    pub live_title: Option<String>,
    pub slide_position: Option<(usize, usize)>,
    pub current_slide: Option<Slide>,
    pub next_slide: Option<Slide>,
    pub active_override: OutputOverride,
    pub stage_message: Option<String>,
    pub active_timer: Option<ServiceTimer>,
}

/// Immutable state snapshot shared across Operator, Audience, and Stage windows.
#[derive(Debug, Clone, PartialEq)]
pub struct PresentationSnapshot {
    pub version: u64,
    pub preview: Option<DeckCursor>,
    pub live: Option<DeckCursor>,
    pub output_override: OutputOverride,
    pub direct_click_live: bool,
    pub active_alert: Option<AudienceAlert>,
    pub active_timer: Option<ServiceTimer>,
    pub audience: AudienceRenderState,
    pub stage: StageRenderState,
}
