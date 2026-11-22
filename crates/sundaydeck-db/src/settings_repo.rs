use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use sundaydeck_core::MediaAssetId;

use crate::connection::now_epoch_ms;
use crate::error::DbError;

const KEY_APP_SETTINGS: &str = "app_settings_v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppSettings {
    pub church_name: String,
    pub logo_asset_id: Option<MediaAssetId>,
    pub direct_click_live: bool,
    pub audience_display_uuid: Option<String>,
    pub stage_display_uuid: Option<String>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            church_name: "Grace Fellowship".to_owned(),
            logo_asset_id: None,
            direct_click_live: false,
            audience_display_uuid: None,
            stage_display_uuid: None,
        }
    }
}

pub fn load_settings(conn: &Connection) -> Result<AppSettings, DbError> {
    let raw: Option<String> = conn
        .query_row(
            "SELECT value_json FROM app_settings WHERE key = ?1",
            [KEY_APP_SETTINGS],
            |row| row.get(0),
        )
        .optional()?;

    match raw {
        Some(json) => Ok(serde_json::from_str(&json)?),
        None => Ok(AppSettings::default()),
    }
}

pub fn save_settings(conn: &Connection, settings: &AppSettings) -> Result<(), DbError> {
    let json = serde_json::to_string(settings)?;
    conn.execute(
        "INSERT INTO app_settings (key, value_json, updated_at)
         VALUES (?1, ?2, ?3)
         ON CONFLICT(key) DO UPDATE SET
             value_json = excluded.value_json,
             updated_at = excluded.updated_at",
        params![KEY_APP_SETTINGS, json, now_epoch_ms()],
    )?;
    Ok(())
}
