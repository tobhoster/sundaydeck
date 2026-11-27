use std::path::PathBuf;

use gpui::{AsyncApp, Context, Entity, PathPromptOptions};
use sundaydeck_core::{Song, SongImporter, UniversalSongImporter};
use sundaydeck_db::DbWorker;
use sundaydeck_db::legacy_importers::{import_easyworship_sqlite, import_openlp_sqlite};

use crate::store::PresentationStore;
use crate::views::settings_modal::SettingsModal;

/// Parses one or more song files or SQLite song databases exported from EasyWorship,
/// OpenLP, ProPresenter (.pro6), VideoPsalm, FreeShow, ChordPro, OpenLyrics, SongSelect,
/// or numbered hymnals.
pub fn import_external_song_files(paths: &[PathBuf]) -> Result<(Vec<Song>, String), String> {
    let mut all_songs = Vec::new();
    let mut source_labels = Vec::new();

    for path in paths {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_ascii_lowercase())
            .unwrap_or_default();
        let fname = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("songs.txt");

        if matches!(ext.as_str(), "sqlite" | "sqlite3" | "db") {
            let sibling_words = path.with_file_name("SongWords.db");
            let sibling_words_lower = path.with_file_name("songwords.db");
            let words_path = if sibling_words.is_file() {
                Some(sibling_words)
            } else if sibling_words_lower.is_file() {
                Some(sibling_words_lower)
            } else {
                None
            };

            if let Some(words) = words_path {
                let songs = import_easyworship_sqlite(path, &words)
                    .map_err(|e| format!("EasyWorship import failed for {fname}: {e}"))?;
                all_songs.extend(songs);
                source_labels.push("EasyWorship");
            } else {
                let songs = import_openlp_sqlite(path)
                    .map_err(|e| format!("OpenLP/SQLite song import failed for {fname}: {e}"))?;
                all_songs.extend(songs);
                source_labels.push("OpenLP");
            }
        } else {
            let raw = std::fs::read_to_string(path)
                .map_err(|e| format!("Could not read {fname}: {e}"))?;
            let songs = UniversalSongImporter
                .import_text(fname, &raw)
                .map_err(|e| format!("Could not import {fname}: {e}"))?;
            all_songs.extend(songs);
            source_labels.push("Library File");
        }
    }

    if all_songs.is_empty() {
        return Err("No songs were found in the selected file(s).".to_owned());
    }
    source_labels.sort_unstable();
    source_labels.dedup();
    Ok((all_songs, source_labels.join(" / ")))
}

/// Where a bulk song import lands: the in-memory library and, when present, the SQLite worker.
pub struct ImportedSongsSink<'a> {
    pub store: &'a Entity<PresentationStore>,
    pub db: Option<&'a DbWorker>,
}

/// Merges bulk-imported songs into the library, then persists them on the SQLite worker. A song
/// that fails to save stays in the in-memory library for this session; the failure is logged
/// instead of silently dropped.
pub async fn merge_and_save_imported_songs(
    sink: ImportedSongsSink<'_>,
    songs: Vec<Song>,
    cx: &mut AsyncApp,
) {
    sink.store
        .update(cx, |s, cx| s.merge_imported_songs(songs.clone(), cx));
    if let Some(db) = sink.db {
        for song in songs {
            if let Err(err) = db.upsert_song(song).await {
                tracing::warn!("could not save an imported song: {err}");
            }
        }
    }
}

impl SettingsModal {
    pub(crate) fn migrate_external_songs(&mut self, cx: &mut Context<Self>) {
        let rx = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: true,
            prompt: Some(
                "Select Songs From EasyWorship (Songs.db), OpenLP (songs.sqlite), ProPresenter, FreeShow, VideoPsalm, ChordPro, or SongSelect"
                    .into(),
            ),
        });
        self.busy = true;
        self.error_message = None;
        cx.notify();

        let store = self.store.clone();
        let db = self.db.clone();
        cx.spawn(async move |this, cx| {
            let Ok(Ok(Some(paths))) = rx.await else {
                let _ = this.update(cx, |m, cx| {
                    m.busy = false;
                    cx.notify();
                });
                return;
            };
            if paths.is_empty() {
                let _ = this.update(cx, |m, cx| {
                    m.busy = false;
                    cx.notify();
                });
                return;
            }

            let res = cx
                .background_executor()
                .spawn(async move { import_external_song_files(&paths) })
                .await;

            match res {
                Ok((songs, source)) => {
                    let sink = ImportedSongsSink {
                        store: &store,
                        db: db.as_ref(),
                    };
                    let count = songs.len();
                    merge_and_save_imported_songs(sink, songs, cx).await;
                    let _ = this.update(cx, |m, cx| {
                        m.busy = false;
                        m.status_note = Some(format!(
                            "Imported {count} song(s) from {source} into your offline library."
                        ));
                        m.error_message = None;
                        cx.notify();
                    });
                }
                Err(err) => {
                    let _ = this.update(cx, |m, cx| {
                        m.busy = false;
                        m.error_message = Some(err);
                        cx.notify();
                    });
                }
            }
        })
        .detach();
    }
}
