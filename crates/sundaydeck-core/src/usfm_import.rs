use crate::bible_books::book_by_usfx;
use crate::bible_import::{ParsedBible, ParsedVerse, import_bible_xml};
use crate::error::ImportError;

/// Trait-based Bible importer pipeline enabling pluggable support for USFM, USFX, OSIS,
/// Zefania XML, SQLite, and `.sdeck-pack` Bible sources.
pub trait BibleImporter: Send + Sync {
    fn format_name(&self) -> &'static str;
    fn can_handle(&self, filename: &str, sample: &str) -> bool;
    fn import(&self, code: &str, name: &str, raw: &str) -> Result<ParsedBible, ImportError>;
}

pub struct UsfmBibleImporter;
pub struct XmlBibleImporter;

impl BibleImporter for UsfmBibleImporter {
    fn format_name(&self) -> &'static str {
        "USFM (eBible.org / SIL)"
    }

    fn can_handle(&self, filename: &str, sample: &str) -> bool {
        let lower = filename.to_ascii_lowercase();
        lower.ends_with(".usfm")
            || lower.ends_with(".sfm")
            || sample.contains("\\id ")
            || (sample.contains("\\c ") && sample.contains("\\v "))
    }

    fn import(&self, code: &str, name: &str, raw: &str) -> Result<ParsedBible, ImportError> {
        import_usfm_text(code, name, "und", raw)
    }
}

impl BibleImporter for XmlBibleImporter {
    fn format_name(&self) -> &'static str {
        "OSIS / USFX / Zefania XML"
    }

    fn can_handle(&self, filename: &str, sample: &str) -> bool {
        let lower = filename.to_ascii_lowercase();
        lower.ends_with(".xml")
            || lower.ends_with(".osis")
            || lower.ends_with(".usfx")
            || sample.trim_start().starts_with('<')
    }

    fn import(&self, code: &str, name: &str, raw: &str) -> Result<ParsedBible, ImportError> {
        import_bible_xml(code, name, raw)
    }
}

/// Auto-detects USFM, USFX, OSIS, or Zefania text formats and parses into a [`ParsedBible`].
pub fn detect_and_import_bible(
    code: &str,
    name: &str,
    filename: &str,
    raw: &str,
) -> Result<ParsedBible, ImportError> {
    let sample = &raw[..raw.len().min(600)];
    let importers: [&dyn BibleImporter; 2] = [&UsfmBibleImporter, &XmlBibleImporter];
    for importer in importers {
        if importer.can_handle(filename, sample) {
            return importer.import(code, name, raw);
        }
    }
    Err(ImportError::UnsupportedFormat {
        detail: format!("unrecognized Bible format for '{filename}'"),
    })
}

/// Parses SIL / eBible.org **USFM** (`.usfm` / `.sfm`) text (single book or concatenated multi-book)
/// supporting all 73 Catholic + 66 Protestant canonical USFM 3-letter book IDs (`GEN`..`REV`,
/// `TOB`, `JDT`, `WIS`, `SIR`, `BAR`, `1MA`, `2MA`, `ESG`, `DAG`).
pub fn import_usfm_text(
    code: &str,
    name: &str,
    language: &str,
    usfm: &str,
) -> Result<ParsedBible, ImportError> {
    if usfm.trim().is_empty() {
        return Err(ImportError::EmptyInput);
    }

    let mut verses: Vec<ParsedVerse> = Vec::new();
    let mut cur_book: Option<u8> = None;
    let mut cur_chapter: Option<u16> = None;
    let mut cur_verse: Option<u16> = None;
    let mut verse_buf = String::new();

    let flush = |verses: &mut Vec<ParsedVerse>,
                 cur_book: Option<u8>,
                 cur_chapter: Option<u16>,
                 cur_verse: &mut Option<u16>,
                 buf: &mut String| {
        if let (Some(b), Some(c), Some(v)) = (cur_book, cur_chapter, *cur_verse) {
            let cleaned = clean_usfm_inline(buf);
            if !cleaned.is_empty() {
                verses.push(ParsedVerse {
                    book_number: b,
                    chapter: c,
                    verse: v,
                    text: cleaned,
                });
            }
        }
        *cur_verse = None;
        buf.clear();
    };

    for raw_line in usfm.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(rest) = line.strip_prefix("\\id ") {
            flush(&mut verses, cur_book, cur_chapter, &mut cur_verse, &mut verse_buf);
            let usfm_id = rest.split_whitespace().next().unwrap_or("");
            cur_book = book_by_usfx(usfm_id).map(|m| m.number);
            cur_chapter = None;
            continue;
        }
        if let Some(rest) = line.strip_prefix("\\c ") {
            flush(&mut verses, cur_book, cur_chapter, &mut cur_verse, &mut verse_buf);
            cur_chapter = rest
                .split_whitespace()
                .next()
                .and_then(|s| s.parse::<u16>().ok());
            continue;
        }
        if let Some(rest) = line.strip_prefix("\\v ") {
            flush(&mut verses, cur_book, cur_chapter, &mut cur_verse, &mut verse_buf);
            let mut parts = rest.splitn(2, char::is_whitespace);
            let v_tok = parts.next().unwrap_or("");
            let start_num = v_tok
                .split(['-', ','])
                .next()
                .and_then(|s| s.parse::<u16>().ok());
            cur_verse = start_num;
            if let Some(body) = parts.next() {
                verse_buf.push_str(body.trim());
            }
            continue;
        }
        // Continuation poetry / paragraph lines (`\p`, `\q1`, `\q2`, `\m`, `\nb`) append to active verse.
        if cur_verse.is_some() {
            if let Some(stripped) = strip_usfm_line_marker(line) {
                if !stripped.is_empty() {
                    if !verse_buf.is_empty() {
                        verse_buf.push(' ');
                    }
                    verse_buf.push_str(stripped);
                }
            } else if !line.starts_with('\\') {
                if !verse_buf.is_empty() {
                    verse_buf.push(' ');
                }
                verse_buf.push_str(line);
            }
        }
    }

    flush(&mut verses, cur_book, cur_chapter, &mut cur_verse, &mut verse_buf);

    if verses.is_empty() {
        return Err(ImportError::NoContent { kind: "USFM verses" });
    }

    Ok(ParsedBible {
        code: code.trim().to_ascii_uppercase(),
        name: if name.trim().is_empty() {
            code.trim().to_owned()
        } else {
            name.trim().to_owned()
        },
        language: language.to_owned(),
        copyright: String::new(),
        verses,
    })
}

fn strip_usfm_line_marker(line: &str) -> Option<&str> {
    for prefix in ["\\p ", "\\q ", "\\q1 ", "\\q2 ", "\\q3 ", "\\m ", "\\pi ", "\\li "] {
        if let Some(rest) = line.strip_prefix(prefix) {
            return Some(rest.trim());
        }
    }
    None
}

/// Strips USFM footnotes (`\f ... \f*`), cross-refs (`\x ... \x*`), word attributes (`|strong=...`),
/// and inline formatting tags (`\nd`, `\wj`, `\add`, `\qs`) while preserving readable Scripture text.
fn clean_usfm_inline(raw: &str) -> String {
    let mut no_notes = strip_paired_tag(raw, "\\f", "\\f*");
    no_notes = strip_paired_tag(&no_notes, "\\x", "\\x*");

    let mut out = String::with_capacity(no_notes.len());
    let chars: Vec<char> = no_notes.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '\\' {
            i += 1;
            while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '+' || chars[i] == '*') {
                i += 1;
            }
            if i < chars.len() && chars[i] == ' ' {
                i += 1;
            }
            continue;
        }
        if chars[i] == '|' {
            // Skip Strong's/lemma attributes up to the closing `\w*` tag.
            while i < chars.len() && chars[i] != '\\' {
                i += 1;
            }
            continue;
        }
        out.push(chars[i]);
        i += 1;
    }

    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn strip_paired_tag(input: &str, open: &str, close: &str) -> String {
    let mut res = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(start) = rest.find(open) {
        res.push_str(&rest[..start]);
        let after = &rest[start + open.len()..];
        if let Some(end) = after.find(close) {
            rest = &after[end + close.len()..];
        } else {
            rest = "";
            break;
        }
    }
    res.push_str(rest);
    res
}
