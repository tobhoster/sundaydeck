use std::path::Path;

use chrono::{DateTime, Utc};
use rusqlite::{Connection, params};
use sundaydeck_core::{CustomSlideSpec, SlideId, SongId, default_custom_slide_templates};

use crate::connection::now_epoch_ms;
use crate::error::DbError;

/// Suppress duplicate CCLI usage rows for the same song projected within 15 minutes
/// (e.g. stepping back and forth through Chorus / Verse during a single worship set).
const CCLI_DEDUPE_WINDOW_MS: i64 = 15 * 60 * 1_000;

/// Input payload for recording a live projected song into the CCLI compliance log.
#[derive(Debug, Clone)]
pub struct CcliProjectionInput {
    pub song_id: Option<SongId>,
    pub title: String,
    pub author: String,
    pub ccli_number: String,
    pub copyright: String,
    pub projected_at: i64,
}

/// Logged live song projection row for monthly CCLI reporting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CcliUsageRecord {
    pub id: i64,
    pub projected_at: i64,
    pub song_id: Option<String>,
    pub title: String,
    pub author: String,
    pub ccli_number: String,
    pub copyright: String,
}

/// Seeds built-in starter custom slides (Welcome, Sermon Points, Pastor Quote, Birthdays, etc.)
/// only when the `custom_slides` table is empty.
pub fn seed_default_custom_slides_if_empty(conn: &Connection) -> Result<bool, DbError> {
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM custom_slides", [], |row| row.get(0))?;
    if count > 0 {
        return Ok(false);
    }
    let templates = default_custom_slide_templates();
    let base_ts = now_epoch_ms();
    for (idx, spec) in templates.iter().enumerate() {
        let spec_json = serde_json::to_string(spec)?;
        let ts = base_ts.saturating_sub(i64::try_from(idx).unwrap_or(0) * 1_000);
        conn.execute(
            "INSERT OR IGNORE INTO custom_slides (id, label, spec_json, updated_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![spec.id.to_string(), spec.label, spec_json, ts],
        )?;
    }
    Ok(true)
}

/// Loads all saved reusable Custom Slides newest-first, skipping any corrupt JSON row.
pub fn list_custom_slides(conn: &Connection) -> Result<Vec<CustomSlideSpec>, DbError> {
    let mut stmt = conn
        .prepare("SELECT id, spec_json FROM custom_slides ORDER BY updated_at DESC, label ASC")?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    let mut out = Vec::new();
    for row in rows {
        let (id_str, raw_json) = row?;
        match serde_json::from_str::<CustomSlideSpec>(&raw_json) {
            Ok(spec) => out.push(spec),
            Err(err) => {
                tracing::warn!(slide_id = %id_str, %err, "skipping unreadable custom_slides row");
            }
        }
    }
    Ok(out)
}

/// Persists (insert or update) a reusable Custom Slide into SQLite.
pub fn upsert_custom_slide(conn: &Connection, spec: &CustomSlideSpec) -> Result<(), DbError> {
    let spec_json = serde_json::to_string(spec)?;
    conn.execute(
        "INSERT INTO custom_slides (id, label, spec_json, updated_at)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(id) DO UPDATE SET
             label = excluded.label,
             spec_json = excluded.spec_json,
             updated_at = excluded.updated_at",
        params![spec.id.to_string(), spec.label, spec_json, now_epoch_ms()],
    )?;
    Ok(())
}

/// Removes a custom slide from the reusable SQLite library.
pub fn delete_custom_slide(conn: &Connection, id: SlideId) -> Result<(), DbError> {
    conn.execute(
        "DELETE FROM custom_slides WHERE id = ?1",
        params![id.to_string()],
    )?;
    Ok(())
}

/// Records a live projected song in `ccli_usage_log`, deduplicating repeated triggers of the same
/// song within 15 minutes. Returns `Ok(true)` if a new row was written.
pub fn record_ccli_projection(
    conn: &Connection,
    entry: &CcliProjectionInput,
) -> Result<bool, DbError> {
    let clean_title = entry.title.trim();
    if clean_title.is_empty() {
        return Ok(false);
    }
    let cutoff = entry.projected_at.saturating_sub(CCLI_DEDUPE_WINDOW_MS);
    let song_id_str = entry.song_id.map(|id| id.to_string());
    let recent_exists: bool = if let Some(ref sid) = song_id_str {
        conn.query_row(
            "SELECT EXISTS(
                SELECT 1 FROM ccli_usage_log
                WHERE projected_at >= ?1 AND (song_id = ?2 OR lower(title) = lower(?3))
             )",
            params![cutoff, sid, clean_title],
            |row| row.get(0),
        )?
    } else {
        conn.query_row(
            "SELECT EXISTS(
                SELECT 1 FROM ccli_usage_log
                WHERE projected_at >= ?1 AND lower(title) = lower(?2)
             )",
            params![cutoff, clean_title],
            |row| row.get(0),
        )?
    };
    if recent_exists {
        return Ok(false);
    }

    conn.execute(
        "INSERT INTO ccli_usage_log (projected_at, song_id, title, author, ccli_number, copyright)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            entry.projected_at,
            song_id_str,
            clean_title,
            entry.author.trim(),
            entry.ccli_number.trim(),
            entry.copyright.trim(),
        ],
    )?;
    Ok(true)
}

pub fn count_ccli_projections(conn: &Connection) -> Result<usize, DbError> {
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM ccli_usage_log", [], |row| row.get(0))?;
    Ok(usize::try_from(count).unwrap_or(0))
}

pub fn list_ccli_usage(conn: &Connection, limit: usize) -> Result<Vec<CcliUsageRecord>, DbError> {
    let cap = i64::try_from(limit.max(1)).unwrap_or(500);
    let mut stmt = conn.prepare(
        "SELECT id, projected_at, song_id, title, author, ccli_number, copyright
         FROM ccli_usage_log
         ORDER BY projected_at DESC, id DESC
         LIMIT ?1",
    )?;
    let rows = stmt.query_map(params![cap], |row| {
        Ok(CcliUsageRecord {
            id: row.get(0)?,
            projected_at: row.get(1)?,
            song_id: row.get(2)?,
            title: row.get(3)?,
            author: row.get(4)?,
            ccli_number: row.get(5)?,
            copyright: row.get(6)?,
        })
    })?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

/// Exports the full CCLI projection history to an RFC-4180 CSV file (safe against spreadsheet
/// formula injection) via atomic write-and-rename. Returns the number of exported rows.
pub fn export_ccli_csv(conn: &Connection, dest: &Path) -> Result<usize, DbError> {
    let records = list_ccli_usage(conn, 50_000)?;
    let mut csv = String::from("projected_at_utc,title,author,ccli_number,copyright\n");
    for rec in &records {
        let iso = format_epoch_ms_utc(rec.projected_at);
        csv.push_str(&csv_cell(&iso));
        csv.push(',');
        csv.push_str(&csv_cell(&rec.title));
        csv.push(',');
        csv.push_str(&csv_cell(&rec.author));
        csv.push(',');
        csv.push_str(&csv_cell(&rec.ccli_number));
        csv.push(',');
        csv.push_str(&csv_cell(&rec.copyright));
        csv.push('\n');
    }
    if let Some(parent) = dest.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = dest.with_extension("csv.tmp");
    std::fs::write(&tmp, csv.as_bytes())?;
    std::fs::rename(&tmp, dest)?;
    Ok(records.len())
}

/// Creates an online, transactionally consistent SQLite snapshot using `VACUUM INTO` written to a
/// sibling temp path first and atomically renamed over `dest`. Returns the backup file size in bytes.
pub fn create_safe_backup(conn: &Connection, dest: &Path) -> Result<u64, DbError> {
    if let Some(parent) = dest.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = dest.with_extension("bak.tmp");
    if tmp.exists() {
        let _ = std::fs::remove_file(&tmp);
    }
    let tmp_str = tmp.to_str().ok_or_else(|| {
        DbError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "backup path contains non-UTF-8 characters",
        ))
    })?;
    conn.execute("VACUUM INTO ?1", params![tmp_str])?;
    std::fs::rename(&tmp, dest)?;
    let bytes = std::fs::metadata(dest)?.len();
    Ok(bytes)
}

fn format_epoch_ms_utc(epoch_ms: i64) -> String {
    DateTime::<Utc>::from_timestamp_millis(epoch_ms)
        .map(|dt| dt.format("%Y-%m-%dT%H:%M:%SZ").to_string())
        .unwrap_or_else(|| epoch_ms.to_string())
}

fn csv_cell(raw: &str) -> String {
    let cleaned = raw.replace(['\r', '\n'], " ");
    let safe = if cleaned.starts_with(['=', '+', '-', '@', '\t']) {
        format!("'{cleaned}")
    } else {
        cleaned
    };
    let escaped = safe.replace('"', "\"\"");
    format!("\"{escaped}\"")
}
