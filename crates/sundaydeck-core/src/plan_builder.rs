use crate::ids::ServiceItemId;
use crate::ooxml_import::{ExtractedBlock, ExtractedDocument, OfficeFormat};
use crate::plan_matchers::{
    classify_text_block, extract_scripture_refs_from_text, match_library_song,
    paginate_block_to_slides, pick_theme_for_kind, resolve_theme_name,
};
use crate::reflow::reflow_song;
use crate::scripture::{BibleVerse, PaginationBudget, ScriptureRef, paginate_scripture};
use crate::service::{ServiceItem, ServiceItemPayload};
use crate::song::Song;
use crate::theme::{Theme, ThemeTarget};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanItemKind {
    TitleSlide,
    SongMatch,
    Scripture,
    SermonPoint,
    Announcement,
    Closing,
}

impl PlanItemKind {
    pub const fn badge(self) -> &'static str {
        match self {
            Self::TitleSlide => "TITLE",
            Self::SongMatch => "SONG",
            Self::Scripture => "BIBLE",
            Self::SermonPoint => "SERMON",
            Self::Announcement => "ANNOUNCE",
            Self::Closing => "CLOSING",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlanApplyMode {
    #[default]
    ReplaceSchedule,
    AppendToSchedule,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedServiceItem {
    pub id: ServiceItemId,
    pub enabled: bool,
    pub kind: PlanItemKind,
    pub title: String,
    pub subtitle: String,
    pub slide_count: usize,
    pub theme_name_hint: String,
    pub speaker_note: Option<String>,
    pub item: ServiceItem,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PlanStats {
    pub source_blocks: usize,
    pub songs_matched: usize,
    pub scriptures_resolved: usize,
    pub sermon_decks: usize,
    pub total_slides: usize,
    pub stage_notes: usize,
}

/// Complete layered SundayDeck service plan synthesized deterministically from a pastor's
/// PowerPoint (`.pptx`), Word document (`.docx`), or plain sermon outline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedServicePlan {
    pub source_name: String,
    pub source_format: OfficeFormat,
    pub sermon_title: String,
    pub translation_code: String,
    pub items: Vec<PlannedServiceItem>,
    pub stats: PlanStats,
}

impl GeneratedServicePlan {
    pub fn enabled_items(&self) -> Vec<ServiceItem> {
        self.items
            .iter()
            .filter(|entry| entry.enabled)
            .map(|entry| entry.item.clone())
            .collect()
    }
}

/// Deterministically builds a layered service plan from an [`ExtractedDocument`],
/// linking library songs, resolving offline SQLite Bible passages, auto-paginating long
/// pastor sermon paragraphs into congregation-ready slides, and assigning curated OFL themes.
pub fn build_service_plan<F>(
    doc: &ExtractedDocument,
    songs: &[Song],
    themes: &[Theme],
    translation_code: &str,
    mut lookup_verses: F,
) -> GeneratedServicePlan
where
    F: FnMut(&ScriptureRef) -> Vec<BibleVerse>,
{
    let mut items = Vec::new();
    let mut stats = PlanStats {
        source_blocks: doc.blocks.len(),
        ..PlanStats::default()
    };

    for (idx, block) in doc.blocks.iter().enumerate() {
        let heading = block.heading.as_deref().unwrap_or("").trim();
        let combined_text = join_block_text(block);

        if let Some(song) = match_library_song(heading, &block.paragraphs, songs) {
            let slides = reflow_song(song, song.default_arrangement_id, None).len();
            let theme_hint = resolve_theme_name(themes, song.default_theme_id, ThemeTarget::Song);
            let item_id = ServiceItemId::new_v7();
            let item = ServiceItem {
                id: item_id,
                title: song.title.clone(),
                theme_override_id: song.default_theme_id,
                payload: ServiceItemPayload::Song {
                    song_id: song.id,
                    arrangement_id: song.default_arrangement_id,
                    reflow_override: None,
                },
            };
            stats.songs_matched += 1;
            stats.total_slides += slides;
            items.push(PlannedServiceItem {
                id: item_id,
                enabled: true,
                kind: PlanItemKind::SongMatch,
                title: song.title.clone(),
                subtitle: format!(
                    "LIBRARY SONG // {slides:02} SLIDES // CCLI #{}",
                    song.ccli_number
                ),
                slide_count: slides,
                theme_name_hint: theme_hint,
                speaker_note: block.speaker_note.clone(),
                item,
            });
            continue;
        }

        let refs = extract_scripture_refs_from_text(&combined_text);
        if !refs.is_empty() && is_pure_scripture_block(heading, &block.paragraphs) {
            for sref in refs {
                if let Some(planned) = build_planned_scripture(
                    &sref,
                    translation_code,
                    themes,
                    block.speaker_note.clone(),
                    &mut lookup_verses,
                ) {
                    stats.scriptures_resolved += 1;
                    stats.total_slides += planned.slide_count;
                    if planned.speaker_note.is_some() {
                        stats.stage_notes += 1;
                    }
                    items.push(planned);
                }
            }
            continue;
        }

        let kind = classify_text_block(idx, heading, &block.paragraphs);
        let (theme_id, theme_name) = pick_theme_for_kind(kind, themes);
        let slides = paginate_block_to_slides(
            kind,
            heading,
            &block.paragraphs,
            &doc.title,
            block.speaker_note.as_deref(),
        );
        if !slides.is_empty() {
            let item_id = ServiceItemId::new_v7();
            let title = if !heading.is_empty() {
                heading.to_owned()
            } else {
                format!("Slide {}", block.index)
            };
            let slide_count = slides.len();
            let note_tag = if block.speaker_note.is_some() {
                stats.stage_notes += 1;
                " // STAGE NOTE"
            } else {
                ""
            };
            let payload = if slide_count == 1 && block.speaker_note.is_none() {
                let first = &slides[0];
                ServiceItemPayload::CustomSlide {
                    body: first.body.clone(),
                    footer: first.footer.clone(),
                }
            } else {
                ServiceItemPayload::TextDeck { slides }
            };
            if kind == PlanItemKind::SermonPoint {
                stats.sermon_decks += 1;
            }
            stats.total_slides += slide_count;
            items.push(PlannedServiceItem {
                id: item_id,
                enabled: true,
                kind,
                title: title.clone(),
                subtitle: format!(
                    "{slide_count:02} SLIDE(S) // {}{note_tag}",
                    theme_name.to_ascii_uppercase()
                ),
                slide_count,
                theme_name_hint: theme_name,
                speaker_note: block.speaker_note.clone(),
                item: ServiceItem {
                    id: item_id,
                    title,
                    theme_override_id: theme_id,
                    payload,
                },
            });
        }

        for sref in refs {
            if let Some(planned) =
                build_planned_scripture(&sref, translation_code, themes, None, &mut lookup_verses)
            {
                stats.scriptures_resolved += 1;
                stats.total_slides += planned.slide_count;
                items.push(planned);
            }
        }
    }

    GeneratedServicePlan {
        source_name: doc.source_name.clone(),
        source_format: doc.format,
        sermon_title: doc.title.clone(),
        translation_code: translation_code.to_owned(),
        items,
        stats,
    }
}

fn build_planned_scripture<F>(
    sref: &ScriptureRef,
    translation_code: &str,
    themes: &[Theme],
    speaker_note: Option<String>,
    lookup_verses: &mut F,
) -> Option<PlannedServiceItem>
where
    F: FnMut(&ScriptureRef) -> Vec<BibleVerse>,
{
    let verses = lookup_verses(sref);
    let ref_label = sref.format_display();
    let cached_verses: Vec<(String, String)> = if verses.is_empty() {
        vec![(
            sref.verse_start.unwrap_or(1).to_string(),
            format!("[{ref_label} not installed in local {translation_code} store]"),
        )]
    } else {
        verses
            .iter()
            .map(|v| (v.verse.to_string(), v.text.clone()))
            .collect()
    };

    let slide_count = if verses.is_empty() {
        1
    } else {
        paginate_scripture(&verses, PaginationBudget::default())
            .len()
            .max(1)
    };
    let (theme_id, theme_name) = pick_theme_for_kind(PlanItemKind::Scripture, themes);
    let item_id = ServiceItemId::new_v7();
    let title = format!("{ref_label} ({translation_code})");

    Some(PlannedServiceItem {
        id: item_id,
        enabled: true,
        kind: PlanItemKind::Scripture,
        title: title.clone(),
        subtitle: format!(
            "{translation_code} // {:02} VERSE(S) -> {slide_count:02} SLIDE(S)",
            cached_verses.len()
        ),
        slide_count,
        theme_name_hint: theme_name,
        speaker_note,
        item: ServiceItem {
            id: item_id,
            title,
            theme_override_id: theme_id,
            payload: ServiceItemPayload::Scripture {
                translation_code: translation_code.to_owned(),
                reference: ref_label,
                cached_verses,
            },
        },
    })
}

fn is_pure_scripture_block(heading: &str, paragraphs: &[String]) -> bool {
    let lower_heading = heading.to_ascii_lowercase();
    let body_len: usize = paragraphs.iter().map(String::len).sum();
    if lower_heading.starts_with("scripture")
        || lower_heading.starts_with("bible")
        || lower_heading.starts_with("reading")
    {
        return body_len <= 140;
    }
    if paragraphs.is_empty() {
        return true;
    }
    let looks_like_sermon_point = lower_heading.starts_with("point ")
        || lower_heading.starts_with("step ")
        || lower_heading.starts_with("part ")
        || lower_heading.starts_with("application");
    !looks_like_sermon_point && paragraphs.len() == 1 && body_len <= 90
}

fn join_block_text(block: &ExtractedBlock) -> String {
    let mut parts = Vec::new();
    if let Some(h) = &block.heading {
        parts.push(h.as_str());
    }
    for p in &block.paragraphs {
        parts.push(p.as_str());
    }
    parts.join(" ")
}
