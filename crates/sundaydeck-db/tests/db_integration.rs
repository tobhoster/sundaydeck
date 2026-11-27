use sundaydeck_core::defaults::{
    default_sample_service, default_themes, sample_amazing_grace, sample_come_thou_fount,
};
use sundaydeck_core::starter_hymns::starter_songs;
use sundaydeck_core::{
    Arrangement, ArrangementId, MediaAssetId, ReflowMode, SectionId, SectionKind, Song, SongId,
    SongSection, ThemeId, ThemeTarget,
};
use sundaydeck_db::bible_repo::search_bible_verses;
use sundaydeck_db::media_repo::{compute_sha256_hex, upsert_media_by_hash};
use sundaydeck_db::seed::seed_defaults_if_empty;
use sundaydeck_db::service_repo::{get_service, list_services, reorder_service_items};
use sundaydeck_db::settings_repo::{load_settings, save_settings};
use sundaydeck_db::song_repo::{delete_song, get_song, search_songs, upsert_song};
use sundaydeck_db::theme_repo::{list_themes, upsert_theme};
use sundaydeck_db::worker::load_bootstrap_sync;
use sundaydeck_db::{AppSettings, Database, DbWorker, MediaAsset, MediaKind};

#[test]
fn seed_is_idempotent_and_populates_starter_library() {
    let mut db = Database::open_in_memory().expect("open in-memory db");
    assert!(seed_defaults_if_empty(db.conn_mut()).expect("first seed"));
    assert!(!seed_defaults_if_empty(db.conn_mut()).expect("second seed is noop"));

    let themes = list_themes(db.conn()).expect("list themes");
    assert_eq!(themes.len(), default_themes().len());

    let songs = search_songs(db.conn(), "").expect("list all songs");
    assert_eq!(songs.len(), starter_songs().len());

    let services = list_services(db.conn()).expect("list services");
    assert_eq!(services.len(), 1);
    assert_eq!(services[0].items.len(), 5);
}

#[test]
fn fts5_search_indexes_lyrics_ccli_and_stays_synced_on_update_and_delete() {
    let mut db = Database::open_in_memory().expect("open db");
    seed_defaults_if_empty(db.conn_mut()).expect("seed");

    // Search by lyric body fragment (prefix)
    let by_lyric = search_songs(db.conn(), "ebenez").expect("lyric fts search");
    assert_eq!(by_lyric.len(), 1);
    assert!(by_lyric[0].title.contains("Come, Thou Fount"));

    // Search by CCLI number
    let by_ccli = search_songs(db.conn(), "22025").expect("ccli fts search");
    assert_eq!(by_ccli.len(), 1);
    assert_eq!(by_ccli[0].title, "Amazing Grace");

    // Arbitrary punctuation in volunteer query must never throw an FTS5 syntax error
    let punctuated = search_songs(db.conn(), "\"grace\" (amazing) *!").expect("punctuated search");
    assert!(punctuated.iter().any(|s| s.title == "Amazing Grace"));
    let _operators =
        search_songs(db.conn(), "\"grace\" OR NOT * ()))").expect("operator-safe search");

    // Update lyrics and verify FTS trigger refreshes index
    let mut grace = get_song(db.conn(), sample_amazing_grace().id)
        .expect("get grace")
        .expect("grace exists");
    grace.sections[0].body = "Unique sentinel lyric phrase for FTS update test".to_owned();
    upsert_song(db.conn_mut(), &grace).expect("update grace");

    let updated_hits = search_songs(db.conn(), "sentinel").expect("updated fts search");
    assert_eq!(updated_hits.len(), 1);
    assert_eq!(updated_hits[0].id, grace.id);

    // Delete song and verify FTS trigger clears index
    delete_song(db.conn(), grace.id).expect("delete grace");
    let after_delete = search_songs(db.conn(), "sentinel").expect("post-delete search");
    assert!(after_delete.is_empty());
}

/// (query, song title that must still be found, word that must appear in a WEB verse hit)
const FTS5_HOSTILE_QUERIES: &[(&str, Option<&str>, Option<&str>)] = &[
    (r#"" * ( ) : ^ - ' OR AND NOT NEAR( title: 🙏"#, None, None),
    ("\"", None, None),
    ("\"\"", None, None),
    ("''", None, None),
    ("*", None, None),
    ("(", None, None),
    (")", None, None),
    (":", None, None),
    ("^", None, None),
    ("-", None, None),
    ("OR", None, None),
    ("AND", None, None),
    ("NOT", None, None),
    ("NEAR(", None, None),
    ("title:", None, None),
    ("🙏", None, None),
    ("a OR", None, None),
    ("-a", None, None),
    ("Señor", None, None),
    ("graça", None, None),
    ("", Some("Amazing Grace"), None),
    ("   \t\n ", Some("Amazing Grace"), None),
    ("\"grace", Some("Amazing Grace"), None),
    ("grace\"", Some("Amazing Grace"), None),
    ("(grace)", Some("Amazing Grace"), None),
    ("^grace*", Some("Amazing Grace"), None),
    ("-grace", Some("Amazing Grace"), None),
    ("grace:", Some("Amazing Grace"), None),
    ("grace 🙏", Some("Amazing Grace"), None),
    ("amázing gráce", Some("Amazing Grace"), None),
    ("\"shepherd", None, Some("shepherd")),
    ("(shepherd) 🙏", None, Some("shepherd")),
    ("-shepherd*", None, Some("shepherd")),
    ("shépherd:", None, Some("shepherd")),
];

#[test]
fn fts5_song_and_bible_search_accept_hostile_queries() {
    let mut db = Database::open_in_memory().expect("open db");
    seed_defaults_if_empty(db.conn_mut()).expect("seed");
    let worker = DbWorker::spawn_in_memory().expect("spawn worker");

    for &(query, song, verse_word) in FTS5_HOSTILE_QUERIES {
        let direct = search_songs(db.conn(), query)
            .unwrap_or_else(|err| panic!("search_songs({query:?}): {err}"));
        let via_worker = futures_lite_block_on(worker.search_songs(query))
            .unwrap_or_else(|err| panic!("DbWorker::search_songs({query:?}): {err}"));
        let verses = search_bible_verses(db.conn(), "WEB", query, 20)
            .unwrap_or_else(|err| panic!("search_bible_verses({query:?}): {err}"));
        let verses_via_worker = futures_lite_block_on(worker.search_bible_verses("WEB", query, 20))
            .unwrap_or_else(|err| panic!("DbWorker::search_bible_verses({query:?}): {err}"));

        if let Some(title) = song {
            for (path, hits) in [("direct", &direct), ("worker", &via_worker)] {
                assert!(
                    hits.iter().any(|s| s.title == title),
                    "{path} song search {query:?} must still find {title:?}"
                );
            }
        }
        if let Some(word) = verse_word {
            for (path, hits) in [("direct", &verses), ("worker", &verses_via_worker)] {
                assert!(
                    hits.iter().any(|v| v.text.to_lowercase().contains(word)),
                    "{path} verse search {query:?} must still find {word:?}"
                );
            }
        }
    }
}

#[test]
fn non_destructive_arrangement_preserves_repeated_section_order() {
    let mut db = Database::open_in_memory().expect("open db");
    let v1 = SectionId::new_v7();
    let ch = SectionId::new_v7();
    let arr_id = ArrangementId::new_v7();

    let song = Song {
        id: SongId::new_v7(),
        title: "Repeat Chorus Test".to_owned(),
        alternate_title: String::new(),
        author: "Test Author".to_owned(),
        copyright: String::new(),
        ccli_number: "99999".to_owned(),
        default_theme_id: None,
        default_arrangement_id: Some(arr_id),
        default_reflow_mode: ReflowMode::Lines2,
        primary_lang: "en".to_owned(),
        secondary_lang: None,
        hymnal_entries: Vec::new(),
        traditions: Vec::new(),
        sections: vec![
            SongSection {
                id: v1,
                kind: SectionKind::Verse,
                label: "Verse 1".to_owned(),
                ordinal: 0,
                body: "Line one\nLine two".to_owned(),
                secondary_body: None,
                stage_chords: None,
            },
            SongSection {
                id: ch,
                kind: SectionKind::Chorus,
                label: "Chorus".to_owned(),
                ordinal: 1,
                body: "Chorus line".to_owned(),
                secondary_body: None,
                stage_chords: None,
            },
        ],
        arrangements: vec![Arrangement {
            id: arr_id,
            name: "Sunday Extended".to_owned(),
            section_ids: vec![v1, ch, v1, ch, ch],
        }],
    };

    upsert_song(db.conn_mut(), &song).expect("insert song");
    let loaded = get_song(db.conn(), song.id)
        .expect("get song")
        .expect("song exists");

    assert_eq!(loaded.sections.len(), 2);
    assert_eq!(loaded.arrangements[0].section_ids, vec![v1, ch, v1, ch, ch]);
}

#[test]
fn single_default_theme_per_target_automatically_demotes_previous_default() {
    let mut db = Database::open_in_memory().expect("open db");
    let mut themes = default_themes();
    let mut first_song_theme = themes.remove(0);
    first_song_theme.target = ThemeTarget::Song;
    first_song_theme.is_default = true;
    upsert_theme(db.conn_mut(), &first_song_theme).expect("insert first default");

    let mut replacement = first_song_theme.clone();
    replacement.id = ThemeId::new_v7();
    replacement.name = "Alternate Song Default".to_owned();
    replacement.is_default = true;
    upsert_theme(db.conn_mut(), &replacement).expect("insert replacement default");

    let all = list_themes(db.conn()).expect("list themes");
    let song_defaults: Vec<_> = all
        .iter()
        .filter(|t| t.target == ThemeTarget::Song && t.is_default)
        .collect();
    assert_eq!(song_defaults.len(), 1);
    assert_eq!(song_defaults[0].id, replacement.id);
}

#[test]
fn service_item_reorder_updates_positions_atomically() {
    let mut db = Database::open_in_memory().expect("open db");
    seed_defaults_if_empty(db.conn_mut()).expect("seed");
    let service = list_services(db.conn()).expect("list").remove(0);
    let mut reversed: Vec<_> = service.items.iter().map(|i| i.id).collect();
    reversed.reverse();

    reorder_service_items(db.conn_mut(), service.id, &reversed).expect("reorder");
    let reloaded = get_service(db.conn(), service.id)
        .expect("reload")
        .expect("exists");
    let reloaded_ids: Vec<_> = reloaded.items.iter().map(|i| i.id).collect();
    assert_eq!(reloaded_ids, reversed);
}

#[test]
fn media_assets_deduplicate_by_sha256_and_settings_roundtrip() {
    let db = Database::open_in_memory().expect("open db");
    let hash = compute_sha256_hex(b"sample-logo-bytes");
    let first = MediaAsset {
        id: MediaAssetId::new_v7(),
        kind: MediaKind::Logo,
        name: "church-logo.png".to_owned(),
        sha256: hash.clone(),
        relative_path: format!("media/{hash}.png"),
        mime_type: "image/png".to_owned(),
        width: Some(1920),
        height: Some(1080),
        byte_size: 17,
    };
    let mut dup = first.clone();
    dup.id = MediaAssetId::new_v7();

    let saved_a = upsert_media_by_hash(db.conn(), &first).expect("insert first");
    let saved_b = upsert_media_by_hash(db.conn(), &dup).expect("dedup second");
    assert_eq!(saved_a.id, saved_b.id);

    let settings = AppSettings {
        church_name: "River Valley Church".to_owned(),
        logo_asset_id: Some(saved_a.id),
        logo_uri: Some("/tmp/river-valley-logo.png".to_owned()),
        direct_click_live: true,
        audience_display_uuid: Some("display-2".to_owned()),
        stage_display_uuid: None,
        motion_mode: "crossfade".to_owned(),
        voice_advance_threshold_pct: 75,
        ..AppSettings::default()
    };
    save_settings(db.conn(), &settings).expect("save settings");
    let loaded = load_settings(db.conn()).expect("load settings");
    assert_eq!(loaded, settings);
}

#[test]
fn async_db_worker_loads_bootstrap_and_searches_off_thread() {
    let worker = DbWorker::spawn_in_memory().expect("spawn worker");
    let boot = futures_lite_block_on(worker.load_bootstrap()).expect("bootstrap");
    assert_eq!(boot.themes.len(), default_themes().len());
    assert_eq!(boot.loaded_songs.len(), starter_songs().len());

    let hits = futures_lite_block_on(worker.search_songs("river")).expect("fts search");
    assert_eq!(hits.len(), 1);
    assert!(hits[0].title.contains("It Is Well"));
}

#[derive(Debug, Clone, Copy)]
enum Corruption {
    SongSectionKind,
    ServiceItemPayload,
    SettingsJson,
}

/// Damages one row behind the repo's back, the way a crashed write or a hand-edited DB would.
fn corrupt(db: &Database, corruption: Corruption) {
    let conn = db.conn();
    let changed = match corruption {
        Corruption::SongSectionKind => {
            conn.execute_batch("PRAGMA ignore_check_constraints = ON;")
                .expect("disable CHECK");
            let changed = conn
                .execute(
                    "UPDATE song_sections SET kind = 'not_a_kind' WHERE song_id = ?1",
                    [sample_come_thou_fount().id.to_string()],
                )
                .expect("corrupt song section");
            conn.execute_batch("PRAGMA ignore_check_constraints = OFF;")
                .expect("re-enable CHECK");
            changed
        }
        Corruption::ServiceItemPayload => conn
            .execute(
                "UPDATE service_items SET payload_json = '{\"not\":\"a payload\"}' WHERE id = ?1",
                [default_sample_service().items[2].id.to_string()],
            )
            .expect("corrupt service item"),
        Corruption::SettingsJson => conn
            .execute(
                "UPDATE app_settings SET value_json = '{\"church_name\": '",
                [],
            )
            .expect("corrupt settings"),
    };
    assert!(changed > 0, "{corruption:?} must hit a seeded row");
}

#[test]
fn bootstrap_skips_a_corrupt_song_service_item_or_settings_row() {
    use Corruption::{ServiceItemPayload, SettingsJson, SongSectionKind};
    let cases: [&[Corruption]; 4] = [
        &[SongSectionKind],
        &[ServiceItemPayload],
        &[SettingsJson],
        &[SongSectionKind, ServiceItemPayload, SettingsJson],
    ];
    let saved = AppSettings {
        church_name: "St. Bootstrap".to_owned(),
        onboarding_completed: true,
        ..AppSettings::default()
    };
    let bad_song = sample_come_thou_fount().id;
    let bad_item = default_sample_service().items[2].id;
    let mut failures = Vec::new();

    for corruptions in cases {
        let mut db = Database::open_in_memory().expect("open db");
        seed_defaults_if_empty(db.conn_mut()).expect("seed");
        save_settings(db.conn(), &saved).expect("save settings");
        for &corruption in corruptions {
            corrupt(&db, corruption);
        }
        let has = |c: fn(&Corruption) -> bool| corruptions.iter().any(c);

        let boot = match load_bootstrap_sync(&db) {
            Ok(boot) => boot,
            Err(err) => {
                failures.push(format!("{corruptions:?}: {err}"));
                continue;
            }
        };

        let song_ids: Vec<SongId> = boot.loaded_songs.iter().map(|s| s.id).collect();
        let song_corrupt = has(|c| matches!(c, SongSectionKind));
        let expected_songs = starter_songs().len();
        assert_eq!(
            song_ids.len(),
            if song_corrupt {
                expected_songs - 1
            } else {
                expected_songs
            },
            "{corruptions:?}"
        );
        assert_eq!(
            !song_ids.contains(&bad_song),
            song_corrupt,
            "{corruptions:?}"
        );
        assert!(
            song_ids.contains(&sample_amazing_grace().id),
            "{corruptions:?}"
        );

        assert_eq!(boot.services.len(), 1, "{corruptions:?}");
        let items = &boot.services[0].items;
        let item_corrupt = has(|c| matches!(c, ServiceItemPayload));
        assert_eq!(
            items.len(),
            if item_corrupt { 4 } else { 5 },
            "{corruptions:?}"
        );
        assert_eq!(
            !items.iter().any(|i| i.id == bad_item),
            item_corrupt,
            "{corruptions:?}"
        );

        let expected_settings = if has(|c| matches!(c, SettingsJson)) {
            AppSettings::default()
        } else {
            saved.clone()
        };
        assert_eq!(boot.settings, expected_settings, "{corruptions:?}");
        assert_eq!(boot.themes.len(), default_themes().len(), "{corruptions:?}");
    }

    assert!(
        failures.is_empty(),
        "one corrupt row must not abort startup:\n{}",
        failures.join("\n")
    );
}

fn futures_lite_block_on<F: std::future::Future>(fut: F) -> F::Output {
    use std::pin::pin;
    use std::task::{Context, Poll, Wake, Waker};
    struct ThreadWaker(std::thread::Thread);
    impl Wake for ThreadWaker {
        fn wake(self: std::sync::Arc<Self>) {
            self.0.unpark();
        }
    }
    let waker = Waker::from(std::sync::Arc::new(ThreadWaker(std::thread::current())));
    let mut cx = Context::from_waker(&waker);
    let mut pinned = pin!(fut);
    loop {
        match pinned.as_mut().poll(&mut cx) {
            Poll::Ready(val) => return val,
            Poll::Pending => std::thread::park(),
        }
    }
}
