use std::collections::BTreeMap;
use std::path::Path;

use rusqlite::{Connection, OptionalExtension, params};
use sundaydeck_core::{
    BibleTranslation, BibleTranslationId, BibleVerse, ParsedBible, ParsedVerse, ScriptureRef,
    book_by_number,
};

use crate::connection::now_epoch_ms;
use crate::error::DbError;

pub fn list_translations(conn: &Connection) -> Result<Vec<BibleTranslation>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT id, code, name, language, copyright
         FROM bible_translations
         ORDER BY code COLLATE NOCASE ASC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
        ))
    })?;

    let mut out = Vec::new();
    for row in rows {
        let (id_str, code, name, language, copyright) = row?;
        out.push(BibleTranslation {
            id: BibleTranslationId::parse_str(&id_str)?,
            code,
            name,
            language,
            copyright,
        });
    }
    Ok(out)
}

/// Bulk-imports a [`ParsedBible`] atomically, replacing any existing translation with the same code.
pub fn import_parsed_bible(
    conn: &mut Connection,
    parsed: &ParsedBible,
) -> Result<BibleTranslation, DbError> {
    let now = now_epoch_ms();
    let code = parsed.code.trim().to_ascii_uppercase();
    let tx = conn.transaction()?;

    tx.execute("DELETE FROM bible_translations WHERE code = ?1", [&code])?;

    let translation_id = BibleTranslationId::new_v7();
    let id_str = translation_id.to_string();
    tx.execute(
        "INSERT INTO bible_translations (id, code, name, language, copyright, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            id_str,
            code,
            parsed.name,
            parsed.language,
            parsed.copyright,
            now
        ],
    )?;

    let mut max_chapter_per_book: BTreeMap<u8, u16> = BTreeMap::new();
    for v in &parsed.verses {
        max_chapter_per_book
            .entry(v.book_number)
            .and_modify(|max_c| *max_c = (*max_c).max(v.chapter))
            .or_insert(v.chapter);
    }

    {
        let mut book_stmt = tx.prepare(
            "INSERT INTO bible_books (translation_id, book_number, name, chapter_count)
             VALUES (?1, ?2, ?3, ?4)",
        )?;
        for (book_num, chapter_count) in max_chapter_per_book {
            let book_name = book_by_number(book_num)
                .map(|m| m.name)
                .unwrap_or("Unknown");
            book_stmt.execute(params![
                id_str,
                i64::from(book_num),
                book_name,
                i64::from(chapter_count)
            ])?;
        }
    }

    {
        let mut verse_stmt = tx.prepare(
            "INSERT OR REPLACE INTO bible_verses (translation_id, book_number, chapter, verse, text)
             VALUES (?1, ?2, ?3, ?4, ?5)",
        )?;
        for v in &parsed.verses {
            verse_stmt.execute(params![
                id_str,
                i64::from(v.book_number),
                i64::from(v.chapter),
                i64::from(v.verse),
                v.text,
            ])?;
        }
    }

    tx.commit()?;
    Ok(BibleTranslation {
        id: translation_id,
        code,
        name: parsed.name.clone(),
        language: parsed.language.clone(),
        copyright: parsed.copyright.clone(),
    })
}

/// Imports an open external SQLite Bible database supporting both standard
/// `verses(book, chapter, verse, text)` and Scrollmapper (`t_*(b, c, v, t)`) schemas.
pub fn import_sqlite_bible(
    conn: &mut Connection,
    external_path: &Path,
    code: &str,
    name: &str,
) -> Result<BibleTranslation, DbError> {
    let ext = Connection::open(external_path)?;
    let query = detect_external_sqlite_bible_query(&ext).ok_or_else(|| DbError::NotFound {
        entity: "sqlite_bible_table",
        id: external_path.display().to_string(),
    })?;

    let mut stmt = ext.prepare(&query)?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, i64>(2)?,
            row.get::<_, String>(3)?,
        ))
    })?;

    let mut verses = Vec::new();
    for row in rows {
        let (b, c, v, text) = row?;
        if let (Ok(book_number), Ok(chapter), Ok(verse)) =
            (u8::try_from(b), u16::try_from(c), u16::try_from(v))
        {
            let cleaned: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
            if book_by_number(book_number).is_some()
                && chapter > 0
                && verse > 0
                && !cleaned.is_empty()
            {
                verses.push(ParsedVerse {
                    book_number,
                    chapter,
                    verse,
                    text: cleaned,
                });
            }
        }
    }

    let parsed = ParsedBible {
        code: code.to_owned(),
        name: name.to_owned(),
        language: "en".to_owned(),
        copyright: String::new(),
        verses,
    };
    import_parsed_bible(conn, &parsed)
}

/// Loads all verses in a chapter for smooth continuous-scroll browsing during preaching.
pub fn get_chapter_verses(
    conn: &Connection,
    translation_code: &str,
    book_number: u8,
    chapter: u16,
) -> Result<Vec<BibleVerse>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT t.code, v.book_number, COALESCE(b.name, ''), v.chapter, v.verse, v.text
         FROM bible_verses v
         JOIN bible_translations t ON t.id = v.translation_id
         LEFT JOIN bible_books b ON b.translation_id = v.translation_id AND b.book_number = v.book_number
         WHERE UPPER(t.code) = UPPER(?1) AND v.book_number = ?2 AND v.chapter = ?3
         ORDER BY v.verse ASC",
    )?;
    collect_verses(stmt.query_map(
        params![translation_code, i64::from(book_number), i64::from(chapter)],
        map_verse_tuple,
    )?)
}

/// Loads a specific verse, verse range, or entire chapter from a parsed [`ScriptureRef`].
pub fn get_passage(
    conn: &Connection,
    translation_code: &str,
    reference: &ScriptureRef,
) -> Result<Vec<BibleVerse>, DbError> {
    match (reference.verse_start, reference.verse_end) {
        (Some(start), Some(end)) => {
            let mut stmt = conn.prepare(
                "SELECT t.code, v.book_number, COALESCE(b.name, ''), v.chapter, v.verse, v.text
                 FROM bible_verses v
                 JOIN bible_translations t ON t.id = v.translation_id
                 LEFT JOIN bible_books b ON b.translation_id = v.translation_id AND b.book_number = v.book_number
                 WHERE UPPER(t.code) = UPPER(?1)
                   AND v.book_number = ?2
                   AND v.chapter = ?3
                   AND v.verse BETWEEN ?4 AND ?5
                 ORDER BY v.verse ASC",
            )?;
            collect_verses(stmt.query_map(
                params![
                    translation_code,
                    i64::from(reference.book_number),
                    i64::from(reference.chapter),
                    i64::from(start),
                    i64::from(end),
                ],
                map_verse_tuple,
            )?)
        }
        (Some(start), None) => get_passage(
            conn,
            translation_code,
            &ScriptureRef {
                verse_end: Some(start),
                ..*reference
            },
        ),
        (None, _) => get_chapter_verses(
            conn,
            translation_code,
            reference.book_number,
            reference.chapter,
        ),
    }
}

/// Sub-millisecond FTS5 keyword search scoped to a translation.
pub fn search_bible_verses(
    conn: &Connection,
    translation_code: &str,
    query: &str,
    limit: usize,
) -> Result<Vec<BibleVerse>, DbError> {
    let Some(fts_query) = build_fts5_prefix_query(query) else {
        return Ok(Vec::new());
    };
    let max_rows = i64::try_from(limit.clamp(1, 200)).unwrap_or(50);

    let mut stmt = conn.prepare(
        "SELECT t.code, v.book_number, COALESCE(b.name, ''), v.chapter, v.verse, v.text
         FROM bible_verses_fts f
         JOIN bible_verses v ON v.id = f.rowid
         JOIN bible_translations t ON t.id = v.translation_id
         LEFT JOIN bible_books b ON b.translation_id = v.translation_id AND b.book_number = v.book_number
         WHERE bible_verses_fts MATCH ?1 AND UPPER(t.code) = UPPER(?2)
         ORDER BY bm25(bible_verses_fts), v.book_number ASC, v.chapter ASC, v.verse ASC
         LIMIT ?3",
    )?;
    collect_verses(stmt.query_map(
        params![fts_query, translation_code, max_rows],
        map_verse_tuple,
    )?)
}

fn map_verse_tuple(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<(String, i64, String, i64, i64, String)> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
        row.get(5)?,
    ))
}

fn collect_verses<I>(rows: I) -> Result<Vec<BibleVerse>, DbError>
where
    I: Iterator<Item = rusqlite::Result<(String, i64, String, i64, i64, String)>>,
{
    let mut out = Vec::new();
    for row in rows {
        let (code, b, name, c, v, text) = row?;
        let book_number = u8::try_from(b).unwrap_or(1);
        let book_name = if name.is_empty() {
            book_by_number(book_number)
                .map(|m| m.name.to_owned())
                .unwrap_or_else(|| format!("Book {book_number}"))
        } else {
            name
        };
        out.push(BibleVerse {
            translation_code: code,
            book_number,
            book_name,
            chapter: u16::try_from(c).unwrap_or(1),
            verse: u16::try_from(v).unwrap_or(1),
            text,
        });
    }
    Ok(out)
}

fn detect_external_sqlite_bible_query(ext: &Connection) -> Option<String> {
    let has_verses: Option<String> = ext
        .query_row(
            "SELECT name FROM sqlite_master WHERE type='table' AND LOWER(name) = 'verses' LIMIT 1",
            [],
            |r| r.get(0),
        )
        .optional()
        .ok()
        .flatten();
    if let Some(table) = has_verses {
        let cols = table_columns(ext, &table);
        if cols.iter().any(|c| c == "book_number") {
            return Some(format!(
                "SELECT book_number, chapter, verse, text FROM \"{table}\""
            ));
        }
        if cols.iter().any(|c| c == "book") {
            return Some(format!(
                "SELECT book, chapter, verse, text FROM \"{table}\""
            ));
        }
    }

    let scrollmapper: Option<String> = ext
        .query_row(
            "SELECT name FROM sqlite_master WHERE type='table' AND name LIKE 't_%' LIMIT 1",
            [],
            |r| r.get(0),
        )
        .optional()
        .ok()
        .flatten();
    if let Some(table) = scrollmapper {
        let cols = table_columns(ext, &table);
        if cols.iter().any(|c| c == "b") && cols.iter().any(|c| c == "t") {
            return Some(format!("SELECT b, c, v, t FROM \"{table}\""));
        }
    }
    None
}

fn table_columns(conn: &Connection, table: &str) -> Vec<String> {
    let pragma = format!("PRAGMA table_info(\"{table}\")");
    let Ok(mut stmt) = conn.prepare(&pragma) else {
        return Vec::new();
    };
    let Ok(rows) = stmt.query_map([], |r| r.get::<_, String>(1)) else {
        return Vec::new();
    };
    rows.flatten().map(|s| s.to_ascii_lowercase()).collect()
}

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
