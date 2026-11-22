use rusqlite::Connection;
use sundaydeck_core::defaults::{
    default_sample_service, default_themes, sample_amazing_grace, sample_come_thou_fount,
    sample_it_is_well,
};

use crate::error::DbError;
use crate::service_repo::upsert_service;
use crate::settings_repo::{AppSettings, save_settings};
use crate::song_repo::upsert_song;
use crate::theme_repo::upsert_theme;

/// Seeds built-in themes, public-domain hymns, and a sample service on a fresh database.
/// Returns `true` if initial seed rows were written.
pub fn seed_defaults_if_empty(conn: &mut Connection) -> Result<bool, DbError> {
    let theme_count: i64 = conn.query_row("SELECT COUNT(*) FROM themes", [], |row| row.get(0))?;
    let song_count: i64 = conn.query_row("SELECT COUNT(*) FROM songs", [], |row| row.get(0))?;
    if theme_count > 0 || song_count > 0 {
        return Ok(false);
    }

    for theme in default_themes() {
        upsert_theme(conn, &theme)?;
    }

    for song in [
        sample_amazing_grace(),
        sample_come_thou_fount(),
        sample_it_is_well(),
    ] {
        upsert_song(conn, &song)?;
    }

    upsert_service(conn, &default_sample_service())?;
    save_settings(conn, &AppSettings::default())?;
    Ok(true)
}
