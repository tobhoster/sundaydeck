use rusqlite::{Connection, OptionalExtension, params};
use sundaydeck_core::song::SongSummary;
use sundaydeck_core::{
    Arrangement, ArrangementId, ReflowMode, SectionId, SectionKind, Song, SongId, SongSection,
    ThemeId,
};

use crate::connection::now_epoch_ms;
use crate::error::DbError;

pub fn list_songs(conn: &Connection) -> Result<Vec<SongSummary>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT s.id, s.title, s.author, s.ccli_number,
                (SELECT COUNT(*) FROM song_sections sec WHERE sec.song_id = s.id)
         FROM songs s
         ORDER BY s.title COLLATE NOCASE ASC",
    )?;

    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, i64>(4)?,
        ))
    })?;

    let mut out = Vec::new();
    for row in rows {
        let (id_str, title, author, ccli_number, count) = row?;
        out.push(SongSummary {
            id: SongId::parse_str(&id_str)?,
            title,
            author,
            ccli_number,
            section_count: usize::try_from(count).unwrap_or(0),
        });
    }
    Ok(out)
}

/// Sub-millisecond FTS5 prefix search across title, author, CCLI number, and lyrics.
pub fn search_songs(conn: &Connection, query: &str) -> Result<Vec<SongSummary>, DbError> {
    let Some(fts_query) = build_fts5_prefix_query(query) else {
        return list_songs(conn);
    };

    let mut stmt = conn.prepare(
        "SELECT s.id, s.title, s.author, s.ccli_number,
                (SELECT COUNT(*) FROM song_sections sec WHERE sec.song_id = s.id)
         FROM songs_fts f
         JOIN songs s ON s.rowid = f.rowid
         WHERE songs_fts MATCH ?1
         ORDER BY bm25(songs_fts, 10.0, 5.0, 4.0, 8.0, 1.0), s.title COLLATE NOCASE ASC
         LIMIT 100",
    )?;

    let rows = stmt.query_map([fts_query], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, i64>(4)?,
        ))
    })?;

    let mut out = Vec::new();
    for row in rows {
        let (id_str, title, author, ccli_number, count) = row?;
        out.push(SongSummary {
            id: SongId::parse_str(&id_str)?,
            title,
            author,
            ccli_number,
            section_count: usize::try_from(count).unwrap_or(0),
        });
    }
    Ok(out)
}

pub fn get_song(conn: &Connection, id: SongId) -> Result<Option<Song>, DbError> {
    let header = conn
        .query_row(
            "SELECT id, title, alternate_title, author, copyright, ccli_number,
                    default_theme_id, default_arrangement_id, default_reflow_mode
             FROM songs WHERE id = ?1",
            [id.to_string()],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, Option<String>>(7)?,
                    row.get::<_, String>(8)?,
                ))
            },
        )
        .optional()?;

    let Some((
        id_str,
        title,
        alternate_title,
        author,
        copyright,
        ccli_number,
        default_theme_str,
        default_arr_str,
        reflow_str,
    )) = header
    else {
        return Ok(None);
    };

    let song_id = SongId::parse_str(&id_str)?;
    let sections = load_song_sections(conn, song_id)?;
    let arrangements = load_song_arrangements(conn, song_id)?;

    Ok(Some(Song {
        id: song_id,
        title,
        alternate_title,
        author,
        copyright,
        ccli_number,
        default_theme_id: default_theme_str
            .as_deref()
            .map(ThemeId::parse_str)
            .transpose()?,
        default_arrangement_id: default_arr_str
            .as_deref()
            .map(ArrangementId::parse_str)
            .transpose()?,
        default_reflow_mode: ReflowMode::from_db_str(&reflow_str)?,
        sections,
        arrangements,
    }))
}

pub fn upsert_song(conn: &mut Connection, song: &Song) -> Result<(), DbError> {
    let now = now_epoch_ms();
    let search_text = song.lyrics_search_text();
    let song_id_str = song.id.to_string();

    let tx = conn.transaction()?;
    tx.execute(
        "INSERT INTO songs (
             id, title, alternate_title, author, copyright, ccli_number,
             default_theme_id, default_arrangement_id, default_reflow_mode,
             lyrics_search_text, created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
         ON CONFLICT(id) DO UPDATE SET
             title = excluded.title,
             alternate_title = excluded.alternate_title,
             author = excluded.author,
             copyright = excluded.copyright,
             ccli_number = excluded.ccli_number,
             default_theme_id = excluded.default_theme_id,
             default_arrangement_id = excluded.default_arrangement_id,
             default_reflow_mode = excluded.default_reflow_mode,
             lyrics_search_text = excluded.lyrics_search_text,
             updated_at = excluded.updated_at",
        params![
            song_id_str,
            song.title,
            song.alternate_title,
            song.author,
            song.copyright,
            song.ccli_number,
            song.default_theme_id.map(|id| id.to_string()),
            song.default_arrangement_id.map(|id| id.to_string()),
            song.default_reflow_mode.as_db_str(),
            search_text,
            now,
            now,
        ],
    )?;

    tx.execute(
        "DELETE FROM song_arrangements WHERE song_id = ?1",
        [&song_id_str],
    )?;
    tx.execute(
        "DELETE FROM song_sections WHERE song_id = ?1",
        [&song_id_str],
    )?;

    for (idx, section) in song.sections.iter().enumerate() {
        let ordinal = i64::try_from(idx).unwrap_or(0);
        tx.execute(
            "INSERT INTO song_sections (id, song_id, kind, label, ordinal, body)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                section.id.to_string(),
                song_id_str,
                section.kind.as_db_str(),
                section.label,
                ordinal,
                section.body,
            ],
        )?;
    }

    for arr in &song.arrangements {
        let arr_id_str = arr.id.to_string();
        tx.execute(
            "INSERT INTO song_arrangements (id, song_id, name, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![arr_id_str, song_id_str, arr.name, now, now],
        )?;
        for (pos, sec_id) in arr.section_ids.iter().enumerate() {
            tx.execute(
                "INSERT INTO song_arrangement_items (arrangement_id, position, section_id)
                 VALUES (?1, ?2, ?3)",
                params![
                    arr_id_str,
                    i64::try_from(pos).unwrap_or(0),
                    sec_id.to_string()
                ],
            )?;
        }
    }

    tx.commit()?;
    Ok(())
}

pub fn delete_song(conn: &Connection, id: SongId) -> Result<(), DbError> {
    conn.execute("DELETE FROM songs WHERE id = ?1", [id.to_string()])?;
    Ok(())
}

fn load_song_sections(conn: &Connection, song_id: SongId) -> Result<Vec<SongSection>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT id, kind, label, ordinal, body
         FROM song_sections
         WHERE song_id = ?1
         ORDER BY ordinal ASC",
    )?;
    let rows = stmt.query_map([song_id.to_string()], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, i64>(3)?,
            row.get::<_, String>(4)?,
        ))
    })?;

    let mut sections = Vec::new();
    for row in rows {
        let (id_str, kind_str, label, ordinal, body) = row?;
        sections.push(SongSection {
            id: SectionId::parse_str(&id_str)?,
            kind: SectionKind::from_db_str(&kind_str)?,
            label,
            ordinal: u32::try_from(ordinal).unwrap_or(0),
            body,
        });
    }
    Ok(sections)
}

fn load_song_arrangements(conn: &Connection, song_id: SongId) -> Result<Vec<Arrangement>, DbError> {
    let mut arr_stmt = conn.prepare(
        "SELECT id, name FROM song_arrangements WHERE song_id = ?1 ORDER BY created_at ASC",
    )?;
    let arr_rows = arr_stmt.query_map([song_id.to_string()], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;

    let mut item_stmt = conn.prepare(
        "SELECT section_id FROM song_arrangement_items
         WHERE arrangement_id = ?1
         ORDER BY position ASC",
    )?;

    let mut arrangements = Vec::new();
    for row in arr_rows {
        let (arr_id_str, name) = row?;
        let arr_id = ArrangementId::parse_str(&arr_id_str)?;
        let sec_rows = item_stmt.query_map([&arr_id_str], |r| r.get::<_, String>(0))?;
        let mut section_ids = Vec::new();
        for sec_res in sec_rows {
            section_ids.push(SectionId::parse_str(&sec_res?)?);
        }
        arrangements.push(Arrangement {
            id: arr_id,
            name,
            section_ids,
        });
    }
    Ok(arrangements)
}

/// Sanitizes arbitrary volunteer input into safe FTS5 prefix tokens so quotes
/// or special operators can never throw a syntax error during live search.
fn build_fts5_prefix_query(raw: &str) -> Option<String> {
    let tokens: Vec<String> = raw
        .split(|c: char| !c.is_alphanumeric())
        .filter(|tok| !tok.is_empty())
        .map(|tok| format!("\"{tok}\"*"))
        .collect();
    if tokens.is_empty() {
        None
    } else {
        Some(tokens.join(" "))
    }
}
