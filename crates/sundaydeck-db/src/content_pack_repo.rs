use rusqlite::{Connection, params};
use sundaydeck_core::{
    LiturgyLine, LiturgySpeaker, LiturgyTemplate, ParsedBible, ParsedVerse, SdeckPack,
};

use crate::bible_repo::import_parsed_bible;
use crate::connection::now_epoch_ms;
use crate::error::DbError;
use crate::song_repo::upsert_song;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentPackRecord {
    pub pack_id: String,
    pub name: String,
    pub version: String,
    pub tradition: String,
    pub license: String,
    pub attribution: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PackInstallReport {
    pub bibles_installed: usize,
    pub songs_installed: usize,
    pub liturgies_installed: usize,
}

/// Installs a verified portable `.sdeck-pack` archive (Bibles, numbered hymnals/songs,
/// and liturgical call-and-response templates) and records its license & attribution.
pub fn install_sdeck_pack(
    conn: &mut Connection,
    pack: &SdeckPack,
) -> Result<PackInstallReport, DbError> {
    let mut report = PackInstallReport::default();

    for bible in &pack.bibles {
        let parsed = ParsedBible {
            code: bible.code.clone(),
            name: bible.name.clone(),
            language: bible.language.clone(),
            copyright: bible.copyright.clone(),
            verses: bible
                .verses
                .iter()
                .map(|(book_number, chapter, verse, text)| ParsedVerse {
                    book_number: *book_number,
                    chapter: *chapter,
                    verse: *verse,
                    text: text.clone(),
                })
                .collect(),
        };
        import_parsed_bible(conn, &parsed)?;
        report.bibles_installed += 1;
    }

    for song in &pack.songs {
        upsert_song(conn, song)?;
        report.songs_installed += 1;
    }

    let now = now_epoch_ms();
    let tx = conn.transaction()?;
    for lit in &pack.liturgies {
        tx.execute(
            "INSERT INTO liturgy_templates (id, title, tradition, language, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(id) DO UPDATE SET
                title = excluded.title,
                tradition = excluded.tradition,
                language = excluded.language",
            params![lit.id, lit.title, lit.tradition, lit.language, now],
        )?;
        tx.execute("DELETE FROM liturgy_lines WHERE template_id = ?1", [&lit.id])?;
        for (pos, line) in lit.lines.iter().enumerate() {
            tx.execute(
                "INSERT INTO liturgy_lines (template_id, position, speaker, text, secondary_text)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    lit.id,
                    i64::try_from(pos).unwrap_or(0),
                    speaker_to_db(line.speaker),
                    line.text,
                    line.secondary_text,
                ],
            )?;
        }
        report.liturgies_installed += 1;
    }

    let m = &pack.manifest;
    tx.execute(
        "INSERT INTO content_packs (pack_id, name, version, tradition, license, attribution, installed_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT(pack_id) DO UPDATE SET
            name = excluded.name,
            version = excluded.version,
            tradition = excluded.tradition,
            license = excluded.license,
            attribution = excluded.attribution,
            installed_at = excluded.installed_at",
        params![
            m.pack_id,
            m.name,
            m.version,
            m.tradition,
            m.license,
            m.attribution,
            now
        ],
    )?;
    tx.commit()?;
    Ok(report)
}

pub fn list_content_packs(conn: &Connection) -> Result<Vec<ContentPackRecord>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT pack_id, name, version, tradition, license, attribution
         FROM content_packs ORDER BY name COLLATE NOCASE ASC",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(ContentPackRecord {
            pack_id: r.get(0)?,
            name: r.get(1)?,
            version: r.get(2)?,
            tradition: r.get(3)?,
            license: r.get(4)?,
            attribution: r.get(5)?,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn list_liturgies(conn: &Connection) -> Result<Vec<LiturgyTemplate>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT id, title, tradition, language FROM liturgy_templates ORDER BY title COLLATE NOCASE ASC",
    )?;
    let headers: Vec<(String, String, String, String)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?
        .collect::<Result<Vec<_>, _>>()?;

    let mut line_stmt = conn.prepare(
        "SELECT speaker, text, secondary_text FROM liturgy_lines
         WHERE template_id = ?1 ORDER BY position ASC",
    )?;
    let mut out = Vec::new();
    for (id, title, tradition, language) in headers {
        let lines = line_stmt
            .query_map([&id], |r| {
                let spk: String = r.get(0)?;
                Ok(LiturgyLine {
                    speaker: speaker_from_db(&spk),
                    text: r.get(1)?,
                    secondary_text: r.get(2)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        out.push(LiturgyTemplate {
            id,
            title,
            tradition,
            language,
            lines,
        });
    }
    Ok(out)
}

const fn speaker_to_db(speaker: LiturgySpeaker) -> &'static str {
    match speaker {
        LiturgySpeaker::Leader => "leader",
        LiturgySpeaker::Congregation => "congregation",
        LiturgySpeaker::All => "all",
        LiturgySpeaker::Cantor => "cantor",
        LiturgySpeaker::Response => "response",
    }
}

fn speaker_from_db(raw: &str) -> LiturgySpeaker {
    match raw {
        "congregation" => LiturgySpeaker::Congregation,
        "all" => LiturgySpeaker::All,
        "cantor" => LiturgySpeaker::Cantor,
        "response" => LiturgySpeaker::Response,
        _ => LiturgySpeaker::Leader,
    }
}
