use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};

use crate::bible_books::{book_by_number, book_by_osis, book_by_usfx};
use crate::error::ImportError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedVerse {
    pub book_number: u8,
    pub chapter: u16,
    pub verse: u16,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedBible {
    pub code: String,
    pub name: String,
    pub language: String,
    pub copyright: String,
    pub verses: Vec<ParsedVerse>,
}

/// Auto-detects OSIS XML, USFX XML, or Zefania XML and parses verses into a [`ParsedBible`].
pub fn import_bible_xml(code: &str, name: &str, xml: &str) -> Result<ParsedBible, ImportError> {
    let trimmed = xml.trim();
    if trimmed.is_empty() {
        return Err(ImportError::EmptyInput);
    }

    let head = &trimmed[..trimmed.len().min(600)].to_ascii_lowercase();
    if head.contains("<xmlbible") || head.contains("<biblebook") {
        import_zefania_xml(code, name, trimmed)
    } else if head.contains("<osis") {
        import_osis_xml(code, name, trimmed)
    } else if head.contains("<usfx") {
        import_usfx_xml(code, name, trimmed)
    } else {
        Err(ImportError::UnsupportedFormat {
            detail: "expected OSIS, USFX, or Zefania XML root element".to_owned(),
        })
    }
}

/// Parses Zefania XML (`<XMLBIBLE><BIBLEBOOK bnumber="1"><CHAPTER cnumber="1"><VERS vnumber="1">`).
pub fn import_zefania_xml(code: &str, name: &str, xml: &str) -> Result<ParsedBible, ImportError> {
    let mut reader = Reader::from_str(xml);
    let mut verses = Vec::new();
    let mut cur_book: Option<u8> = None;
    let mut cur_chapter: Option<u16> = None;
    let mut cur_verse: Option<u16> = None;
    let mut text_buf = String::new();
    let mut detected_name = name.trim().to_owned();

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let tag = local_upper(e.name().as_ref());
                match tag.as_str() {
                    "XMLBIBLE" if detected_name.is_empty() => {
                        if let Some(n) = attr(&e, "biblename") {
                            detected_name = n;
                        }
                    }
                    "BIBLEBOOK" => {
                        cur_book = attr(&e, "bnumber").and_then(|s| s.parse::<u8>().ok());
                    }
                    "CHAPTER" => {
                        cur_chapter = attr(&e, "cnumber").and_then(|s| s.parse::<u16>().ok());
                    }
                    "VERS" => {
                        cur_verse = attr(&e, "vnumber").and_then(|s| s.parse::<u16>().ok());
                        text_buf.clear();
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(e)) if cur_verse.is_some() => {
                append_decoded(&e, &mut text_buf)?;
            }
            Ok(Event::End(e)) => {
                if local_upper(e.name().as_ref()) == "VERS" {
                    if let (Some(b), Some(c), Some(v)) = (cur_book, cur_chapter, cur_verse.take()) {
                        push_cleaned_verse(&mut verses, b, c, v, &text_buf);
                    }
                    text_buf.clear();
                }
            }
            Ok(Event::Eof) => break,
            Err(err) => {
                return Err(ImportError::MalformedXml {
                    detail: err.to_string(),
                });
            }
            _ => {}
        }
    }

    finish_parsed_bible(code, detected_name, verses)
}

/// Parses OSIS XML (both container `<verse osisID="Gen.1.1">...</verse>` and milestone `sID`/`eID`).
pub fn import_osis_xml(code: &str, name: &str, xml: &str) -> Result<ParsedBible, ImportError> {
    let mut reader = Reader::from_str(xml);
    let mut verses = Vec::new();
    let mut active_ref: Option<(u8, u16, u16)> = None;
    let mut text_buf = String::new();
    let mut skip_note_depth: usize = 0;

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let tag = local_lower(e.name().as_ref());
                if tag == "note" || tag == "title" {
                    skip_note_depth += 1;
                } else if tag == "verse"
                    && let Some(id) = attr(&e, "osisID").or_else(|| attr(&e, "sID"))
                {
                    active_ref = parse_osis_verse_id(&id);
                    text_buf.clear();
                }
            }
            Ok(Event::Empty(e)) => {
                if local_lower(e.name().as_ref()) == "verse" {
                    if let Some(id) = attr(&e, "sID") {
                        if let Some((b, c, v)) = active_ref.take() {
                            push_cleaned_verse(&mut verses, b, c, v, &text_buf);
                        }
                        active_ref = parse_osis_verse_id(&id);
                        text_buf.clear();
                    } else if attr(&e, "eID").is_some() {
                        if let Some((b, c, v)) = active_ref.take() {
                            push_cleaned_verse(&mut verses, b, c, v, &text_buf);
                        }
                        text_buf.clear();
                    }
                }
            }
            Ok(Event::Text(e)) if active_ref.is_some() && skip_note_depth == 0 => {
                append_decoded(&e, &mut text_buf)?;
            }
            Ok(Event::End(e)) => {
                let tag = local_lower(e.name().as_ref());
                if tag == "note" || tag == "title" {
                    skip_note_depth = skip_note_depth.saturating_sub(1);
                } else if tag == "verse" {
                    if let Some((b, c, v)) = active_ref.take() {
                        push_cleaned_verse(&mut verses, b, c, v, &text_buf);
                    }
                    text_buf.clear();
                }
            }
            Ok(Event::Eof) => break,
            Err(err) => {
                return Err(ImportError::MalformedXml {
                    detail: err.to_string(),
                });
            }
            _ => {}
        }
    }

    finish_parsed_bible(code, name.to_owned(), verses)
}

/// Parses USFX XML (`<book id="GEN">`, `<c id="1"/>`, `<v id="1"/>`, skipping `<f>` footnotes).
pub fn import_usfx_xml(code: &str, name: &str, xml: &str) -> Result<ParsedBible, ImportError> {
    let mut reader = Reader::from_str(xml);
    let mut verses = Vec::new();
    let mut cur_book: Option<u8> = None;
    let mut cur_chapter: Option<u16> = None;
    let mut cur_verse: Option<u16> = None;
    let mut text_buf = String::new();
    let mut skip_depth: usize = 0;

    let flush = |verses: &mut Vec<ParsedVerse>,
                 cur_book: Option<u8>,
                 cur_chapter: Option<u16>,
                 cur_verse: &mut Option<u16>,
                 buf: &mut String| {
        if let (Some(b), Some(c), Some(v)) = (cur_book, cur_chapter, cur_verse.take()) {
            push_cleaned_verse(verses, b, c, v, buf);
        }
        buf.clear();
    };

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let tag = local_lower(e.name().as_ref());
                match tag.as_str() {
                    "book" => {
                        flush(
                            &mut verses,
                            cur_book,
                            cur_chapter,
                            &mut cur_verse,
                            &mut text_buf,
                        );
                        cur_book = attr(&e, "id")
                            .and_then(|id| book_by_usfx(&id))
                            .map(|m| m.number);
                        cur_chapter = None;
                    }
                    "f" | "x" | "h" | "s" | "mt" => skip_depth += 1,
                    "v" => {
                        flush(
                            &mut verses,
                            cur_book,
                            cur_chapter,
                            &mut cur_verse,
                            &mut text_buf,
                        );
                        cur_verse = attr(&e, "id").and_then(|s| s.parse::<u16>().ok());
                    }
                    _ => {}
                }
            }
            Ok(Event::Empty(e)) => {
                let tag = local_lower(e.name().as_ref());
                match tag.as_str() {
                    "c" => {
                        flush(
                            &mut verses,
                            cur_book,
                            cur_chapter,
                            &mut cur_verse,
                            &mut text_buf,
                        );
                        cur_chapter = attr(&e, "id").and_then(|s| s.parse::<u16>().ok());
                    }
                    "v" => {
                        flush(
                            &mut verses,
                            cur_book,
                            cur_chapter,
                            &mut cur_verse,
                            &mut text_buf,
                        );
                        cur_verse = attr(&e, "id").and_then(|s| s.parse::<u16>().ok());
                    }
                    "ve" => {
                        flush(
                            &mut verses,
                            cur_book,
                            cur_chapter,
                            &mut cur_verse,
                            &mut text_buf,
                        );
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(e)) if cur_verse.is_some() && skip_depth == 0 => {
                append_decoded(&e, &mut text_buf)?;
            }
            Ok(Event::End(e)) => {
                let tag = local_lower(e.name().as_ref());
                if matches!(tag.as_str(), "f" | "x" | "h" | "s" | "mt") {
                    skip_depth = skip_depth.saturating_sub(1);
                } else if tag == "book" || tag == "c" {
                    flush(
                        &mut verses,
                        cur_book,
                        cur_chapter,
                        &mut cur_verse,
                        &mut text_buf,
                    );
                }
            }
            Ok(Event::Eof) => break,
            Err(err) => {
                return Err(ImportError::MalformedXml {
                    detail: err.to_string(),
                });
            }
            _ => {}
        }
    }

    flush(
        &mut verses,
        cur_book,
        cur_chapter,
        &mut cur_verse,
        &mut text_buf,
    );
    finish_parsed_bible(code, name.to_owned(), verses)
}

fn finish_parsed_bible(
    code: &str,
    name: String,
    verses: Vec<ParsedVerse>,
) -> Result<ParsedBible, ImportError> {
    if verses.is_empty() {
        return Err(ImportError::NoContent {
            kind: "Bible verses",
        });
    }
    let clean_code = code.trim().to_ascii_uppercase();
    let final_code = if clean_code.is_empty() {
        "CUSTOM".to_owned()
    } else {
        clean_code
    };
    let final_name = if name.trim().is_empty() {
        final_code.clone()
    } else {
        name.trim().to_owned()
    };
    Ok(ParsedBible {
        code: final_code,
        name: final_name,
        language: "en".to_owned(),
        copyright: String::new(),
        verses,
    })
}

fn parse_osis_verse_id(raw: &str) -> Option<(u8, u16, u16)> {
    let first_ref = raw.split_whitespace().next()?;
    let mut parts = first_ref.split('.');
    let book_str = parts.next()?;
    let chapter = parts.next()?.parse::<u16>().ok()?;
    let verse = parts.next()?.parse::<u16>().ok()?;
    let book = book_by_osis(book_str)?;
    Some((book.number, chapter, verse))
}

fn push_cleaned_verse(
    verses: &mut Vec<ParsedVerse>,
    book_number: u8,
    chapter: u16,
    verse: u16,
    raw: &str,
) {
    if book_by_number(book_number).is_none() || chapter == 0 || verse == 0 {
        return;
    }
    let cleaned: String = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    if !cleaned.is_empty() {
        verses.push(ParsedVerse {
            book_number,
            chapter,
            verse,
            text: cleaned,
        });
    }
}

fn append_decoded(
    e: &quick_xml::events::BytesText<'_>,
    out: &mut String,
) -> Result<(), ImportError> {
    let text = e.xml_content(quick_xml::XmlVersion::Implicit1_0);
    if !out.is_empty() && !out.ends_with(' ') && !text.starts_with(' ') {
        out.push(' ');
    }
    out.push_str(&text);
    Ok(())
}

fn local_lower(s: &str) -> String {
    s.rsplit(':').next().unwrap_or(s).to_ascii_lowercase()
}

fn local_upper(s: &str) -> String {
    s.rsplit(':').next().unwrap_or(s).to_ascii_uppercase()
}

fn attr(e: &BytesStart<'_>, name: &str) -> Option<String> {
    for a in e.attributes().flatten() {
        if a.key.as_ref().eq_ignore_ascii_case(name) {
            return a
                .normalized_value(quick_xml::XmlVersion::Implicit1_0)
                .ok()
                .map(|v| v.into_owned());
        }
    }
    None
}
