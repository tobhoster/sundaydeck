use std::path::Path;

use rusqlite::Connection;
use sundaydeck_core::{
    ExtractedDocument, GeneratedServicePlan, build_service_plan, extract_office_file,
    extract_plain_outline,
};

use crate::bible_repo::get_passage;
use crate::error::DbError;
use crate::song_repo::{get_song, list_songs};
use crate::theme_repo::list_themes;

/// Reads a pastor's `.pptx`, `.docx`, or plain-text outline from disk and builds a complete
/// layered SundayDeck service plan resolved against local SQLite songs, themes, and Bibles.
pub fn build_plan_from_file_sync(
    conn: &Connection,
    path: &Path,
    translation_code: &str,
) -> Result<GeneratedServicePlan, DbError> {
    let doc = extract_office_file(path)?;
    build_plan_from_document_sync(conn, &doc, translation_code)
}

/// Parses pasted sermon notes/outline text and builds a layered SundayDeck service plan
/// resolved against local SQLite songs, themes, and Bibles.
pub fn build_plan_from_text_sync(
    conn: &Connection,
    source_name: &str,
    text: &str,
    translation_code: &str,
) -> Result<GeneratedServicePlan, DbError> {
    let doc = extract_plain_outline(source_name, text)?;
    build_plan_from_document_sync(conn, &doc, translation_code)
}

pub fn build_plan_from_document_sync(
    conn: &Connection,
    doc: &ExtractedDocument,
    translation_code: &str,
) -> Result<GeneratedServicePlan, DbError> {
    let themes = list_themes(conn)?;
    let summaries = list_songs(conn)?;
    let mut songs = Vec::with_capacity(summaries.len());
    for summary in &summaries {
        if let Some(full) = get_song(conn, summary.id)? {
            songs.push(full);
        }
    }

    let plan = build_service_plan(doc, &songs, &themes, translation_code, |sref| {
        get_passage(conn, translation_code, sref).unwrap_or_default()
    });
    Ok(plan)
}
