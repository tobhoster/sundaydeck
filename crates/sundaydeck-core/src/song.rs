use serde::{Deserialize, Serialize};

use crate::error::ValidationError;
use crate::global_content::HymnalEntry;
use crate::ids::{ArrangementId, SectionId, SongId, ThemeId};

/// Structural role of a reusable song section.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SectionKind {
    Intro,
    Verse,
    PreChorus,
    Chorus,
    Bridge,
    Tag,
    Interlude,
    Ending,
    Other,
}

impl SectionKind {
    pub const fn as_db_str(self) -> &'static str {
        match self {
            Self::Intro => "intro",
            Self::Verse => "verse",
            Self::PreChorus => "pre_chorus",
            Self::Chorus => "chorus",
            Self::Bridge => "bridge",
            Self::Tag => "tag",
            Self::Interlude => "interlude",
            Self::Ending => "ending",
            Self::Other => "other",
        }
    }

    pub fn from_db_str(raw: &str) -> Result<Self, ValidationError> {
        match raw {
            "intro" => Ok(Self::Intro),
            "verse" => Ok(Self::Verse),
            "pre_chorus" => Ok(Self::PreChorus),
            "chorus" => Ok(Self::Chorus),
            "bridge" => Ok(Self::Bridge),
            "tag" => Ok(Self::Tag),
            "interlude" => Ok(Self::Interlude),
            "ending" => Ok(Self::Ending),
            "other" => Ok(Self::Other),
            other => Err(ValidationError::UnknownVariant {
                field: "song_section.kind",
                raw: other.to_owned(),
            }),
        }
    }

    /// Short uppercase badge label rendered on slide thumbnails.
    pub const fn short_tag(self) -> &'static str {
        match self {
            Self::Intro => "INT",
            Self::Verse => "VRS",
            Self::PreChorus => "PRE",
            Self::Chorus => "CHR",
            Self::Bridge => "BRG",
            Self::Tag => "TAG",
            Self::Interlude => "ITL",
            Self::Ending => "END",
            Self::Other => "SEC",
        }
    }
}

/// Non-destructive lyric pagination policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReflowMode {
    ByStanza,
    Lines1,
    Lines2,
    Lines4,
}

impl ReflowMode {
    pub const ALL: [Self; 4] = [Self::ByStanza, Self::Lines1, Self::Lines2, Self::Lines4];

    pub const fn as_db_str(self) -> &'static str {
        match self {
            Self::ByStanza => "by_stanza",
            Self::Lines1 => "lines_1",
            Self::Lines2 => "lines_2",
            Self::Lines4 => "lines_4",
        }
    }

    pub fn from_db_str(raw: &str) -> Result<Self, ValidationError> {
        match raw {
            "by_stanza" => Ok(Self::ByStanza),
            "lines_1" => Ok(Self::Lines1),
            "lines_2" => Ok(Self::Lines2),
            "lines_4" => Ok(Self::Lines4),
            other => Err(ValidationError::UnknownVariant {
                field: "reflow_mode",
                raw: other.to_owned(),
            }),
        }
    }

    pub const fn display_label(self) -> &'static str {
        match self {
            Self::ByStanza => "Stanza",
            Self::Lines1 => "1 Line",
            Self::Lines2 => "2 Lines",
            Self::Lines4 => "4 Lines",
        }
    }

    pub const fn max_lines_per_slide(self) -> Option<usize> {
        match self {
            Self::ByStanza => None,
            Self::Lines1 => Some(1),
            Self::Lines2 => Some(2),
            Self::Lines4 => Some(4),
        }
    }
}

fn default_lang_en() -> String {
    "en".to_owned()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SongSection {
    pub id: SectionId,
    pub kind: SectionKind,
    pub label: String,
    pub ordinal: u32,
    pub body: String,
    /// Optional paired second-language lyrics (e.g. Yoruba + English or Spanish + English).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secondary_body: Option<String>,
    /// Optional chord-annotated lyrics surfaced only on the Stage Confidence Monitor.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stage_chords: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Arrangement {
    pub id: ArrangementId,
    pub name: String,
    pub section_ids: Vec<SectionId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Song {
    pub id: SongId,
    pub title: String,
    pub alternate_title: String,
    pub author: String,
    pub copyright: String,
    pub ccli_number: String,
    pub default_theme_id: Option<ThemeId>,
    pub default_arrangement_id: Option<ArrangementId>,
    pub default_reflow_mode: ReflowMode,
    #[serde(default = "default_lang_en")]
    pub primary_lang: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secondary_lang: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hymnal_entries: Vec<HymnalEntry>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub traditions: Vec<String>,
    pub sections: Vec<SongSection>,
    pub arrangements: Vec<Arrangement>,
}

impl Song {
    /// Materializes a single normalized lyric string (including secondary language + ASCII-folded
    /// search text + hymn numbers) for SQLite FTS5 indexing.
    pub fn lyrics_search_text(&self) -> String {
        let mut parts: Vec<String> = Vec::new();
        for h in &self.hymnal_entries {
            parts.push(format!("{} {} #{}", h.hymnal_code, h.hymn_number, h.hymn_number));
        }
        for section in &self.sections {
            let trimmed = section.body.trim();
            if !trimmed.is_empty() {
                parts.push(trimmed.to_owned());
                let folded = crate::bible_aliases::strip_diacritics(trimmed);
                if folded != trimmed.to_ascii_lowercase() {
                    parts.push(folded);
                }
            }
            if let Some(sec) = section.secondary_body.as_deref()
                && !sec.trim().is_empty()
            {
                parts.push(sec.trim().to_owned());
            }
        }
        parts.join("\n")
    }

    /// Resolves the requested arrangement, falling back to default or first available.
    pub fn resolve_arrangement(&self, requested: Option<ArrangementId>) -> Option<&Arrangement> {
        let target_id = requested.or(self.default_arrangement_id);
        target_id
            .and_then(|id| self.arrangements.iter().find(|arr| arr.id == id))
            .or_else(|| self.arrangements.first())
    }

    pub fn section_by_id(&self, id: SectionId) -> Option<&SongSection> {
        self.sections.iter().find(|section| section.id == id)
    }

    /// Formats a clean CCLI/author footer line for display at the bottom of song slides.
    pub fn formatted_footer(&self) -> Option<String> {
        let mut parts: Vec<String> = Vec::new();
        if !self.author.trim().is_empty() {
            parts.push(self.author.trim().to_owned());
        }
        if !self.ccli_number.trim().is_empty() {
            parts.push(format!("CCLI #{}", self.ccli_number.trim()));
        }
        if parts.is_empty() {
            None
        } else {
            Some(parts.join("  •  "))
        }
    }
}

/// Lightweight library listing row returned by FTS5 song search.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SongSummary {
    pub id: SongId,
    pub title: String,
    pub author: String,
    pub ccli_number: String,
    pub section_count: usize,
}
