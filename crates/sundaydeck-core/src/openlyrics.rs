use quick_xml::Reader;
use quick_xml::events::Event;

use crate::error::ImportError;
use crate::ids::{ArrangementId, SectionId, SongId};
use crate::song::{Arrangement, ReflowMode, SectionKind, Song, SongSection};

/// Imports a standard OpenLyrics XML (`.xml`) song, honoring `<verseOrder>` non-destructively.
pub fn import_openlyrics_xml(xml: &str) -> Result<Song, ImportError> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(false);

    let mut title = String::new();
    let mut alternate_title = String::new();
    let mut authors: Vec<String> = Vec::new();
    let mut copyright = String::new();
    let mut ccli_number = String::new();
    let mut verse_order_raw = String::new();

    let mut sections_by_code: Vec<(String, SongSection)> = Vec::new();
    let mut in_tag: Vec<String> = Vec::new();
    let mut current_verse_code: Option<String> = None;
    let mut current_lines = String::new();
    let mut text_buf = String::new();

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let tag = local_name(e.name().as_ref());
                if tag == "verse" {
                    current_verse_code = attr_value(&e, b"name");
                    current_lines.clear();
                } else if tag == "lines" && !current_lines.is_empty() {
                    current_lines.push_str("\n\n");
                }
                text_buf.clear();
                in_tag.push(tag);
            }
            Ok(Event::Empty(e)) => {
                if local_name(e.name().as_ref()) == "br" && current_verse_code.is_some() {
                    current_lines.push('\n');
                }
            }
            Ok(Event::Text(e)) => {
                let decoded = e.decode().map_err(|err| ImportError::MalformedXml {
                    detail: err.to_string(),
                })?;
                if current_verse_code.is_some() && in_tag.iter().any(|t| t == "lines") {
                    current_lines.push_str(&decoded);
                } else {
                    text_buf.push_str(&decoded);
                }
            }
            Ok(Event::End(e)) => {
                let tag = local_name(e.name().as_ref());
                let val = text_buf.trim().to_owned();
                match tag.as_str() {
                    "title" if !val.is_empty() => {
                        if title.is_empty() {
                            title = val;
                        } else if alternate_title.is_empty() {
                            alternate_title = val;
                        }
                    }
                    "author" if !val.is_empty() => authors.push(val),
                    "copyright" if !val.is_empty() => copyright = val,
                    "ccliNo" if !val.is_empty() => ccli_number = val,
                    "verseOrder" if !val.is_empty() => verse_order_raw = val,
                    "verse" => {
                        if let Some(code) = current_verse_code.take() {
                            let body = normalize_multiline_body(&current_lines);
                            if !body.is_empty() {
                                let (kind, label) = parse_openlyrics_code(&code);
                                let ordinal = sections_by_code.len() as u32;
                                sections_by_code.push((
                                    code.to_ascii_lowercase(),
                                    SongSection {
                                        id: SectionId::new_v7(),
                                        kind,
                                        label,
                                        ordinal,
                                        body,
                                        secondary_body: None,
                                        stage_chords: None,
                                    },
                                ));
                            }
                        }
                    }
                    _ => {}
                }
                in_tag.pop();
                text_buf.clear();
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

    if sections_by_code.is_empty() {
        return Err(ImportError::NoContent {
            kind: "song lyrics",
        });
    }
    if title.is_empty() {
        title = "Untitled Song".to_owned();
    }

    let arrangement_ids = resolve_verse_order(&verse_order_raw, &sections_by_code);
    let arr_id = ArrangementId::new_v7();
    let sections: Vec<SongSection> = sections_by_code.into_iter().map(|(_, s)| s).collect();

    Ok(Song {
        id: SongId::new_v7(),
        title,
        alternate_title,
        author: authors.join(", "),
        copyright,
        ccli_number,
        default_theme_id: None,
        default_arrangement_id: Some(arr_id),
        default_reflow_mode: ReflowMode::ByStanza,
        primary_lang: "en".to_owned(),
        secondary_lang: None,
        hymnal_entries: Vec::new(),
        traditions: Vec::new(),
        sections,
        arrangements: vec![Arrangement {
            id: arr_id,
            name: "Default".to_owned(),
            section_ids: arrangement_ids,
        }],
    })
}

fn parse_openlyrics_code(code: &str) -> (SectionKind, String) {
    let lower = code.trim().to_ascii_lowercase();
    let num: String = lower.chars().skip(1).collect();
    let suffix = if num.is_empty() {
        String::new()
    } else {
        format!(" {num}")
    };
    match lower.chars().next() {
        Some('v') => (SectionKind::Verse, format!("Verse{suffix}")),
        Some('c') => (SectionKind::Chorus, format!("Chorus{suffix}")),
        Some('p') => (SectionKind::PreChorus, format!("Pre-Chorus{suffix}")),
        Some('b') => (SectionKind::Bridge, format!("Bridge{suffix}")),
        Some('e') => (SectionKind::Ending, format!("Ending{suffix}")),
        Some('t') => (SectionKind::Tag, format!("Tag{suffix}")),
        Some('i') => (SectionKind::Intro, format!("Intro{suffix}")),
        _ => (SectionKind::Other, code.to_owned()),
    }
}

fn resolve_verse_order(raw_order: &str, sections: &[(String, SongSection)]) -> Vec<SectionId> {
    let mut ids = Vec::new();
    for token in raw_order.split_whitespace() {
        let key = token.to_ascii_lowercase();
        if let Some((_, sec)) = sections.iter().find(|(code, _)| *code == key) {
            ids.push(sec.id);
        }
    }
    if ids.is_empty() {
        sections.iter().map(|(_, s)| s.id).collect()
    } else {
        ids
    }
}

fn normalize_multiline_body(raw: &str) -> String {
    raw.lines()
        .map(str::trim)
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_owned()
}

fn local_name(bytes: &[u8]) -> String {
    let full = std::str::from_utf8(bytes).unwrap_or("");
    full.rsplit(':').next().unwrap_or(full).to_owned()
}

fn attr_value(e: &quick_xml::events::BytesStart<'_>, key: &[u8]) -> Option<String> {
    for attr in e.attributes().flatten() {
        if attr.key.as_ref() == key {
            return attr
                .normalized_value(quick_xml::XmlVersion::Implicit1_0)
                .ok()
                .map(|v| v.into_owned());
        }
    }
    None
}
