use crate::error::ImportError;
use crate::ids::{ArrangementId, SectionId, SongId};
use crate::openlyrics::import_openlyrics_xml;
use crate::song::{Arrangement, ReflowMode, SectionKind, Song, SongSection};

/// Detects OpenLyrics XML, SongSelect `.usr`, or SongSelect/plain `.txt` and imports a [`Song`].
pub fn detect_and_import_song(filename: &str, raw: &str) -> Result<Song, ImportError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(ImportError::EmptyInput);
    }

    let lower = filename.to_ascii_lowercase();
    if lower.ends_with(".xml") || trimmed.starts_with('<') {
        return import_openlyrics_xml(trimmed);
    }
    if lower.ends_with(".usr") || trimmed.starts_with("[File]") {
        return import_songselect_usr(trimmed);
    }

    let fallback_title = std::path::Path::new(filename)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Imported Song");
    import_plain_text_song(fallback_title, trimmed)
}

/// Imports a SongSelect `.usr` file (`[File]`, `Title=`, `Fields=`, `Words=` with `/t` & `/n`).
pub fn import_songselect_usr(raw: &str) -> Result<Song, ImportError> {
    let mut title = String::new();
    let mut author = String::new();
    let mut copyright = String::new();
    let mut ccli_number = String::new();
    let mut fields_raw = String::new();
    let mut words_raw = String::new();

    for line in raw.lines() {
        let Some((key, value)) = line.trim().split_once('=') else {
            continue;
        };
        let value = value.trim();
        match key.trim() {
            "Title" => title = value.to_owned(),
            "Author" => author = value.replace('|', ", "),
            "Copyright" => copyright = value.to_owned(),
            "CCLI" | "SongNumber" => ccli_number = value.to_owned(),
            "Fields" => fields_raw = value.to_owned(),
            "Words" => words_raw = value.to_owned(),
            _ => {}
        }
    }

    let labels: Vec<&str> = fields_raw
        .split("/t")
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    let bodies: Vec<&str> = words_raw
        .split("/t")
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();

    let mut sections = Vec::new();
    for (idx, body_raw) in bodies.iter().enumerate() {
        let label_raw = labels.get(idx).copied().unwrap_or("Verse");
        let (kind, label) = classify_section_header(label_raw, idx + 1);
        let body = body_raw
            .replace("/n", "\n")
            .lines()
            .map(str::trim)
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
            .to_owned();
        if !body.is_empty() {
            sections.push(SongSection {
                id: SectionId::new_v7(),
                kind,
                label,
                ordinal: sections.len() as u32,
                body,
                secondary_body: None,
                stage_chords: None,
            });
        }
    }

    if sections.is_empty() {
        return Err(ImportError::NoContent {
            kind: "song lyrics",
        });
    }
    if title.is_empty() {
        title = "Imported SongSelect Song".to_owned();
    }

    let arr_id = ArrangementId::new_v7();
    let section_ids = sections.iter().map(|s| s.id).collect();
    Ok(Song {
        id: SongId::new_v7(),
        title,
        alternate_title: String::new(),
        author,
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
            section_ids,
        }],
    })
}

/// Imports SongSelect plain text (`.txt`) or blank-line-delimited lyric text files.
pub fn import_plain_text_song(title_hint: &str, raw: &str) -> Result<Song, ImportError> {
    let blocks = split_text_blocks(raw);
    if blocks.is_empty() {
        return Err(ImportError::EmptyInput);
    }

    let mut title = title_hint.trim().to_owned();
    let mut author = String::new();
    let mut copyright = String::new();
    let mut ccli_number = String::new();
    let mut sections: Vec<SongSection> = Vec::new();

    for (block_idx, lines) in blocks.into_iter().enumerate() {
        if lines
            .iter()
            .any(|l| l.to_ascii_lowercase().starts_with("ccli song #"))
        {
            extract_songselect_footer(&lines, &mut author, &mut copyright, &mut ccli_number);
            continue;
        }
        if block_idx == 0 && lines.len() == 1 && !looks_like_section_header(&lines[0]) {
            title = lines[0].clone();
            continue;
        }

        let (kind, label, body_lines) = if looks_like_section_header(&lines[0]) && lines.len() > 1 {
            let (k, l) = classify_section_header(&lines[0], sections.len() + 1);
            (k, l, &lines[1..])
        } else {
            (
                SectionKind::Verse,
                format!("Verse {}", sections.len() + 1),
                &lines[..],
            )
        };

        let body = body_lines.join("\n").trim().to_owned();
        if !body.is_empty() {
            sections.push(SongSection {
                id: SectionId::new_v7(),
                kind,
                label,
                ordinal: sections.len() as u32,
                body,
                secondary_body: None,
                stage_chords: None,
            });
        }
    }

    if sections.is_empty() {
        return Err(ImportError::NoContent {
            kind: "song lyrics",
        });
    }
    if title.is_empty() {
        title = "Imported Song".to_owned();
    }

    let arr_id = ArrangementId::new_v7();
    let section_ids = sections.iter().map(|s| s.id).collect();
    Ok(Song {
        id: SongId::new_v7(),
        title,
        alternate_title: String::new(),
        author,
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
            section_ids,
        }],
    })
}

fn split_text_blocks(raw: &str) -> Vec<Vec<String>> {
    let mut blocks = Vec::new();
    let mut current = Vec::new();
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if !current.is_empty() {
                blocks.push(std::mem::take(&mut current));
            }
        } else {
            current.push(trimmed.to_owned());
        }
    }
    if !current.is_empty() {
        blocks.push(current);
    }
    blocks
}

fn extract_songselect_footer(
    lines: &[String],
    author: &mut String,
    copyright: &mut String,
    ccli: &mut String,
) {
    for line in lines {
        let lower = line.to_ascii_lowercase();
        if let Some(rest) = lower.strip_prefix("ccli song #") {
            *ccli = rest.trim().to_owned();
        } else if line.contains('©')
            || lower.contains("copyright")
            || lower.contains("public domain")
        {
            if !copyright.is_empty() {
                copyright.push_str(" | ");
            }
            copyright.push_str(line);
        } else if !lower.starts_with("ccli license") && author.is_empty() {
            *author = line.clone();
        }
    }
}

fn looks_like_section_header(line: &str) -> bool {
    let clean = line
        .trim()
        .trim_matches(|c| matches!(c, '[' | ']' | ':'))
        .to_ascii_lowercase();
    let prefixes = [
        "verse",
        "chorus",
        "pre-chorus",
        "pre chorus",
        "bridge",
        "tag",
        "ending",
        "outro",
        "intro",
        "interlude",
        "refrain",
    ];
    prefixes.iter().any(|p| clean.starts_with(p)) && clean.len() <= 28
}

fn classify_section_header(raw: &str, fallback_num: usize) -> (SectionKind, String) {
    let clean = raw
        .trim()
        .trim_matches(|c| matches!(c, '[' | ']' | ':'))
        .trim();
    let lower = clean.to_ascii_lowercase();
    let kind = if lower.starts_with("pre-chorus") || lower.starts_with("pre chorus") {
        SectionKind::PreChorus
    } else if lower.starts_with("chorus") || lower.starts_with("refrain") {
        SectionKind::Chorus
    } else if lower.starts_with("bridge") {
        SectionKind::Bridge
    } else if lower.starts_with("tag") {
        SectionKind::Tag
    } else if lower.starts_with("ending") || lower.starts_with("outro") {
        SectionKind::Ending
    } else if lower.starts_with("intro") {
        SectionKind::Intro
    } else if lower.starts_with("interlude") {
        SectionKind::Interlude
    } else if lower.starts_with("verse") {
        SectionKind::Verse
    } else {
        SectionKind::Other
    };
    let label = if clean.is_empty() {
        format!("Verse {fallback_num}")
    } else {
        clean.to_owned()
    };
    (kind, label)
}
