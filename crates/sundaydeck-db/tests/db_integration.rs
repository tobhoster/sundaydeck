use sundaydeck_core::defaults::{default_themes, sample_amazing_grace};
use sundaydeck_core::{
    Arrangement, ArrangementId, MediaAssetId, ReflowMode, SectionId, SectionKind, Song, SongId,
    SongSection, ThemeId, ThemeTarget,
};
use sundaydeck_db::media_repo::{compute_sha256_hex, upsert_media_by_hash};
use sundaydeck_db::seed::seed_defaults_if_empty;
use sundaydeck_db::service_repo::{get_service, list_services, reorder_service_items};
use sundaydeck_db::settings_repo::{load_settings, save_settings};
use sundaydeck_db::song_repo::{delete_song, get_song, search_songs, upsert_song};
use sundaydeck_db::theme_repo::{list_themes, upsert_theme};
use sundaydeck_db::{AppSettings, Database, DbWorker, MediaAsset, MediaKind};

#[test]
fn seed_is_idempotent_and_populates_starter_library() {
    let mut db = Database::open_in_memory().expect("open in-memory db");
    assert!(seed_defaults_if_empty(db.conn_mut()).expect("first seed"));
    assert!(!seed_defaults_if_empty(db.conn_mut()).expect("second seed is noop"));

    let themes = list_themes(db.conn()).expect("list themes");
    assert_eq!(themes.len(), 7);

    let songs = search_songs(db.conn(), "").expect("list all songs");
    assert_eq!(songs.len(), 3);

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
    assert_eq!(punctuated.len(), 1);
    assert_eq!(punctuated[0].title, "Amazing Grace");
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
        sections: vec![
            SongSection {
                id: v1,
                kind: SectionKind::Verse,
                label: "Verse 1".to_owned(),
                ordinal: 0,
                body: "Line one\nLine two".to_owned(),
            },
            SongSection {
                id: ch,
                kind: SectionKind::Chorus,
                label: "Chorus".to_owned(),
                ordinal: 1,
                body: "Chorus line".to_owned(),
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
    assert_eq!(boot.themes.len(), 7);
    assert_eq!(boot.loaded_songs.len(), 3);

    let hits = futures_lite_block_on(worker.search_songs("river")).expect("fts search");
    assert_eq!(hits.len(), 1);
    assert!(hits[0].title.contains("It Is Well"));
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
