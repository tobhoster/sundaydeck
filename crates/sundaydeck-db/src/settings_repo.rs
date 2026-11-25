use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use sundaydeck_core::MediaAssetId;

use crate::connection::now_epoch_ms;
use crate::error::DbError;

const KEY_APP_SETTINGS: &str = "app_settings_v1";

fn default_motion_mode() -> String {
    "full".to_owned()
}
fn default_local_llm_url() -> String {
    "http://127.0.0.1:8080/v1/chat/completions".to_owned()
}
fn default_local_llm_model() -> String {
    "qwen3-4b".to_owned()
}
fn default_whisper_url() -> String {
    "http://127.0.0.1:8081/inference".to_owned()
}
const fn default_voice_threshold_pct() -> u8 {
    70
}
const fn default_voice_lock_during_override() -> bool {
    true
}
fn default_church_profile() -> String {
    "contemporary".to_owned()
}
fn default_bilingual_mode() -> String {
    "bilingual_stacked".to_owned()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppSettings {
    pub church_name: String,
    pub logo_asset_id: Option<MediaAssetId>,
    #[serde(default)]
    pub logo_uri: Option<String>,
    pub direct_click_live: bool,
    pub audience_display_uuid: Option<String>,
    pub stage_display_uuid: Option<String>,
    #[serde(default = "default_motion_mode")]
    pub motion_mode: String,
    #[serde(default = "default_local_llm_url")]
    pub local_llm_url: String,
    #[serde(default = "default_local_llm_model")]
    pub local_llm_model: String,
    #[serde(default = "default_whisper_url")]
    pub whisper_url: String,
    #[serde(default = "default_voice_threshold_pct")]
    pub voice_advance_threshold_pct: u8,
    #[serde(default = "default_voice_lock_during_override")]
    pub voice_lock_during_override: bool,
    #[serde(default = "default_church_profile")]
    pub church_profile: String,
    #[serde(default = "default_bilingual_mode")]
    pub bilingual_mode: String,
    #[serde(default)]
    pub enable_catholic_canon: bool,
    #[serde(default)]
    pub onboarding_completed: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            church_name: "Grace Fellowship".to_owned(),
            logo_asset_id: None,
            logo_uri: None,
            direct_click_live: false,
            audience_display_uuid: None,
            stage_display_uuid: None,
            motion_mode: default_motion_mode(),
            local_llm_url: default_local_llm_url(),
            local_llm_model: default_local_llm_model(),
            whisper_url: default_whisper_url(),
            voice_advance_threshold_pct: default_voice_threshold_pct(),
            voice_lock_during_override: default_voice_lock_during_override(),
            church_profile: default_church_profile(),
            bilingual_mode: default_bilingual_mode(),
            enable_catholic_canon: false,
            onboarding_completed: false,
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
