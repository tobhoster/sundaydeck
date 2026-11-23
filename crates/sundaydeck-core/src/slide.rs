use serde::{Deserialize, Serialize};

use crate::ids::{MediaAssetId, SectionId, SlideId};
use crate::song::SectionKind;

/// Renderable content carried by a single resolved slide.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SlideContent {
    Text {
        body: String,
        section_label: Option<String>,
        section_kind: Option<SectionKind>,
        footer: Option<String>,
    },
    Image {
        asset_id: MediaAssetId,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        uri: Option<String>,
        caption: Option<String>,
    },
    Blank,
}

impl SlideContent {
    /// Returns the primary display text for thumbnails and stage confidence monitors.
    pub fn primary_text(&self) -> &str {
        match self {
            Self::Text { body, .. } => body.as_str(),
            Self::Image { caption, .. } => caption.as_deref().unwrap_or("[Image Slide]"),
            Self::Blank => "",
        }
    }
}

/// Immutable presentation-ready slide produced by song reflow, scripture pagination, or media decks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Slide {
    pub id: SlideId,
    pub source_section_id: Option<SectionId>,
    pub label: String,
    pub content: SlideContent,
}
