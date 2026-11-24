use std::collections::HashMap;
use std::path::Path;

use rusqlite::Connection;
use sundaydeck_core::{
    Arrangement, ArrangementId, ReflowMode, SectionId, SectionKind, Song, SongId, SongSection,
    strip_rtf_to_plain_text,
};

use crate::error::DbError;

/// Migrates an EasyWorship 6/7 SQLite song library (`Songs.db` + `SongWords.db`),
/// stripping RTF control codes cleanly into SundayDeck [`Song`] records.
pub fn import_easyworship_sqlite(
    songs_db_path: &Path,
    words_db_path: &Path,
) -> Result<Vec<Song>, DbError> {
    let songs_conn = Connection::open(songs_db_path)?;
    let words_conn = Connection::open(words_db_path)?;

    let mut words_by_id: HashMap<i64, String> = HashMap::new();
    {
        let mut w_stmt = words_conn.prepare("SELECT song_id, words FROM word")?;
        let rows = w_stmt.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))?;
        for row in rows.flatten() {
            words_by_id.insert(row.0, row.1);
        }
    }

    let mut stmt = songs_conn.prepare(
        "SELECT rowid, COALESCE(title, ''), COALESCE(author, ''), COALESCE(copyright, '')
         FROM song ORDER BY title COLLATE NOCASE ASC",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, i64>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, String>(3)?,
        ))
    })?;

    let mut imported = Vec::new();
    for row in rows {
        let (rowid, title, author, copyright) = row?;
        let Some(raw_words) = words_by_id.get(&rowid) else {
            continue;
        };
        let plain = strip_rtf_to_plain_text(raw_words);
        let sections = split_stanzas_to_sections(&plain);
        if sections.is_empty() {
            continue;
        }
        imported.push(build_migrated_song(
            if title.trim().is_empty() {
                "EasyWorship Song".to_owned()
            } else {
                title.trim().to_owned()
            },
            author.trim().to_owned(),
            copyright.trim().to_owned(),
            String::new(),
            sections,
        ));
    }
    Ok(imported)
}

/// Migrates an OpenLP `songs.sqlite` database, parsing XML `<verse>` tags and linked authors.
pub fn import_openlp_sqlite(songs_sqlite_path: &Path) -> Result<Vec<Song>, DbError> {
    let ext = Connection::open(songs_sqlite_path)?;
    let mut authors_by_song: HashMap<i64, Vec<String>> = HashMap::new();
    if let Ok(mut a_stmt) = ext.prepare(
        "SELECT asg.song_id, a.display_name
         FROM authors_songs asg
         JOIN authors a ON a.id = asg.author_id",
    ) && let Ok(rows) = a_stmt.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))
    {
        for (sid, name) in rows.flatten() {
            authors_by_song.entry(sid).or_default().push(name);
        }
    }

    let mut stmt = ext.prepare(
        "SELECT id, COALESCE(title, ''), COALESCE(lyrics, ''),
                COALESCE(copyright, ''), COALESCE(ccli_number, '')
         FROM songs ORDER BY title COLLATE NOCASE ASC",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, i64>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, String>(3)?,
            r.get::<_, String>(4)?,
        ))
    })?;

    let mut imported = Vec::new();
    for row in rows {
        let (sid, title, lyrics_xml, copyright, ccli) = row?;
        let sections = parse_openlp_lyrics_xml(&lyrics_xml);
        if sections.is_empty() {
            continue;
        }
        let author = authors_by_song
            .remove(&sid)
            .map(|v| v.join(", "))
            .unwrap_or_default();
        imported.push(build_migrated_song(
            if title.trim().is_empty() {
                "OpenLP Song".to_owned()
            } else {
                title.trim().to_owned()
            },
            author,
            copyright.trim().to_owned(),
            ccli.trim().to_owned(),
            sections,
        ));
    }
    Ok(imported)
}

fn parse_openlp_lyrics_xml(xml: &str) -> Vec<SongSection> {
    let mut sections = Vec::new();
    let mut rest = xml;
    while let Some(start) = rest.find("<verse") {
        let after_tag = &rest[start..];
        let Some(open_end) = after_tag.find('>') else {
            break;
        };
        let header = &after_tag[..open_end];
        let body_start = &after_tag[open_end + 1..];
        let Some(close_pos) = body_start.find("</verse>") else {
            break;
        };
        let raw_inner = &body_start[..close_pos];
        let cleaned = raw_inner
            .replace("<![CDATA[", "")
            .replace("]]>", "")
            .replace("&amp;", "&")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .trim()
            .to_owned();
        if !cleaned.is_empty() {
            let is_chorus = header.contains("type=\"c\"") || header.contains("type='c'");
            let kind = if is_chorus {
                SectionKind::Chorus
            } else {
                SectionKind::Verse
            };
            let label = if is_chorus {
                "Chorus".to_owned()
            } else {
                format!("Verse {}", sections.len() + 1)
            };
            sections.push(SongSection {
                id: SectionId::new_v7(),
                kind,
                label,
                ordinal: sections.len() as u32,
                body: cleaned,
                secondary_body: None,
                stage_chords: None,
            });
        }
        rest = &body_start[close_pos + 8..];
    }
    if sections.is_empty() {
        split_stanzas_to_sections(xml)
    } else {
        sections
    }
}

fn split_stanzas_to_sections(plain: &str) -> Vec<SongSection> {
    let mut sections = Vec::new();
    for block in plain.split("\n\n") {
        let body = block.trim();
        if body.is_empty() {
            continue;
        }
        let lower = body.to_ascii_lowercase();
        let (kind, label) = if lower.starts_with("chorus") {
            (SectionKind::Chorus, "Chorus".to_owned())
        } else {
            (SectionKind::Verse, format!("Verse {}", sections.len() + 1))
        };
        sections.push(SongSection {
            id: SectionId::new_v7(),
            kind,
            label,
            ordinal: sections.len() as u32,
            body: body.to_owned(),
            secondary_body: None,
            stage_chords: None,
        });
    }
    sections
}

fn build_migrated_song(
    title: String,
    author: String,
    copyright: String,
    ccli_number: String,
    sections: Vec<SongSection>,
) -> Song {
    let arr_id = ArrangementId::new_v7();
    let section_ids = sections.iter().map(|s| s.id).collect();
    Song {
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
            name: "Standard".to_owned(),
            section_ids,
        }],
    }
}
