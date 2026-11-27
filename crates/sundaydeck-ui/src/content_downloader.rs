use std::io::Read;
use std::time::Duration;

use gpui::{App, Entity};
use sundaydeck_core::{BibleTranslation, ParsedBible, import_scrollmapper_json};
use sundaydeck_db::DbWorker;

use crate::store::PresentationStore;
use crate::updater::CURRENT_VERSION;

const SCROLLMAPPER_JSON_BASE: &str =
    "https://raw.githubusercontent.com/scrollmapper/bible_databases/master/formats/json/";
const MAX_REMOTE_BIBLE_BYTES: u64 = 16 * 1024 * 1024;
const DOWNLOAD_TIMEOUT_SECS: u64 = 20;

/// Verified Public Domain / CC0 full Bible available for 1-click or automatic offline download.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FreeBibleCatalogEntry {
    pub code: &'static str,
    pub name: &'static str,
    pub language: &'static str,
    pub language_label: &'static str,
    pub license: &'static str,
    pub books_label: &'static str,
    pub scrollmapper_file: &'static str,
    pub auto_sync_default: bool,
}

pub const FREE_BIBLE_CATALOG: &[FreeBibleCatalogEntry] = &[
    FreeBibleCatalogEntry {
        code: "BSB",
        name: "Berean Standard Bible",
        language: "en",
        language_label: "English",
        license: "Public Domain (CC0)",
        books_label: "66 books · Modern English",
        scrollmapper_file: "BSB.json",
        auto_sync_default: true,
    },
    FreeBibleCatalogEntry {
        code: "KJV",
        name: "King James Version",
        language: "en",
        language_label: "English",
        license: "Public Domain",
        books_label: "66 books · Classic English",
        scrollmapper_file: "KJV.json",
        auto_sync_default: true,
    },
    FreeBibleCatalogEntry {
        code: "ASV",
        name: "American Standard Version",
        language: "en",
        language_label: "English",
        license: "Public Domain",
        books_label: "66 books · Literal English",
        scrollmapper_file: "ASV.json",
        auto_sync_default: false,
    },
    FreeBibleCatalogEntry {
        code: "CPDV",
        name: "Catholic Public Domain Version",
        language: "en",
        language_label: "English",
        license: "Public Domain",
        books_label: "73 books · Deuterocanon",
        scrollmapper_file: "CPDV.json",
        auto_sync_default: false,
    },
    FreeBibleCatalogEntry {
        code: "DRC",
        name: "Douay-Rheims Challoner",
        language: "en",
        language_label: "English",
        license: "Public Domain",
        books_label: "73 books · Classic Catholic",
        scrollmapper_file: "DRC.json",
        auto_sync_default: false,
    },
    FreeBibleCatalogEntry {
        code: "RV1909",
        name: "Reina-Valera 1909",
        language: "es",
        language_label: "Español",
        license: "Public Domain",
        books_label: "66 books · Español",
        scrollmapper_file: "SpaRV.json",
        auto_sync_default: false,
    },
    FreeBibleCatalogEntry {
        code: "BLIVRE",
        name: "Bíblia Livre",
        language: "pt",
        language_label: "Português",
        license: "Public Domain (CC0)",
        books_label: "66 books · Português",
        scrollmapper_file: "PorBLivre.json",
        auto_sync_default: false,
    },
    FreeBibleCatalogEntry {
        code: "SYN1921",
        name: "Bible Synodale 1921",
        language: "fr",
        language_label: "Français",
        license: "Public Domain",
        books_label: "66 books · Français",
        scrollmapper_file: "FreSynodale1921.json",
        auto_sync_default: false,
    },
    FreeBibleCatalogEntry {
        code: "CRAMPON",
        name: "Bible Crampon 1923",
        language: "fr",
        language_label: "Français",
        license: "Public Domain",
        books_label: "73 books · Catholic Français",
        scrollmapper_file: "FreCrampon.json",
        auto_sync_default: false,
    },
];

pub fn catalog_entry_by_code(code: &str) -> Option<&'static FreeBibleCatalogEntry> {
    FREE_BIBLE_CATALOG
        .iter()
        .find(|entry| entry.code.eq_ignore_ascii_case(code))
}

/// Returns `true` when `code` is already installed as a full offline translation
/// (not just the tiny built-in `(Starter)` excerpt).
pub fn is_translation_full_offline(translations: &[BibleTranslation], code: &str) -> bool {
    translations.iter().any(|tr| {
        tr.code.eq_ignore_ascii_case(code) && !tr.name.to_ascii_lowercase().contains("(starter)")
    })
}

/// Returns default free Bibles (`BSB`, `KJV`) that are still only starter excerpts and
/// should be hydrated in the background when online.
pub fn missing_auto_sync_bibles(
    translations: &[BibleTranslation],
) -> Vec<&'static FreeBibleCatalogEntry> {
    if std::env::var("SUNDAYDECK_DISABLE_AUTO_SYNC")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
    {
        return Vec::new();
    }
    FREE_BIBLE_CATALOG
        .iter()
        .filter(|entry| {
            entry.auto_sync_default && !is_translation_full_offline(translations, entry.code)
        })
        .collect()
}

/// Downloads and parses a verified Public Domain / CC0 full Bible over HTTPS with a
/// 16 MiB body cap and 20s timeout. Always run on a background executor.
pub fn download_free_bible_blocking(entry: &FreeBibleCatalogEntry) -> Result<ParsedBible, String> {
    let url = format!("{SCROLLMAPPER_JSON_BASE}{}", entry.scrollmapper_file);
    if !url.starts_with(SCROLLMAPPER_JSON_BASE) || entry.scrollmapper_file.contains("..") {
        return Err("Refused untrusted Bible download URL.".to_owned());
    }

    let config = ureq::config::Config::builder()
        .timeout_global(Some(Duration::from_secs(DOWNLOAD_TIMEOUT_SECS)))
        .build();
    let agent = ureq::Agent::new_with_config(config);
    let user_agent = format!("SundayDeck/{CURRENT_VERSION}");

    let response = agent
        .get(&url)
        .header("User-Agent", &user_agent)
        .call()
        .map_err(|_| {
            format!(
                "Could not download {} (offline or unreachable). Built-in starter passages remain ready.",
                entry.code
            )
        })?;

    let mut bytes = Vec::new();
    response
        .into_body()
        .into_reader()
        .take(MAX_REMOTE_BIBLE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|err| format!("Failed reading {} download: {err}", entry.code))?;

    if (bytes.len() as u64) > MAX_REMOTE_BIBLE_BYTES {
        return Err(format!(
            "{} download exceeded the 16 MiB safety limit.",
            entry.code
        ));
    }

    let raw = String::from_utf8(bytes)
        .map_err(|_| format!("{} download was not valid UTF-8.", entry.code))?;
    import_scrollmapper_json(entry.code, entry.name, entry.language, entry.license, &raw)
        .map_err(|err| format!("Could not parse {}: {err}", entry.code))
}

/// Silently hydrates full offline `BSB` and `KJV` on first launch when online without blocking UI.
/// Call only from `main.rs` on app startup (never in headless tests).
pub fn spawn_first_run_bible_sync(store: Entity<PresentationStore>, db: DbWorker, cx: &mut App) {
    let pending = {
        let translations = &store.read(cx).bible_translations;
        missing_auto_sync_bibles(translations)
    };
    if pending.is_empty() {
        return;
    }

    let codes: Vec<&'static str> = pending.iter().map(|e| e.code).collect();
    store.update(cx, |s, cx| {
        s.set_status(
            format!(
                "Downloading full offline Bibles ({}) in background…",
                codes.join(" & ")
            ),
            cx,
        );
    });

    cx.spawn(async move |cx| {
        let mut installed_codes: Vec<&'static str> = Vec::new();
        let mut total_verses: usize = 0;

        for entry in pending {
            let parsed_res = cx
                .background_executor()
                .spawn(async move { download_free_bible_blocking(entry) })
                .await;
            let Ok(parsed) = parsed_res else {
                continue;
            };
            let verse_count = parsed.verses.len();
            if let Ok(translation) = db.import_parsed_bible(parsed).await {
                installed_codes.push(entry.code);
                total_verses += verse_count;
                store.update(cx, |s, cx| s.register_synced_bible(translation, cx));
            }
        }

        if !installed_codes.is_empty() {
            store.update(cx, |s, cx| {
                s.set_status(
                    format!(
                        "Ready offline: full {} installed ({} verses)",
                        installed_codes.join(" & "),
                        total_verses
                    ),
                    cx,
                );
            });
        }
    })
    .detach();
}

#[cfg(test)]
mod tests {
    use sundaydeck_core::{BibleTranslation, BibleTranslationId};

    use super::{FREE_BIBLE_CATALOG, is_translation_full_offline, missing_auto_sync_bibles};

    fn fake_tr(code: &str, name: &str) -> BibleTranslation {
        BibleTranslation {
            id: BibleTranslationId::new_v7(),
            code: code.to_owned(),
            name: name.to_owned(),
            language: "en".to_owned(),
            copyright: "Public Domain".to_owned(),
        }
    }

    #[test]
    fn detects_starter_vs_full_translations_for_auto_sync() {
        assert!(FREE_BIBLE_CATALOG.len() >= 7);
        let starter_only = vec![
            fake_tr("BSB", "Berean Standard Bible (Starter)"),
            fake_tr("KJV", "King James Version (Starter)"),
        ];
        assert!(!is_translation_full_offline(&starter_only, "BSB"));
        let missing = missing_auto_sync_bibles(&starter_only);
        assert_eq!(missing.len(), 2);

        let full = vec![
            fake_tr("BSB", "Berean Standard Bible"),
            fake_tr("KJV", "King James Version"),
        ];
        assert!(is_translation_full_offline(&full, "BSB"));
        assert!(missing_auto_sync_bibles(&full).is_empty());
    }
}
