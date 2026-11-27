use std::path::PathBuf;

use gpui::{Context, PathPromptOptions};
use sundaydeck_core::{
    detect_and_import_bible, export_sdeck_bundle, import_sdeck_bundle, import_sdeck_pack,
};

use crate::deck_import::{
    build_sample_announcement_loop, import_presentation_or_loop_files, local_media_root,
};
use crate::views::settings_modal::SettingsModal;

impl SettingsModal {
    pub(crate) fn import_bible_pack(&mut self, cx: &mut Context<Self>) {
        let Some(db) = self.db.clone() else {
            self.error_message = Some("Database worker unavailable for Bible import.".to_owned());
            cx.notify();
            return;
        };
        let rx = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Select Offline Bible (.usfm, .xml OSIS/USFX/Zefania, or .sqlite)".into()),
        });
        self.busy = true;
        self.error_message = None;
        cx.notify();

        let store = self.store.clone();
        cx.spawn(async move |this, cx| {
            let Ok(Ok(Some(paths))) = rx.await else {
                let _ = this.update(cx, |m, cx| {
                    m.busy = false;
                    cx.notify();
                });
                return;
            };
            let Some(path) = paths.into_iter().next() else {
                let _ = this.update(cx, |m, cx| {
                    m.busy = false;
                    cx.notify();
                });
                return;
            };

            let (code, name) = infer_bible_code_and_name(&path);
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|s| s.to_ascii_lowercase())
                .unwrap_or_default();

            let outcome = if matches!(ext.as_str(), "sqlite" | "sqlite3" | "db" | "bblx") {
                db.import_sqlite_bible(path, &code, &name)
                    .await
                    .map_err(|e| e.to_string())
            } else {
                let parsed_res = cx
                    .background_executor()
                    .spawn(async move {
                        let raw = std::fs::read_to_string(&path)
                            .map_err(|e| format!("Could not read Bible file: {e}"))?;
                        let fname = path
                            .file_name()
                            .and_then(|s| s.to_str())
                            .unwrap_or("bible.xml");
                        detect_and_import_bible(&code, &name, fname, &raw)
                            .map_err(|e| e.to_string())
                    })
                    .await;
                match parsed_res {
                    Ok(parsed) => db
                        .import_parsed_bible(parsed)
                        .await
                        .map_err(|e| e.to_string()),
                    Err(err) => Err(err),
                }
            };

            let _ = this.update(cx, |m, cx| {
                m.busy = false;
                match outcome {
                    Ok(translation) => {
                        let msg = format!(
                            "Imported {} ({}) for instant offline Scripture lookup.",
                            translation.name, translation.code
                        );
                        store.update(cx, |s, cx| s.register_imported_bible(translation, cx));
                        m.status_note = Some(msg);
                        m.error_message = None;
                    }
                    Err(err) => {
                        m.error_message = Some(err);
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(crate) fn import_community_pack(&mut self, cx: &mut Context<Self>) {
        let Some(db) = self.db.clone() else {
            self.error_message = Some("Database worker unavailable for pack import.".to_owned());
            cx.notify();
            return;
        };
        let rx = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Select Church Starter or Hymnal Pack (.sdeck-pack)".into()),
        });
        self.busy = true;
        self.error_message = None;
        cx.notify();

        let store = self.store.clone();
        cx.spawn(async move |this, cx| {
            let Ok(Ok(Some(paths))) = rx.await else {
                let _ = this.update(cx, |m, cx| {
                    m.busy = false;
                    cx.notify();
                });
                return;
            };
            let Some(src) = paths.into_iter().next() else {
                let _ = this.update(cx, |m, cx| {
                    m.busy = false;
                    cx.notify();
                });
                return;
            };

            let loaded = cx
                .background_executor()
                .spawn(async move { import_sdeck_pack(&src).map_err(|e| e.to_string()) })
                .await;

            match loaded {
                Ok(pack) => {
                    let pack_name = pack.manifest.name.clone();
                    let songs_clone = pack.songs.clone();
                    let res = db.install_sdeck_pack(pack).await.map_err(|e| e.to_string());
                    let _ = this.update(cx, |m, cx| {
                        m.busy = false;
                        match res {
                            Ok(report) => {
                                store.update(cx, |s, cx| s.merge_imported_songs(songs_clone, cx));
                                m.status_note = Some(format!(
                                    "Installed \"{pack_name}\" ({} Bibles, {} songs, {} liturgies).",
                                    report.bibles_installed,
                                    report.songs_installed,
                                    report.liturgies_installed
                                ));
                                m.error_message = None;
                            }
                            Err(err) => m.error_message = Some(err),
                        }
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

    pub(crate) fn export_sdeck(&mut self, cx: &mut Context<Self>) {
        let suggested = self.store.read(cx).service_filename();
        let default_dir = std::env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."));
        let rx = cx.prompt_for_new_path(&default_dir, Some(&suggested));
        let store = self.store.clone();

        cx.spawn(async move |this, cx| {
            let Ok(Ok(Some(mut dest))) = rx.await else {
                return;
            };
            if dest.extension().is_none() {
                dest.set_extension("sdeck");
            }
            let (service, songs, themes, now_ms) = store.read_with(cx, |s, _| {
                let themes = s.themes.iter().map(|t| (**t).clone()).collect::<Vec<_>>();
                (s.service.clone(), s.songs.clone(), themes, s.now_epoch_ms())
            });

            let res = cx
                .background_executor()
                .spawn(async move {
                    export_sdeck_bundle(&dest, &service, &songs, &themes, now_ms)
                        .map_err(|e| e.to_string())
                })
                .await;

            let _ = this.update(cx, |m, cx| {
                match res {
                    Ok(report) => {
                        let msg = format!(
                            "Exported {} ({} items, {} songs, {} media).",
                            report.output_path.display(),
                            report.item_count,
                            report.song_count,
                            report.embedded_media_count
                        );
                        store.update(cx, |s, cx| s.set_status(msg.clone(), cx));
                        m.status_note = Some(msg);
                        m.error_message = None;
                    }
                    Err(err) => m.error_message = Some(err),
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(crate) fn import_sdeck(&mut self, cx: &mut Context<Self>) {
        let rx = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Select Portable SundayDeck Bundle (.sdeck)".into()),
        });
        let store = self.store.clone();
        let db = self.db.clone();

        cx.spawn(async move |this, cx| {
            let Ok(Ok(Some(paths))) = rx.await else {
                return;
            };
            let Some(src) = paths.into_iter().next() else {
                return;
            };
            let extract_dir = local_media_root()
                .join("imported-bundles")
                .join(sundaydeck_core::ServiceId::new_v7().to_string());

            let res =
                cx.background_executor()
                    .spawn(async move {
                        import_sdeck_bundle(&src, &extract_dir).map_err(|e| e.to_string())
                    })
                    .await;

            match res {
                Ok(bundle) => {
                    let title = bundle.service.title.clone();
                    let items_n = bundle.service.items.len();
                    let (songs, themes, schedule) =
                        store.update(cx, |s, cx| s.apply_imported_sdeck(bundle, cx));
                    if let Some(worker) = db {
                        for song in songs {
                            let _ = worker.upsert_song(song).await;
                        }
                        for theme in themes {
                            let _ = worker.upsert_theme(theme).await;
                        }
                        let _ = worker.save_service(schedule).await;
                    }
                    let _ = this.update(cx, |m, cx| {
                        m.status_note = Some(format!(
                            "Imported service bundle \"{title}\" ({items_n} items)."
                        ));
                        m.error_message = None;
                        cx.notify();
                    });
                }
                Err(err) => {
                    let _ = this.update(cx, |m, cx| {
                        m.error_message = Some(err);
                        cx.notify();
                    });
                }
            }
        })
        .detach();
    }

    pub(crate) fn import_deck_files(&mut self, as_loop: bool, cx: &mut Context<Self>) {
        let prompt = if as_loop {
            "Select Pre-Service Announcement Loop Images or PDF"
        } else {
            "Select PDF or Slide Images (.pdf, .png, .jpg, .webp)"
        };
        let rx = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: true,
            prompt: Some(prompt.into()),
        });
        let interval = self.loop_interval_secs;
        let store = self.store.clone();
        let db = self.db.clone();

        cx.spawn(async move |this, cx| {
            let Ok(Ok(Some(paths))) = rx.await else {
                return;
            };
            let res = cx
                .background_executor()
                .spawn(async move { import_presentation_or_loop_files(&paths, as_loop, interval) })
                .await;

            match res {
                Ok(item) => {
                    let title = item.title.clone();
                    let schedule = store.update(cx, |s, cx| s.add_service_item(item, true, cx));
                    if let Some(worker) = db {
                        let _ = worker.save_service(schedule).await;
                    }
                    let _ = this.update(cx, |m, cx| {
                        m.status_note = Some(format!("Added \"{title}\" to Service Order."));
                        m.error_message = None;
                        cx.notify();
                    });
                }
                Err(err) => {
                    let _ = this.update(cx, |m, cx| {
                        m.error_message = Some(err);
                        cx.notify();
                    });
                }
            }
        })
        .detach();
    }

    pub(crate) fn add_sample_loop(&mut self, cx: &mut Context<Self>) {
        let item = build_sample_announcement_loop(self.loop_interval_secs);
        let schedule = self
            .store
            .update(cx, |s, cx| s.add_service_item(item, true, cx));
        if let Some(db) = self.db.clone() {
            cx.spawn(async move |_, _| {
                let _ = db.save_service(schedule).await;
            })
            .detach();
        }
        self.status_note = Some(format!(
            "Added 3-slide Pre-Service Announcement Loop ({}s interval).",
            self.loop_interval_secs
        ));
        self.error_message = None;
        cx.notify();
    }
}

fn infer_bible_code_and_name(path: &std::path::Path) -> (String, String) {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("BIBLE")
        .trim();
    let letters: String = stem
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .take(6)
        .collect::<String>()
        .to_ascii_uppercase();
    let code = if letters.is_empty() {
        "BIBLE".to_owned()
    } else {
        letters
    };
    let name = stem.replace(['_', '-'], " ");
    (code, name)
}
