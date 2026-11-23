use serde::Deserialize;
use sundaydeck_core::{
    BibleVerse, ExtractedDocument, GeneratedServicePlan, PaginationBudget, PlanItemKind, PlanStats,
    PlannedServiceItem, ScriptureRef, ServiceItem, ServiceItemId, ServiceItemPayload, Song,
    TextDeckSlide, Theme, ThemeId, ThemeTarget, paginate_scripture, parse_scripture_ref,
    reflow_song,
};

use crate::error::PlannerError;

pub const SYSTEM_PROMPT: &str = "\
You are SundayDeck's church presentation service planner. Convert pastor sermon slides, Word notes, or rough outlines into a clean, ordered JSON service plan.
Return ONLY one valid JSON object matching this exact schema:
{
  \"sermon_title\": \"string\",
  \"items\": [
    {
      \"kind\": \"title_slide\" | \"song_match\" | \"scripture\" | \"sermon_point\" | \"announcement\" | \"closing\",
      \"title\": \"short clear schedule title\",
      \"song_title\": \"exact title from SONGS_CATALOG if this item is a song, else null\",
      \"scripture_ref\": \"canonical reference like Psalms 23:1-4 or Philippians 4:6-7 if scripture, else null\",
      \"slides\": [\"concise projection slide <= 170 chars\"],
      \"speaker_note\": \"private stage cue for operator/band, else null\"
    }
  ]
}
Rules:
1. Match BOTH explicit and implicit song hints (including author references like John Newton -> Amazing Grace or Horatio Spafford -> It Is Well with My Soul, or typos like Amazng Grce -> Amazing Grace) strictly to exact titles in SONGS_CATALOG.
2. Convert BOTH explicit and implicit Bible references (e.g. 'the shepherd psalm verses 1-4' -> 'Psalms 23:1-4', '1 Cor 13:4-7' -> '1 Corinthians 13:4-7', 'Phil 4:6-7' -> 'Philippians 4:6-7') into separate scripture items with canonical `scripture_ref`.
3. For scripture items and song_match items, keep `slides` as `[]` because SundayDeck loads the real Bible translation and song lyrics offline.
4. For sermon_point items, split dense paragraphs into 1 to 3 crisp, congregation-readable slides (max 170 chars each) while preserving the pastor's meaning.
5. Extract any operational/band/stage instructions (e.g. pause, cue pad, stand up) into `speaker_note` rather than projecting them to the audience.";

/// GBNF grammar guaranteeing strict valid JSON matching [`LlmPlanResponse`] during embedded sampling.
pub const PLAN_JSON_GBNF: &str = r#"
root ::= ws "{" ws "\"sermon_title\"" ws ":" ws string ws "," ws "\"items\"" ws ":" ws "[" ws (item (ws "," ws item)*)? ws "]" ws "}" ws
item ::= "{" ws "\"kind\"" ws ":" ws kind ws "," ws "\"title\"" ws ":" ws string ws "," ws "\"song_title\"" ws ":" ws optstr ws "," ws "\"scripture_ref\"" ws ":" ws optstr ws "," ws "\"slides\"" ws ":" ws strarr ws "," ws "\"speaker_note\"" ws ":" ws optstr ws "}"
kind ::= "\"title_slide\"" | "\"song_match\"" | "\"scripture\"" | "\"sermon_point\"" | "\"announcement\"" | "\"closing\""
optstr ::= "null" | string
strarr ::= "[" ws (string (ws "," ws string)*)? ws "]"
string ::= "\"" ([^"\\\x7F\x00-\x1F] | "\\" (["\\/bfnrt] | "u" [0-9a-fA-F] [0-9a-fA-F] [0-9a-fA-F] [0-9a-fA-F]))* "\""
ws ::= [ \t\n\r]*
"#;

#[derive(Debug, Deserialize)]
pub struct LlmPlanResponse {
    #[serde(default)]
    pub sermon_title: String,
    #[serde(default)]
    pub items: Vec<LlmPlanItem>,
}

#[derive(Debug, Deserialize)]
pub struct LlmPlanItem {
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub song_title: Option<String>,
    #[serde(default)]
    pub scripture_ref: Option<String>,
    #[serde(default, deserialize_with = "deserialize_string_or_vec")]
    pub slides: Vec<String>,
    #[serde(default)]
    pub speaker_note: Option<String>,
}

fn deserialize_string_or_vec<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrVec {
        Null,
        One(String),
        Many(Vec<String>),
    }
    match Option::<StringOrVec>::deserialize(deserializer)? {
        None | Some(StringOrVec::Null) => Ok(Vec::new()),
        Some(StringOrVec::One(s)) => Ok(vec![s]),
        Some(StringOrVec::Many(v)) => Ok(v),
    }
}

pub fn build_user_prompt(
    doc: &ExtractedDocument,
    songs: &[Song],
    themes: &[Theme],
    translation_code: &str,
) -> String {
    let mut out = String::new();
    out.push_str("SONGS_CATALOG:\n");
    for song in songs {
        out.push_str(&format!(
            "- title: \"{}\" | author: \"{}\" | ccli: \"{}\"\n",
            song.title, song.author, song.ccli_number
        ));
    }
    out.push_str("\nAVAILABLE_THEMES: ");
    let theme_names: Vec<&str> = themes.iter().map(|t| t.name.as_str()).collect();
    out.push_str(&theme_names.join(", "));
    out.push_str(&format!("\nBIBLE_TRANSLATION: {translation_code}\n"));
    out.push_str(&format!(
        "\nDOCUMENT_SOURCE: {} ({})\nEXTRACTED_BLOCKS:\n",
        doc.source_name,
        doc.format.label()
    ));

    for block in &doc.blocks {
        out.push_str(&format!("\n[Block {}]\n", block.index));
        if let Some(h) = &block.heading {
            out.push_str(&format!("Heading: {h}\n"));
        }
        for para in &block.paragraphs {
            out.push_str(&format!("Text: {para}\n"));
        }
        if let Some(note) = &block.speaker_note {
            out.push_str(&format!("SpeakerNote: {note}\n"));
        }
    }
    out
}

/// Extracts the outermost JSON object from raw model output (handles optional markdown fences).
pub fn extract_json_object(raw: &str) -> Option<&str> {
    let trimmed = raw.trim();
    let start = trimmed.find('{')?;
    let end = trimmed.rfind('}')?;
    if end < start {
        return None;
    }
    Some(&trimmed[start..=end])
}

/// Parses an LLM JSON response and materializes real SundayDeck items backed by the local
/// SQLite song library and offline Bible verse lookup (zero hallucinated lyrics or verses).
pub fn materialize_llm_plan<F>(
    raw_output: &str,
    doc: &ExtractedDocument,
    songs: &[Song],
    themes: &[Theme],
    translation_code: &str,
    mut lookup_verses: F,
) -> Result<GeneratedServicePlan, PlannerError>
where
    F: FnMut(&ScriptureRef) -> Vec<BibleVerse>,
{
    let json_slice = extract_json_object(raw_output).ok_or_else(|| PlannerError::InvalidJson {
        detail: "no JSON object braces found in model response".to_owned(),
    })?;
    let parsed: LlmPlanResponse =
        serde_json::from_str(json_slice).map_err(|err| PlannerError::InvalidJson {
            detail: err.to_string(),
        })?;
    if parsed.items.is_empty() {
        return Err(PlannerError::InvalidJson {
            detail: "model returned zero planned service items".to_owned(),
        });
    }

    let sermon_title = if parsed.sermon_title.trim().is_empty() {
        doc.title.clone()
    } else {
        parsed.sermon_title.trim().to_owned()
    };

    let mut items = Vec::new();
    let mut stats = PlanStats {
        source_blocks: doc.blocks.len(),
        ..PlanStats::default()
    };

    for (idx, entry) in parsed.items.into_iter().enumerate() {
        let kind = parse_plan_kind(&entry.kind);
        let note = clean_opt(entry.speaker_note);

        if kind == PlanItemKind::SongMatch
            && let Some(song) = resolve_catalog_song(
                entry.song_title.as_deref().unwrap_or(&entry.title),
                &entry.title,
                songs,
            )
        {
            let slides = reflow_song(song, song.default_arrangement_id, None).len();
            let theme_hint = resolve_theme_name(themes, song.default_theme_id, ThemeTarget::Song);
            let item_id = ServiceItemId::new_v7();
            if note.is_some() {
                stats.stage_notes += 1;
            }
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
                speaker_note: note,
                item: ServiceItem {
                    id: item_id,
                    title: song.title.clone(),
                    theme_override_id: song.default_theme_id,
                    payload: ServiceItemPayload::Song {
                        song_id: song.id,
                        arrangement_id: song.default_arrangement_id,
                        reflow_override: None,
                    },
                },
            });
            continue;
        }

        if kind == PlanItemKind::Scripture
            && let Some(ref_str) = entry
                .scripture_ref
                .as_deref()
                .filter(|s| !s.trim().is_empty())
                .or(Some(entry.title.as_str()))
            && let Ok(sref) = parse_scripture_ref(ref_str)
        {
            let planned =
                materialize_scripture(&sref, translation_code, themes, note, &mut lookup_verses);
            stats.scriptures_resolved += 1;
            stats.total_slides += planned.slide_count;
            if planned.speaker_note.is_some() {
                stats.stage_notes += 1;
            }
            items.push(planned);
            continue;
        }

        let title = if entry.title.trim().is_empty() {
            format!("Slide {}", idx + 1)
        } else {
            entry.title.trim().to_owned()
        };
        let raw_slides = normalize_slides(&entry.slides, &title);
        let slides: Vec<TextDeckSlide> = raw_slides
            .iter()
            .enumerate()
            .map(|(i, body)| TextDeckSlide {
                label: if raw_slides.len() == 1 {
                    title.clone()
                } else {
                    format!("{title} ({}/{})", i + 1, raw_slides.len())
                },
                body: body.clone(),
                footer: Some(sermon_title.clone()),
                speaker_note: if i == 0 { note.clone() } else { None },
            })
            .collect();

        let slide_count = slides.len().max(1);
        let (theme_id, theme_name) = pick_theme(kind, themes);
        let item_id = ServiceItemId::new_v7();
        let note_tag = if note.is_some() {
            stats.stage_notes += 1;
            " // STAGE NOTE"
        } else {
            ""
        };
        let payload = if slide_count == 1 && note.is_none() {
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
            speaker_note: note,
            item: ServiceItem {
                id: item_id,
                title,
                theme_override_id: theme_id,
                payload,
            },
        });
    }

    if items.is_empty() {
        return Err(PlannerError::InvalidJson {
            detail: "no usable items could be materialized from model JSON".to_owned(),
        });
    }

    let mut plan = GeneratedServicePlan {
        source_name: doc.source_name.clone(),
        source_format: doc.format,
        sermon_title,
        translation_code: translation_code.to_owned(),
        items,
        stats,
    };
    crate::hybrid::apply_hybrid_safety_pass(
        &mut plan,
        doc,
        songs,
        themes,
        translation_code,
        &mut lookup_verses,
    );
    Ok(plan)
}

fn materialize_scripture<F>(
    sref: &ScriptureRef,
    translation_code: &str,
    themes: &[Theme],
    speaker_note: Option<String>,
    lookup_verses: &mut F,
) -> PlannedServiceItem
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
    let (theme_id, theme_name) = pick_theme(PlanItemKind::Scripture, themes);
    let item_id = ServiceItemId::new_v7();
    let title = format!("{ref_label} ({translation_code})");
    PlannedServiceItem {
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
    }
}

fn parse_plan_kind(raw: &str) -> PlanItemKind {
    match raw.trim().to_ascii_lowercase().as_str() {
        "title_slide" | "title" => PlanItemKind::TitleSlide,
        "song_match" | "song" | "worship" => PlanItemKind::SongMatch,
        "scripture" | "bible" => PlanItemKind::Scripture,
        "announcement" | "announcements" | "welcome" => PlanItemKind::Announcement,
        "closing" | "benediction" | "prayer" => PlanItemKind::Closing,
        _ => PlanItemKind::SermonPoint,
    }
}

fn resolve_catalog_song<'a>(
    primary: &str,
    fallback_title: &str,
    songs: &'a [Song],
) -> Option<&'a Song> {
    for candidate in [primary, fallback_title] {
        let q = normalize_alnum(candidate);
        if q.len() < 4 {
            continue;
        }
        if let Some(song) = songs.iter().find(|s| {
            let t = normalize_alnum(&s.title);
            let a = normalize_alnum(&s.author);
            t == q || t.contains(&q) || q.contains(&t) || (!a.is_empty() && q.contains(&a))
        }) {
            return Some(song);
        }
    }
    None
}

fn normalize_slides(slides: &[String], fallback: &str) -> Vec<String> {
    let mut out = Vec::new();
    for s in slides {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.len() <= 220 {
            out.push(trimmed.to_owned());
        } else {
            let mut buf = String::new();
            for word in trimmed.split_whitespace() {
                if !buf.is_empty() && buf.len() + 1 + word.len() > 210 {
                    out.push(std::mem::take(&mut buf));
                }
                if !buf.is_empty() {
                    buf.push(' ');
                }
                buf.push_str(word);
            }
            if !buf.is_empty() {
                out.push(buf);
            }
        }
    }
    if out.is_empty() {
        out.push(fallback.to_owned());
    }
    out
}

fn pick_theme(kind: PlanItemKind, themes: &[Theme]) -> (Option<ThemeId>, String) {
    let preferred = match kind {
        PlanItemKind::TitleSlide => "Modern Display",
        PlanItemKind::Scripture => "Scripture Clean",
        PlanItemKind::Announcement | PlanItemKind::Closing => "Warm Sanctuary Amber",
        PlanItemKind::SongMatch | PlanItemKind::SermonPoint => "Classic Sanctuary Dark",
    };
    if let Some(t) = themes.iter().find(|t| t.name == preferred) {
        return (Some(t.id), t.name.clone());
    }
    themes.first().map_or((None, preferred.to_owned()), |t| {
        (Some(t.id), t.name.clone())
    })
}

fn resolve_theme_name(themes: &[Theme], id: Option<ThemeId>, target: ThemeTarget) -> String {
    id.and_then(|tid| themes.iter().find(|t| t.id == tid))
        .or_else(|| themes.iter().find(|t| t.target == target))
        .map_or_else(|| "Classic Sanctuary Dark".to_owned(), |t| t.name.clone())
}

fn clean_opt(val: Option<String>) -> Option<String> {
    val.map(|s| s.trim().to_owned()).filter(|s| !s.is_empty())
}

fn normalize_alnum(input: &str) -> String {
    input
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || c.is_ascii_whitespace())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}
