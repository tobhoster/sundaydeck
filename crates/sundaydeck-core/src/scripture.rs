use serde::{Deserialize, Serialize};

use crate::bible_books::{book_by_number, match_book_name};
use crate::error::ScriptureParseError;
use crate::ids::{BibleTranslationId, SUNDAYDECK_NAMESPACE, SlideId};
use crate::slide::{Slide, SlideContent};

/// Metadata for an installed offline Bible translation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BibleTranslation {
    pub id: BibleTranslationId,
    pub code: String,
    pub name: String,
    pub language: String,
    pub copyright: String,
}

/// A single Bible verse row returned from offline storage or FTS5 keyword search.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BibleVerse {
    pub translation_code: String,
    pub book_number: u8,
    pub book_name: String,
    pub chapter: u16,
    pub verse: u16,
    pub text: String,
}

/// Parsed canonical Bible reference (single verse, verse range, or whole chapter).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ScriptureRef {
    pub book_number: u8,
    pub chapter: u16,
    pub verse_start: Option<u16>,
    pub verse_end: Option<u16>,
}

impl ScriptureRef {
    pub fn format_display(&self) -> String {
        let book_name = book_by_number(self.book_number)
            .map(|b| b.name)
            .unwrap_or("Unknown");
        match (self.verse_start, self.verse_end) {
            (Some(start), Some(end)) if end > start => {
                format!("{book_name} {}:{start}-{end}", self.chapter)
            }
            (Some(start), _) => format!("{book_name} {}:{start}", self.chapter),
            (None, _) => format!("{book_name} {}", self.chapter),
        }
    }
}

/// Character/verse budget controlling how passages auto-paginate across slides.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaginationBudget {
    pub max_chars_per_slide: usize,
    pub max_verses_per_slide: usize,
}

impl Default for PaginationBudget {
    fn default() -> Self {
        Self {
            max_chars_per_slide: 240,
            max_verses_per_slide: 2,
        }
    }
}

/// Parses shorthand and full Bible references (`jn 3:16`, `1cor 13:4-7`, `ps 23`, `gen 1:1-3`, `jude 24`).
pub fn parse_scripture_ref(raw: &str) -> Result<ScriptureRef, ScriptureParseError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(ScriptureParseError::Empty);
    }

    let (book_part, numeric_part) = split_book_and_numbers(trimmed)?;
    let meta = match_book_name(book_part).ok_or_else(|| ScriptureParseError::UnknownBook {
        book: book_part.to_owned(),
    })?;

    let cleaned_nums = numeric_part.trim();
    if cleaned_nums.is_empty() {
        return Err(ScriptureParseError::MissingChapter {
            raw: trimmed.to_owned(),
        });
    }

    if let Some((chap_str, verse_part)) = split_chapter_verse(cleaned_nums) {
        let chapter = parse_u16_token(chap_str)?;
        let (start, end) = parse_verse_range(verse_part)?;
        Ok(ScriptureRef {
            book_number: meta.number,
            chapter,
            verse_start: Some(start),
            verse_end: end,
        })
    } else if meta.single_chapter {
        // Single-chapter books (e.g. `jude 24` or `jude 24-25`) default to chapter 1.
        let (start, end) = parse_verse_range(cleaned_nums)?;
        Ok(ScriptureRef {
            book_number: meta.number,
            chapter: 1,
            verse_start: Some(start),
            verse_end: end,
        })
    } else {
        let chapter = parse_u16_token(cleaned_nums)?;
        Ok(ScriptureRef {
            book_number: meta.number,
            chapter,
            verse_start: None,
            verse_end: None,
        })
    }
}

/// Splits a sequence of Bible verses cleanly across slides at sentence or word boundaries,
/// attaching verse numbers and formatted scripture citation footers.
pub fn paginate_scripture(verses: &[BibleVerse], budget: PaginationBudget) -> Vec<Slide> {
    if verses.is_empty() {
        return Vec::new();
    }

    let max_chars = budget.max_chars_per_slide.max(40);
    let max_verses = budget.max_verses_per_slide.max(1);
    let mut slides = Vec::new();
    let mut slide_idx: usize = 0;

    let mut pending_text = String::new();
    let mut pending_verses: Vec<&BibleVerse> = Vec::new();

    let flush = |pending_text: &mut String,
                 pending_verses: &mut Vec<&BibleVerse>,
                 slides: &mut Vec<Slide>,
                 slide_idx: &mut usize| {
        if pending_text.is_empty() || pending_verses.is_empty() {
            return;
        }
        let first = pending_verses[0];
        let last = pending_verses[pending_verses.len() - 1];
        let ref_label = format_verse_span(first, last);
        let footer = format!("{ref_label} ({})", first.translation_code);
        let id = SlideId::from_v5(
            &SUNDAYDECK_NAMESPACE,
            format!(
                "scripture:{}:{}:{slide_idx}",
                first.translation_code, ref_label
            )
            .as_bytes(),
        );
        slides.push(Slide {
            id,
            source_section_id: None,
            label: ref_label.clone(),
            content: SlideContent::Text {
                body: std::mem::take(pending_text),
                section_label: Some(ref_label),
                section_kind: None,
                footer: Some(footer),
            },
            transition_override: None,
        });
        pending_verses.clear();
        *slide_idx += 1;
    };

    for verse in verses {
        let formatted_verse = format!("{}  {}", verse.verse, verse.text.trim());
        if formatted_verse.chars().count() > max_chars {
            flush(
                &mut pending_text,
                &mut pending_verses,
                &mut slides,
                &mut slide_idx,
            );
            let chunks = split_long_text(&formatted_verse, max_chars);
            let chunk_total = chunks.len();
            for (c_idx, chunk) in chunks.into_iter().enumerate() {
                let base_ref = format_verse_span(verse, verse);
                let label = if chunk_total > 1 {
                    format!("{base_ref} ({}/{})", c_idx + 1, chunk_total)
                } else {
                    base_ref.clone()
                };
                let footer = format!("{base_ref} ({})", verse.translation_code);
                let id = SlideId::from_v5(
                    &SUNDAYDECK_NAMESPACE,
                    format!(
                        "scripture:{}:{}:{slide_idx}:{c_idx}",
                        verse.translation_code, base_ref
                    )
                    .as_bytes(),
                );
                slides.push(Slide {
                    id,
                    source_section_id: None,
                    label,
                    content: SlideContent::Text {
                        body: chunk,
                        section_label: Some(base_ref),
                        section_kind: None,
                        footer: Some(footer),
                    },
                    transition_override: None,
                });
                slide_idx += 1;
            }
            continue;
        }

        let candidate_len = if pending_text.is_empty() {
            formatted_verse.chars().count()
        } else {
            pending_text.chars().count() + 1 + formatted_verse.chars().count()
        };

        if !pending_verses.is_empty()
            && (pending_verses.len() >= max_verses || candidate_len > max_chars)
        {
            flush(
                &mut pending_text,
                &mut pending_verses,
                &mut slides,
                &mut slide_idx,
            );
        }

        if !pending_text.is_empty() {
            pending_text.push('\n');
        }
        pending_text.push_str(&formatted_verse);
        pending_verses.push(verse);
    }

    flush(
        &mut pending_text,
        &mut pending_verses,
        &mut slides,
        &mut slide_idx,
    );
    slides
}

/// Paginates a paired Primary + Secondary Bible passage (e.g. Spanish *RVR1909* + English *WEB*,
/// or Portuguese *Almeida* + English *KJV*) cleanly into dual-language stacked slides.
pub fn paginate_parallel_scripture(primary: &[BibleVerse], secondary: &[BibleVerse]) -> Vec<Slide> {
    primary
        .iter()
        .enumerate()
        .map(|(idx, p)| {
            let paired = secondary.iter().find(|s| {
                s.book_number == p.book_number && s.chapter == p.chapter && s.verse == p.verse
            });
            let ref_label = format!("{} {}:{}", p.book_name, p.chapter, p.verse);
            let (body, footer) = match paired {
                Some(sec) => (
                    format!(
                        "{}  {}\n\u{2014}\n{}  {}",
                        p.verse,
                        p.text.trim(),
                        sec.verse,
                        sec.text.trim()
                    ),
                    format!(
                        "{ref_label} ({} / {})",
                        p.translation_code, sec.translation_code
                    ),
                ),
                None => (
                    format!("{}  {}", p.verse, p.text.trim()),
                    format!("{ref_label} ({})", p.translation_code),
                ),
            };
            Slide {
                id: SlideId::from_v5(
                    &SUNDAYDECK_NAMESPACE,
                    format!("parallel:{}:{ref_label}:{idx}", p.translation_code).as_bytes(),
                ),
                source_section_id: None,
                label: ref_label.clone(),
                content: SlideContent::Text {
                    body,
                    section_label: Some(ref_label),
                    section_kind: None,
                    footer: Some(footer),
                },
                transition_override: None,
            }
        })
        .collect()
}

fn format_verse_span(first: &BibleVerse, last: &BibleVerse) -> String {
    if first.book_number == last.book_number && first.chapter == last.chapter {
        if first.verse == last.verse {
            format!("{} {}:{}", first.book_name, first.chapter, first.verse)
        } else {
            format!(
                "{} {}:{}-{}",
                first.book_name, first.chapter, first.verse, last.verse
            )
        }
    } else {
        format!(
            "{} {}:{} - {} {}:{}",
            first.book_name, first.chapter, first.verse, last.book_name, last.chapter, last.verse
        )
    }
}

fn split_book_and_numbers(input: &str) -> Result<(&str, &str), ScriptureParseError> {
    // Walk past optional leading book number (`1 `, `2 `, `3 ` or `1cor`), then find
    // the boundary where the chapter/verse digits start.
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    if i < bytes.len() && matches!(bytes[i], b'1'..=b'3') {
        i += 1;
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
    }

    let mut digit_start = None;
    while i < bytes.len() {
        if bytes[i].is_ascii_digit() {
            digit_start = Some(i);
            break;
        }
        i += 1;
    }

    let Some(split_at) = digit_start else {
        return Err(ScriptureParseError::MissingChapter {
            raw: input.to_owned(),
        });
    };

    let book_part = input[..split_at].trim();
    let num_part = input[split_at..].trim();
    if book_part.is_empty() {
        return Err(ScriptureParseError::UnknownBook {
            book: input.to_owned(),
        });
    }
    Ok((book_part, num_part))
}

fn split_chapter_verse(nums: &str) -> Option<(&str, &str)> {
    if let Some(pos) = nums.find([':', '.']) {
        return Some((nums[..pos].trim(), nums[pos + 1..].trim()));
    }
    let mut parts = nums.split_whitespace();
    let first = parts.next()?;
    let second = parts.next()?;
    if parts.next().is_none() {
        Some((first, second))
    } else {
        None
    }
}

fn parse_verse_range(raw: &str) -> Result<(u16, Option<u16>), ScriptureParseError> {
    let cleaned = raw.trim();
    if let Some(pos) = cleaned.find(['-', '–', '—']) {
        let sep_len = cleaned[pos..].chars().next().map_or(1, char::len_utf8);
        let start = parse_u16_token(cleaned[..pos].trim())?;
        let end = parse_u16_token(cleaned[pos + sep_len..].trim())?;
        if end < start {
            return Err(ScriptureParseError::InvertedRange { start, end });
        }
        Ok((start, Some(end)))
    } else {
        let single = parse_u16_token(cleaned)?;
        Ok((single, Some(single)))
    }
}

fn parse_u16_token(token: &str) -> Result<u16, ScriptureParseError> {
    let parsed = token
        .trim()
        .parse::<u16>()
        .map_err(|_| ScriptureParseError::InvalidNumber {
            token: token.to_owned(),
        })?;
    if parsed == 0 {
        return Err(ScriptureParseError::InvalidNumber {
            token: token.to_owned(),
        });
    }
    Ok(parsed)
}

/// Splits long scripture text cleanly at sentence boundaries first, then word boundaries.
fn split_long_text(text: &str, max_chars: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut rest = text.trim();

    while rest.chars().count() > max_chars {
        let mut cutoff_byte = 0;
        let mut last_sentence_byte = None;
        let mut last_space_byte = None;

        for (char_idx, (byte_idx, ch)) in rest.char_indices().enumerate() {
            if char_idx >= max_chars {
                break;
            }
            let next_byte = byte_idx + ch.len_utf8();
            cutoff_byte = next_byte;
            if ch.is_whitespace() {
                last_space_byte = Some(byte_idx);
            } else if matches!(ch, '.' | '!' | '?' | ';') && char_idx >= max_chars / 2 {
                last_sentence_byte = Some(next_byte);
            }
        }

        let split_at = last_sentence_byte
            .or(last_space_byte)
            .unwrap_or(cutoff_byte)
            .max(1);
        let head = rest[..split_at].trim();
        if !head.is_empty() {
            chunks.push(head.to_owned());
        }
        rest = rest[split_at..].trim_start();
    }

    if !rest.is_empty() {
        chunks.push(rest.to_owned());
    }
    chunks
}
