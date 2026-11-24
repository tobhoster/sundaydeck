use serde::Deserialize;

use crate::error::ImportError;
use crate::global_content::HymnalEntry;
use crate::ids::SectionId;
use crate::legacy_formats::{
    finalize_imported_song, import_freeshow_or_videopsalm_json, import_propresenter6_xml,
};
use crate::openlyrics::import_openlyrics_xml;
use crate::song::{SectionKind, Song, SongSection};
use crate::song_import::{detect_and_import_song, import_songselect_usr};

/// Pluggable trait for migrating church song libraries & denominational hymnbooks.
pub trait SongImporter: Send + Sync {
    fn format_name(&self) -> &'static str;
    fn import_text(&self, filename: &str, content: &str) -> Result<Vec<Song>, ImportError>;
}

pub struct UniversalSongImporter;

impl SongImporter for UniversalSongImporter {
    fn format_name(&self) -> &'static str {
        "Universal (OpenLyrics / SongSelect / ChordPro / ProPresenter / FreeShow / Numbered Hymnal)"
    }

    fn import_text(&self, filename: &str, content: &str) -> Result<Vec<Song>, ImportError> {
        let lower = filename.to_ascii_lowercase();
        let trimmed = content.trim();
        if trimmed.is_empty() {
            return Err(ImportError::EmptyInput);
        }
        if lower.ends_with(".cho")
            || lower.ends_with(".chordpro")
            || trimmed.contains("{title:")
            || trimmed.contains("{t:")
        {
            return Ok(vec![import_chordpro_song(trimmed)?]);
        }
        if lower.ends_with(".pro6") || trimmed.contains("<RVPresentationDocument") {
            return Ok(vec![import_propresenter6_xml(trimmed)?]);
        }
        if lower.ends_with(".show")
            || lower.ends_with(".vpc")
            || (lower.ends_with(".json") && trimmed.starts_with('{'))
        {
            if let Ok(songs) = import_numbered_hymnal_json(trimmed) {
                return Ok(songs);
            }
            return import_freeshow_or_videopsalm_json(trimmed);
        }
        if lower.ends_with(".json") && trimmed.starts_with('[') {
            return import_numbered_hymnal_json(trimmed);
        }
        if lower.ends_with(".usr") {
            return Ok(vec![import_songselect_usr(trimmed)?]);
        }
        if lower.ends_with(".xml") && trimmed.to_ascii_lowercase().contains("<song") {
            return Ok(vec![import_openlyrics_xml(trimmed)?]);
        }
        if looks_like_bulk_numbered_hymnal(trimmed) {
            return import_numbered_hymnal_text(trimmed);
        }
        Ok(vec![detect_and_import_song(filename, trimmed)?])
    }
}

/// Imports ChordPro (`.cho`/`.pro`), stripping inline `[G]`/`[Am7]` chords for clean sanctuary
/// projection while preserving the chord-annotated sheet in `section.stage_chords`.
pub fn import_chordpro_song(raw: &str) -> Result<Song, ImportError> {
    if raw.trim().is_empty() {
        return Err(ImportError::EmptyInput);
    }
    let mut title = String::new();
    let mut author = String::new();
    let mut ccli = String::new();
    let mut sections: Vec<SongSection> = Vec::new();
    let mut cur_kind = SectionKind::Verse;
    let mut cur_label = "Verse 1".to_owned();
    let mut verse_n = 1u32;
    let mut plain_lines: Vec<String> = Vec::new();
    let mut chord_lines: Vec<String> = Vec::new();

    let flush = |sections: &mut Vec<SongSection>,
                 kind: SectionKind,
                 label: &str,
                 plain: &mut Vec<String>,
                 chords: &mut Vec<String>| {
        if !plain.is_empty() {
            sections.push(SongSection {
                id: SectionId::new_v7(),
                kind,
                label: label.to_owned(),
                ordinal: sections.len() as u32,
                body: plain.join("\n"),
                secondary_body: None,
                stage_chords: Some(chords.join("\n")),
            });
            plain.clear();
            chords.clear();
        }
    };

    for line in raw.lines() {
        let t = line.trim();
        if t.is_empty() {
            flush(
                &mut sections,
                cur_kind,
                &cur_label,
                &mut plain_lines,
                &mut chord_lines,
            );
            continue;
        }
        if t.starts_with('{') && t.ends_with('}') {
            let inner = &t[1..t.len() - 1];
            let lower = inner.to_ascii_lowercase();
            if let Some(val) = inner
                .strip_prefix("title:")
                .or_else(|| inner.strip_prefix("t:"))
            {
                title = val.trim().to_owned();
            } else if let Some(val) = inner
                .strip_prefix("artist:")
                .or_else(|| inner.strip_prefix("subtitle:"))
            {
                author = val.trim().to_owned();
            } else if let Some(val) = inner.strip_prefix("ccli:") {
                ccli = val.trim().to_owned();
            } else if lower.starts_with("soc") || lower.starts_with("start_of_chorus") {
                flush(
                    &mut sections,
                    cur_kind,
                    &cur_label,
                    &mut plain_lines,
                    &mut chord_lines,
                );
                cur_kind = SectionKind::Chorus;
                cur_label = "Chorus".to_owned();
            } else if lower.starts_with("eoc") || lower.starts_with("end_of_chorus") {
                flush(
                    &mut sections,
                    cur_kind,
                    &cur_label,
                    &mut plain_lines,
                    &mut chord_lines,
                );
                verse_n += 1;
                cur_kind = SectionKind::Verse;
                cur_label = format!("Verse {verse_n}");
            }
            continue;
        }
        chord_lines.push(t.to_owned());
        let stripped = strip_bracket_chords(t);
        if !stripped.is_empty() {
            plain_lines.push(stripped);
        }
    }
    flush(
        &mut sections,
        cur_kind,
        &cur_label,
        &mut plain_lines,
        &mut chord_lines,
    );
    if sections.is_empty() {
        return Err(ImportError::NoContent {
            kind: "ChordPro lyrics",
        });
    }
    Ok(finalize_imported_song(
        if title.is_empty() {
            "Untitled Song".to_owned()
        } else {
            title
        },
        author,
        ccli,
        "en",
        None,
        Vec::new(),
        Vec::new(),
        sections,
    ))
}

/// Parses bulk numbered hymnbook sheets (`BH 330 — Title`, `UMH 378`, `HC 15`, `HGT 42`, `Himno 42`)
/// supporting optional paired bilingual lines (`Primary line || English translation`).
pub fn import_numbered_hymnal_text(raw: &str) -> Result<Vec<Song>, ImportError> {
    let mut songs = Vec::new();
    let mut cur_header: Option<(HymnalEntry, String)> = None;
    let mut cur_body = String::new();

    for line in raw.lines() {
        if let Some(parsed_hdr) = parse_hymn_header_line(line) {
            if let Some((entry, title)) = cur_header.take()
                && let Some(song) = build_hymnal_song(entry, title, &cur_body)
            {
                songs.push(song);
            }
            cur_header = Some(parsed_hdr);
            cur_body.clear();
        } else {
            cur_body.push_str(line);
            cur_body.push('\n');
        }
    }
    if let Some((entry, title)) = cur_header.take()
        && let Some(song) = build_hymnal_song(entry, title, &cur_body)
    {
        songs.push(song);
    }

    if songs.is_empty() {
        return Err(ImportError::NoContent {
            kind: "numbered hymns",
        });
    }
    Ok(songs)
}

#[derive(Deserialize)]
struct JsonHymnEntry {
    hymnal_code: String,
    #[serde(default)]
    hymnal_name: String,
    hymn_number: u32,
    title: String,
    #[serde(default)]
    author: String,
    #[serde(default = "default_lang")]
    primary_lang: String,
    #[serde(default)]
    secondary_lang: Option<String>,
    #[serde(default)]
    traditions: Vec<String>,
    stanzas: Vec<JsonHymnStanza>,
}

#[derive(Deserialize)]
struct JsonHymnStanza {
    #[serde(default = "default_stanza_label")]
    label: String,
    body: String,
    #[serde(default)]
    secondary_body: Option<String>,
}

fn default_lang() -> String {
    "en".to_owned()
}
fn default_stanza_label() -> String {
    "Verse".to_owned()
}

/// Parses structured JSON hymnbook arrays (`[{ "hymnal_code": "BH", "hymn_number": 330, ... }]`).
pub fn import_numbered_hymnal_json(raw: &str) -> Result<Vec<Song>, ImportError> {
    let entries: Vec<JsonHymnEntry> =
        serde_json::from_str(raw).map_err(|e| ImportError::UnsupportedFormat {
            detail: e.to_string(),
        })?;
    let mut out = Vec::new();
    for item in entries {
        let sections: Vec<SongSection> = item
            .stanzas
            .into_iter()
            .enumerate()
            .map(|(i, st)| SongSection {
                id: SectionId::new_v7(),
                kind: if st.label.to_ascii_lowercase().contains("chorus") {
                    SectionKind::Chorus
                } else {
                    SectionKind::Verse
                },
                label: st.label,
                ordinal: i as u32,
                body: st.body,
                secondary_body: st.secondary_body,
                stage_chords: None,
            })
            .collect();
        if sections.is_empty() {
            continue;
        }
        let h_name = if item.hymnal_name.is_empty() {
            item.hymnal_code.clone()
        } else {
            item.hymnal_name
        };
        out.push(finalize_imported_song(
            item.title,
            item.author,
            String::new(),
            &item.primary_lang,
            item.secondary_lang,
            vec![HymnalEntry {
                hymnal_code: item.hymnal_code.to_ascii_uppercase(),
                hymnal_name: h_name,
                hymn_number: item.hymn_number,
            }],
            item.traditions,
            sections,
        ));
    }
    if out.is_empty() {
        return Err(ImportError::NoContent { kind: "JSON hymns" });
    }
    Ok(out)
}

fn looks_like_bulk_numbered_hymnal(raw: &str) -> bool {
    raw.lines()
        .filter(|l| parse_hymn_header_line(l).is_some())
        .count()
        >= 1
}

fn parse_hymn_header_line(line: &str) -> Option<(HymnalEntry, String)> {
    let t = line.trim();
    let (left, title) = t
        .split_once(" — ")
        .or_else(|| t.split_once(" - "))
        .or_else(|| t.split_once(": "))?;
    let tokens: Vec<&str> = left.split_whitespace().collect();
    if tokens.len() != 2 {
        return None;
    }
    let code = tokens[0].trim_matches(['[', ']', '#']).to_ascii_uppercase();
    if code.len() < 2 || !code.chars().all(|c| c.is_ascii_alphabetic() || c == '&') {
        return None;
    }
    let number = tokens[1].trim_start_matches('#').parse::<u32>().ok()?;
    Some((
        HymnalEntry {
            hymnal_name: code.clone(),
            hymnal_code: code,
            hymn_number: number,
        },
        title.trim().to_owned(),
    ))
}

fn build_hymnal_song(entry: HymnalEntry, title: String, body: &str) -> Option<Song> {
    let mut sections = Vec::new();
    let mut has_bilingual = false;
    for stanza in body.split("\n\n") {
        let trimmed = stanza.trim();
        if trimmed.is_empty() {
            continue;
        }
        let mut p_lines = Vec::new();
        let mut s_lines = Vec::new();
        for l in trimmed.lines() {
            if let Some((p, s)) = l.split_once("||") {
                has_bilingual = true;
                p_lines.push(p.trim().to_owned());
                s_lines.push(s.trim().to_owned());
            } else {
                p_lines.push(l.trim().to_owned());
            }
        }
        let ordinal = sections.len() as u32;
        sections.push(SongSection {
            id: SectionId::new_v7(),
            kind: SectionKind::Verse,
            label: format!("Verse {}", ordinal + 1),
            ordinal,
            body: p_lines.join("\n"),
            secondary_body: (!s_lines.is_empty()).then(|| s_lines.join("\n")),
            stage_chords: None,
        });
    }
    if sections.is_empty() {
        return None;
    }
    let tradition = match entry.hymnal_code.as_str() {
        "BH" => "Baptist",
        "HC" | "CC" => "Portuguese / Brazil",
        "MHB" | "UMH" => "Methodist",
        "HGT" | "HIMNO" => "Spanish / Alabanza",
        _ => "Traditional Hymns",
    };
    Some(finalize_imported_song(
        title,
        String::new(),
        String::new(),
        "en",
        has_bilingual.then(|| "en".to_owned()),
        vec![entry],
        vec![tradition.to_owned()],
        sections,
    ))
}

fn strip_bracket_chords(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut in_bracket = false;
    for ch in line.chars() {
        match ch {
            '[' => in_bracket = true,
            ']' => in_bracket = false,
            _ if !in_bracket => out.push(ch),
            _ => {}
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}
