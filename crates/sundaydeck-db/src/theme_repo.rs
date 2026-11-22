use rusqlite::{Connection, OptionalExtension, Row, params};
use sundaydeck_core::{
    BackgroundSpec, HorizontalAlign, Rgba8, SafeMargins, TextShadow, Theme, ThemeId, ThemeTarget,
    VerticalAlign,
};

use crate::connection::now_epoch_ms;
use crate::error::DbError;

pub fn list_themes(conn: &Connection) -> Result<Vec<Theme>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT id, name, target, is_default, background_json, font_family, font_size_pt,
                font_weight, line_height, text_color_rgba, h_align, v_align,
                stroke_width_px, stroke_color_rgba, shadow_json,
                margin_top_pct, margin_right_pct, margin_bottom_pct, margin_left_pct,
                footer_font_size_pt, transition_ms
         FROM themes
         ORDER BY is_default DESC, name COLLATE NOCASE ASC",
    )?;

    let rows = stmt.query_map([], map_theme_row)?;
    let mut themes = Vec::new();
    for row in rows {
        themes.push(row??);
    }
    Ok(themes)
}

pub fn get_theme(conn: &Connection, id: ThemeId) -> Result<Option<Theme>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT id, name, target, is_default, background_json, font_family, font_size_pt,
                font_weight, line_height, text_color_rgba, h_align, v_align,
                stroke_width_px, stroke_color_rgba, shadow_json,
                margin_top_pct, margin_right_pct, margin_bottom_pct, margin_left_pct,
                footer_font_size_pt, transition_ms
         FROM themes
         WHERE id = ?1",
    )?;
    match stmt.query_row([id.to_string()], map_theme_row).optional()? {
        Some(parsed) => Ok(Some(parsed?)),
        None => Ok(None),
    }
}

pub fn upsert_theme(conn: &mut Connection, theme: &Theme) -> Result<(), DbError> {
    theme.validate()?;
    let now = now_epoch_ms();
    let background_json = serde_json::to_string(&theme.background)?;
    let shadow_json = theme
        .shadow
        .as_ref()
        .map(serde_json::to_string)
        .transpose()?;

    let tx = conn.transaction()?;
    if theme.is_default {
        tx.execute(
            "UPDATE themes SET is_default = 0, updated_at = ?1
             WHERE target = ?2 AND id <> ?3 AND is_default = 1",
            params![now, theme.target.as_db_str(), theme.id.to_string()],
        )?;
    }

    tx.execute(
        "INSERT INTO themes (
             id, name, target, is_default, background_kind, background_json,
             font_family, font_size_pt, font_weight, line_height, text_color_rgba,
             h_align, v_align, stroke_width_px, stroke_color_rgba, shadow_json,
             margin_top_pct, margin_right_pct, margin_bottom_pct, margin_left_pct,
             footer_font_size_pt, transition_ms, created_at, updated_at
         ) VALUES (
             ?1, ?2, ?3, ?4, ?5, ?6,
             ?7, ?8, ?9, ?10, ?11,
             ?12, ?13, ?14, ?15, ?16,
             ?17, ?18, ?19, ?20,
             ?21, ?22, ?23, ?24
         )
         ON CONFLICT(id) DO UPDATE SET
             name = excluded.name,
             target = excluded.target,
             is_default = excluded.is_default,
             background_kind = excluded.background_kind,
             background_json = excluded.background_json,
             font_family = excluded.font_family,
             font_size_pt = excluded.font_size_pt,
             font_weight = excluded.font_weight,
             line_height = excluded.line_height,
             text_color_rgba = excluded.text_color_rgba,
             h_align = excluded.h_align,
             v_align = excluded.v_align,
             stroke_width_px = excluded.stroke_width_px,
             stroke_color_rgba = excluded.stroke_color_rgba,
             shadow_json = excluded.shadow_json,
             margin_top_pct = excluded.margin_top_pct,
             margin_right_pct = excluded.margin_right_pct,
             margin_bottom_pct = excluded.margin_bottom_pct,
             margin_left_pct = excluded.margin_left_pct,
             footer_font_size_pt = excluded.footer_font_size_pt,
             transition_ms = excluded.transition_ms,
             updated_at = excluded.updated_at",
        params![
            theme.id.to_string(),
            theme.name,
            theme.target.as_db_str(),
            i64::from(theme.is_default),
            theme.background.db_kind(),
            background_json,
            theme.font_family,
            f64::from(theme.font_size_pt),
            i64::from(theme.font_weight),
            f64::from(theme.line_height),
            i64::from(theme.text_color.as_u32()),
            theme.h_align.as_db_str(),
            theme.v_align.as_db_str(),
            f64::from(theme.stroke_width_px),
            i64::from(theme.stroke_color.as_u32()),
            shadow_json,
            f64::from(theme.margins.top_pct),
            f64::from(theme.margins.right_pct),
            f64::from(theme.margins.bottom_pct),
            f64::from(theme.margins.left_pct),
            f64::from(theme.footer_font_size_pt),
            i64::from(theme.transition_ms),
            now,
            now,
        ],
    )?;
    tx.commit()?;
    Ok(())
}

pub fn delete_theme(conn: &Connection, id: ThemeId) -> Result<(), DbError> {
    conn.execute("DELETE FROM themes WHERE id = ?1", [id.to_string()])?;
    Ok(())
}

fn map_theme_row(row: &Row<'_>) -> rusqlite::Result<Result<Theme, DbError>> {
    let id_str: String = row.get(0)?;
    let name: String = row.get(1)?;
    let target_str: String = row.get(2)?;
    let is_default_int: i64 = row.get(3)?;
    let background_json: String = row.get(4)?;
    let font_family: String = row.get(5)?;
    let font_size_pt: f64 = row.get(6)?;
    let font_weight: i64 = row.get(7)?;
    let line_height: f64 = row.get(8)?;
    let text_color_i64: i64 = row.get(9)?;
    let h_align_str: String = row.get(10)?;
    let v_align_str: String = row.get(11)?;
    let stroke_width_px: f64 = row.get(12)?;
    let stroke_color_i64: i64 = row.get(13)?;
    let shadow_json: Option<String> = row.get(14)?;
    let margin_top_pct: f64 = row.get(15)?;
    let margin_right_pct: f64 = row.get(16)?;
    let margin_bottom_pct: f64 = row.get(17)?;
    let margin_left_pct: f64 = row.get(18)?;
    let footer_font_size_pt: f64 = row.get(19)?;
    let transition_ms: i64 = row.get(20)?;

    Ok(parse_theme_fields(RawThemeRow {
        id_str,
        name,
        target_str,
        is_default_int,
        background_json,
        font_family,
        font_size_pt,
        font_weight,
        line_height,
        text_color_i64,
        h_align_str,
        v_align_str,
        stroke_width_px,
        stroke_color_i64,
        shadow_json,
        margin_top_pct,
        margin_right_pct,
        margin_bottom_pct,
        margin_left_pct,
        footer_font_size_pt,
        transition_ms,
    }))
}

struct RawThemeRow {
    id_str: String,
    name: String,
    target_str: String,
    is_default_int: i64,
    background_json: String,
    font_family: String,
    font_size_pt: f64,
    font_weight: i64,
    line_height: f64,
    text_color_i64: i64,
    h_align_str: String,
    v_align_str: String,
    stroke_width_px: f64,
    stroke_color_i64: i64,
    shadow_json: Option<String>,
    margin_top_pct: f64,
    margin_right_pct: f64,
    margin_bottom_pct: f64,
    margin_left_pct: f64,
    footer_font_size_pt: f64,
    transition_ms: i64,
}

fn parse_theme_fields(raw: RawThemeRow) -> Result<Theme, DbError> {
    let background: BackgroundSpec = serde_json::from_str(&raw.background_json)?;
    let shadow: Option<TextShadow> = raw
        .shadow_json
        .as_deref()
        .map(serde_json::from_str)
        .transpose()?;

    Ok(Theme {
        id: ThemeId::parse_str(&raw.id_str)?,
        name: raw.name,
        target: ThemeTarget::from_db_str(&raw.target_str)?,
        is_default: raw.is_default_int != 0,
        background,
        font_family: raw.font_family,
        font_size_pt: raw.font_size_pt as f32,
        font_weight: u16::try_from(raw.font_weight).unwrap_or(600),
        line_height: raw.line_height as f32,
        text_color: Rgba8(raw.text_color_i64 as u32),
        h_align: HorizontalAlign::from_db_str(&raw.h_align_str)?,
        v_align: VerticalAlign::from_db_str(&raw.v_align_str)?,
        stroke_width_px: raw.stroke_width_px as f32,
        stroke_color: Rgba8(raw.stroke_color_i64 as u32),
        shadow,
        margins: SafeMargins {
            top_pct: raw.margin_top_pct as f32,
            right_pct: raw.margin_right_pct as f32,
            bottom_pct: raw.margin_bottom_pct as f32,
            left_pct: raw.margin_left_pct as f32,
        },
        footer_font_size_pt: raw.footer_font_size_pt as f32,
        transition_ms: u16::try_from(raw.transition_ms).unwrap_or(250),
    })
}
