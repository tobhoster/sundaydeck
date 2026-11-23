use std::collections::HashSet;

use crate::bible_books::CANONICAL_BOOKS;
use crate::ids::ThemeId;
use crate::plan_builder::PlanItemKind;
use crate::scripture::{ScriptureRef, parse_scripture_ref};
use crate::service::TextDeckSlide;
use crate::song::Song;
use crate::theme::{Theme, ThemeTarget};

const MAX_SLIDE_CHARS: usize = 210;
const MAX_SLIDE_LINES: usize = 4;

/// Scans arbitrary prose (headings, slide bullets, Word paragraphs) and extracts every valid,
/// deduplicated [`ScriptureRef`] using the canonical 66-book table.
pub fn extract_scripture_refs_from_text(text: &str) -> Vec<ScriptureRef> {
    let tokens: Vec<&str> = text.split_whitespace().collect();
    let mut found = Vec::new();
    let mut seen: HashSet<(u8, u16, Option<u16>, Option<u16>)> = HashSet::new();

    let mut i = 0;
    while i < tokens.len() {
        let mut matched_span = 0usize;
        for len in (2..=4).rev() {
            if i + len > tokens.len() {
                continue;
            }
            let last_clean = trim_ref_edges(tokens[i + len - 1]);
            if !last_clean.contains(':') {
                continue;
            }
            let raw_window = tokens[i..i + len]
                .iter()
                .map(|t| trim_ref_edges(t))
                .collect::<Vec<_>>()
                .join(" ");
            if !starts_with_known_book_prefix(&raw_window) {
                continue;
            }
            if let Ok(parsed) = parse_scripture_ref(&raw_window) {
                let key = (
                    parsed.book_number,
                    parsed.chapter,
                    parsed.verse_start,
                    parsed.verse_end,
                );
                if seen.insert(key) {
                    found.push(parsed);
                }
                matched_span = len;
                break;
            }
        }
        i += matched_span.max(1);
    }
    found
}

/// Realistic multi-part Sunday Pastor Sermon outline for 1-click preview/testing in the UI.
pub fn sample_pastor_sermon_outline() -> &'static str {
    "# Walking in Unshakable Peace\n\
     Sunday Morning Series — Week 3\n\
     Grace Fellowship\n\
     \n\
     # Announcements & Welcome\n\
     - Kids Check-In open in West Hall until 10:15 AM\n\
     - Community Prayer Night — Wednesday at 7:00 PM\n\
     - Youth Autumn Retreat registration closes Friday\n\
     \n\
     # Opening Song: Amazing Grace\n\
     \n\
     # Scripture: Psalm 23:1-3\n\
     \n\
     # Point 1: Peace Begins with Trust, Not Control\n\
     When anxiety rises, our instinct is to tighten our grip on outcomes we were never built to carry. Scripture invites us instead to lay down the illusion of control and rest in the Shepherd who leads us beside still waters.\n\
     Read Philippians 4:6-7 together as a congregation.\n\
     Note: Pause 5 seconds after reading before advancing to Point 2.\n\
     \n\
     # Point 2: Love Sustains Us Through the Valley\n\
     Biblical love is neither passive sentiment nor fragile emotion. It endures every season with patience, kindness, and steady courage rooted in Christ.\n\
     Key Passage: 1 Corinthians 13:4-7\n\
     Note: Worship keys pad softly underneath.\n\
     \n\
     # Closing Song: It Is Well with My Soul\n\
     \n\
     # Closing Prayer & Benediction\n\
     May the peace of God, which surpasses all understanding,\n\
     guard your hearts and minds in Christ Jesus.\n"
}

pub(crate) fn match_library_song<'a>(
    heading: &str,
    paragraphs: &[String],
    songs: &'a [Song],
) -> Option<&'a Song> {
    let lower_heading = heading.to_ascii_lowercase();
    let explicit_song = strip_song_prefix(&lower_heading);
    let candidate = explicit_song.unwrap_or(&lower_heading).trim();

    if !candidate.is_empty()
        && let Some(found) = find_song_by_query(candidate, songs)
    {
        return Some(found);
    }
    if explicit_song.is_some()
        && let Some(first_para) = paragraphs.first()
    {
        return find_song_by_query(&first_para.to_ascii_lowercase(), songs);
    }
    None
}

pub(crate) fn classify_text_block(
    idx: usize,
    heading: &str,
    paragraphs: &[String],
) -> PlanItemKind {
    let lower = heading.to_ascii_lowercase();
    let all_lower = format!("{} {}", lower, paragraphs.join(" ").to_ascii_lowercase());

    if idx == 0 && paragraphs.len() <= 3 && all_lower.len() <= 160 {
        return PlanItemKind::TitleSlide;
    }
    if lower.contains("announcement")
        || lower.contains("welcome")
        || lower.contains("this week")
        || lower.contains("events")
        || lower.contains("check-in")
    {
        return PlanItemKind::Announcement;
    }
    if lower.contains("closing")
        || lower.contains("benediction")
        || lower.contains("dismissal")
        || lower.contains("prayer")
    {
        return PlanItemKind::Closing;
    }
    PlanItemKind::SermonPoint
}

pub(crate) fn paginate_block_to_slides(
    kind: PlanItemKind,
    heading: &str,
    paragraphs: &[String],
    sermon_title: &str,
    speaker_note: Option<&str>,
) -> Vec<TextDeckSlide> {
    if kind == PlanItemKind::TitleSlide {
        let mut lines = Vec::new();
        if !heading.is_empty() {
            lines.push(heading.to_owned());
        }
        lines.extend(paragraphs.iter().take(2).cloned());
        let footer = paragraphs.get(2).cloned();
        return vec![TextDeckSlide {
            label: "Title".to_owned(),
            body: lines.join("\n"),
            footer,
            speaker_note: speaker_note.map(ToOwned::to_owned),
        }];
    }

    let chunks = chunk_paragraphs_for_projection(paragraphs);
    if chunks.is_empty() {
        if heading.is_empty() {
            return Vec::new();
        }
        return vec![TextDeckSlide {
            label: heading.to_owned(),
            body: heading.to_owned(),
            footer: Some(sermon_title.to_owned()),
            speaker_note: speaker_note.map(ToOwned::to_owned),
        }];
    }

    let total = chunks.len();
    chunks
        .into_iter()
        .enumerate()
        .map(|(i, chunk)| {
            let label = if heading.is_empty() {
                format!("Slide {}", i + 1)
            } else if total == 1 {
                heading.to_owned()
            } else {
                format!("{heading} ({}/{total})", i + 1)
            };
            let body = if i == 0 && !heading.is_empty() && kind == PlanItemKind::SermonPoint {
                format!("{heading}\n{chunk}")
            } else {
                chunk
            };
            TextDeckSlide {
                label,
                body,
                footer: Some(sermon_title.to_owned()),
                speaker_note: if i == 0 {
                    speaker_note.map(ToOwned::to_owned)
                } else {
                    None
                },
            }
        })
        .collect()
}

pub(crate) fn pick_theme_for_kind(
    kind: PlanItemKind,
    themes: &[Theme],
) -> (Option<ThemeId>, String) {
    let preferred_name = match kind {
        PlanItemKind::TitleSlide => "Modern Display",
        PlanItemKind::Scripture => "Scripture Clean",
        PlanItemKind::Announcement | PlanItemKind::Closing => "Warm Sanctuary Amber",
        PlanItemKind::SongMatch | PlanItemKind::SermonPoint => "Classic Sanctuary Dark",
    };
    if let Some(theme) = themes.iter().find(|t| t.name == preferred_name) {
        return (Some(theme.id), theme.name.clone());
    }
    let fallback_target = match kind {
        PlanItemKind::Scripture => ThemeTarget::Scripture,
        PlanItemKind::SongMatch => ThemeTarget::Song,
        _ => ThemeTarget::General,
    };
    if let Some(theme) = themes
        .iter()
        .find(|t| t.target == fallback_target)
        .or_else(|| themes.first())
    {
        return (Some(theme.id), theme.name.clone());
    }
    (None, preferred_name.to_owned())
}

pub(crate) fn resolve_theme_name(
    themes: &[Theme],
    id: Option<ThemeId>,
    target: ThemeTarget,
) -> String {
    id.and_then(|tid| themes.iter().find(|t| t.id == tid))
        .or_else(|| themes.iter().find(|t| t.target == target))
        .map_or_else(|| "Classic Sanctuary Dark".to_owned(), |t| t.name.clone())
}

fn find_song_by_query<'a>(query: &str, songs: &'a [Song]) -> Option<&'a Song> {
    let q_norm = normalize_title(query);
    if q_norm.len() < 4 {
        return None;
    }
    songs.iter().find(|song| {
        let title_norm = normalize_title(&song.title);
        let alt_norm = normalize_title(&song.alternate_title);
        title_norm == q_norm
            || (!alt_norm.is_empty() && alt_norm == q_norm)
            || (q_norm.len() >= 6 && title_norm.contains(&q_norm))
            || (title_norm.len() >= 6 && q_norm.contains(&title_norm))
    })
}

fn strip_song_prefix(lower: &str) -> Option<&str> {
    for prefix in [
        "opening song:",
        "closing song:",
        "worship song:",
        "opening hymn:",
        "closing hymn:",
        "song:",
        "hymn:",
        "worship:",
        "sing:",
    ] {
        if let Some(rest) = lower.strip_prefix(prefix) {
            return Some(rest.trim());
        }
    }
    None
}

fn chunk_paragraphs_for_projection(paragraphs: &[String]) -> Vec<String> {
    let mut sentences: Vec<String> = Vec::new();
    for para in paragraphs {
        if para.len() <= MAX_SLIDE_CHARS {
            sentences.push(para.trim().to_owned());
            continue;
        }
        for sentence in split_sentences(para) {
            if !sentence.is_empty() {
                sentences.push(sentence);
            }
        }
    }

    let mut slides: Vec<String> = Vec::new();
    let mut current: Vec<String> = Vec::new();
    let mut current_chars = 0usize;

    for part in sentences {
        let len = part.len();
        if !current.is_empty()
            && (current_chars + len + 1 > MAX_SLIDE_CHARS || current.len() >= MAX_SLIDE_LINES)
        {
            slides.push(current.join("\n"));
            current.clear();
            current_chars = 0;
        }
        current_chars += len + 1;
        current.push(part);
    }
    if !current.is_empty() {
        slides.push(current.join("\n"));
    }
    slides
}

fn split_sentences(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for ch in text.chars() {
        cur.push(ch);
        if matches!(ch, '.' | '!' | '?') && cur.len() >= 30 {
            out.push(cur.trim().to_owned());
            cur.clear();
        }
    }
    if !cur.trim().is_empty() {
        out.push(cur.trim().to_owned());
    }
    out
}

fn trim_ref_edges(token: &str) -> &str {
    token.trim_matches(|c: char| matches!(c, '(' | ')' | '[' | ']' | ',' | '.' | ';' | '"' | '\''))
}

fn normalize_title(raw: &str) -> String {
    raw.chars()
        .filter(|c| c.is_ascii_alphanumeric() || c.is_ascii_whitespace())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}

fn starts_with_known_book_prefix(window: &str) -> bool {
    let lower = window.to_ascii_lowercase();
    let first = lower.split_whitespace().next().unwrap_or("");
    if matches!(first, "1" | "2" | "3" | "i" | "ii" | "iii") {
        let second = lower.split_whitespace().nth(1).unwrap_or("");
        return CANONICAL_BOOKS.iter().any(|b| {
            b.name.to_ascii_lowercase().contains(second)
                || b.aliases.iter().any(|a| a.starts_with(second))
        });
    }
    CANONICAL_BOOKS.iter().any(|b| {
        b.name.to_ascii_lowercase().starts_with(first)
            || b.osis_id.to_ascii_lowercase() == first
            || b.aliases.contains(&first)
    })
}
