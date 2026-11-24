use crate::error::ImportError;
use crate::global_content::HymnalEntry;
use crate::ids::{ArrangementId, SectionId, SongId};
use crate::song::{Arrangement, ReflowMode, SectionKind, Song, SongSection};
use crate::song_import::detect_and_import_song;

/// Imports ProPresenter 6 (`.pro6`) XML slide documents into a SundayDeck [`Song`].
pub fn import_propresenter6_xml(xml: &str) -> Result<Song, ImportError> {
    let mut sections = Vec::new();
    let mut rest = xml;
    while let Some(pos) = rest.find("RTFData=\"") {
        let after = &rest[pos + 9..];
        let Some(end) = after.find('"') else { break };
        let raw_text = strip_rtf_to_plain_text(&after[..end]);
        if !raw_text.is_empty() {
            sections.push(SongSection {
                id: SectionId::new_v7(),
                kind: SectionKind::Verse,
                label: format!("Slide {}", sections.len() + 1),
                ordinal: sections.len() as u32,
                body: raw_text,
                secondary_body: None,
                stage_chords: None,
            });
        }
        rest = &after[end + 1..];
    }
    if sections.is_empty() {
        return detect_and_import_song("propresenter.txt", xml);
    }
    Ok(finalize_imported_song(
        "Imported ProPresenter Song".to_owned(),
        String::new(),
        String::new(),
        "en",
        None,
        Vec::new(),
        Vec::new(),
        sections,
    ))
}

/// Imports FreeShow (`.show`/`.json`) or VideoPsalm (`.vpc`/`.json`) JSON song structures.
pub fn import_freeshow_or_videopsalm_json(raw: &str) -> Result<Vec<Song>, ImportError> {
    let val: serde_json::Value =
        serde_json::from_str(raw).map_err(|e| ImportError::UnsupportedFormat {
            detail: e.to_string(),
        })?;
    let title = val
        .get("name")
        .or_else(|| val.get("Text"))
        .and_then(|v| v.as_str())
        .unwrap_or("Imported Song")
        .to_owned();
    let mut sections = Vec::new();
    if let Some(slides) = val.get("slides").and_then(|v| v.as_object()) {
        for (_, slide_val) in slides {
            if let Some(items) = slide_val.get("items").and_then(|v| v.as_array()) {
                let mut lines = Vec::new();
                for item in items {
                    if let Some(l_arr) = item.get("lines").and_then(|v| v.as_array()) {
                        for l in l_arr {
                            if let Some(t_arr) = l.get("text").and_then(|v| v.as_array()) {
                                let joined: String = t_arr
                                    .iter()
                                    .filter_map(|t| t.get("value").and_then(|v| v.as_str()))
                                    .collect();
                                if !joined.trim().is_empty() {
                                    lines.push(joined.trim().to_owned());
                                }
                            }
                        }
                    }
                }
                if !lines.is_empty() {
                    sections.push(SongSection {
                        id: SectionId::new_v7(),
                        kind: SectionKind::Verse,
                        label: format!("Slide {}", sections.len() + 1),
                        ordinal: sections.len() as u32,
                        body: lines.join("\n"),
                        secondary_body: None,
                        stage_chords: None,
                    });
                }
            }
        }
    }
    if let Some(verses) = val.get("Verses").and_then(|v| v.as_array()) {
        for v in verses {
            if let Some(txt) = v.get("Text").and_then(|t| t.as_str())
                && !txt.trim().is_empty()
            {
                sections.push(SongSection {
                    id: SectionId::new_v7(),
                    kind: SectionKind::Verse,
                    label: format!("Verse {}", sections.len() + 1),
                    ordinal: sections.len() as u32,
                    body: txt.trim().to_owned(),
                    secondary_body: None,
                    stage_chords: None,
                });
            }
        }
    }
    if sections.is_empty() {
        return Err(ImportError::NoContent {
            kind: "FreeShow/VideoPsalm slides",
        });
    }
    Ok(vec![finalize_imported_song(
        title,
        String::new(),
        String::new(),
        "en",
        None,
        Vec::new(),
        Vec::new(),
        sections,
    )])
}

/// Strips RTF control words (`\par`, `\fs24`, `{\fonttbl ...}`) used by EasyWorship & ProPresenter
/// into crisp UTF-8 plain lyrics.
pub fn strip_rtf_to_plain_text(rtf: &str) -> String {
    if !rtf.trim_start().starts_with("{\\rtf") {
        return rtf.trim().to_owned();
    }
    let mut out = String::with_capacity(rtf.len());
    let chars: Vec<char> = rtf.chars().collect();
    let mut i = 0;
    let mut skip_group_depth: Option<usize> = None;
    let mut depth = 0usize;

    while i < chars.len() {
        match chars[i] {
            '{' => {
                depth += 1;
                let ahead: String = chars[i + 1..(i + 14).min(chars.len())].iter().collect();
                if skip_group_depth.is_none()
                    && (ahead.starts_with("\\fonttbl")
                        || ahead.starts_with("\\colortbl")
                        || ahead.starts_with("\\stylesheet")
                        || ahead.starts_with("\\*"))
                {
                    skip_group_depth = Some(depth);
                }
                i += 1;
            }
            '}' => {
                if skip_group_depth == Some(depth) {
                    skip_group_depth = None;
                }
                depth = depth.saturating_sub(1);
                i += 1;
            }
            '\\' => {
                i += 1;
                if i >= chars.len() {
                    break;
                }
                if chars[i] == '\'' && i + 2 < chars.len() {
                    let hex: String = chars[i + 1..i + 3].iter().collect();
                    if skip_group_depth.is_none()
                        && let Ok(byte) = u8::from_str_radix(&hex, 16)
                    {
                        out.push(byte as char);
                    }
                    i += 3;
                    continue;
                }
                let start = i;
                while i < chars.len() && chars[i].is_ascii_alphabetic() {
                    i += 1;
                }
                let word: String = chars[start..i].iter().collect();
                while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '-') {
                    i += 1;
                }
                if i < chars.len() && chars[i] == ' ' {
                    i += 1;
                }
                if skip_group_depth.is_none() && (word == "par" || word == "line") {
                    out.push('\n');
                }
            }
            '\r' | '\n' => i += 1,
            ch => {
                if skip_group_depth.is_none() {
                    out.push(ch);
                }
                i += 1;
            }
        }
    }
    out.lines()
        .map(str::trim)
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_owned()
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn finalize_imported_song(
    title: String,
    author: String,
    ccli_number: String,
    primary_lang: &str,
    secondary_lang: Option<String>,
    hymnal_entries: Vec<HymnalEntry>,
    traditions: Vec<String>,
    sections: Vec<SongSection>,
) -> Song {
    let arr_id = ArrangementId::new_v7();
    let section_ids = sections.iter().map(|s| s.id).collect();
    let alternate_title = hymnal_entries
        .iter()
        .map(HymnalEntry::display_code)
        .collect::<Vec<_>>()
        .join(" · ");
    Song {
        id: SongId::new_v7(),
        title,
        alternate_title,
        author,
        copyright: String::new(),
        ccli_number,
        default_theme_id: None,
        default_arrangement_id: Some(arr_id),
        default_reflow_mode: ReflowMode::ByStanza,
        primary_lang: primary_lang.to_owned(),
        secondary_lang,
        hymnal_entries,
        traditions,
        sections,
        arrangements: vec![Arrangement {
            id: arr_id,
            name: "Standard".to_owned(),
            section_ids,
        }],
    }
}
