use rusqlite::Connection;
use sundaydeck_core::{
    PaginationBudget, detect_and_import_song, import_bible_xml, paginate_scripture,
    parse_scripture_ref,
};
use sundaydeck_db::Database;
use sundaydeck_db::bible_repo::{
    get_chapter_verses, get_passage, import_parsed_bible, import_sqlite_bible, list_translations,
    search_bible_verses,
};
use sundaydeck_db::seed::seed_defaults_if_empty;
use sundaydeck_db::song_repo::{search_songs, upsert_song};

#[test]
fn starter_bibles_support_multi_translation_lookup_fts_and_pagination() {
    let mut db = Database::open_in_memory().expect("open db");
    seed_defaults_if_empty(db.conn_mut()).expect("seed");

    let translations = list_translations(db.conn()).expect("translations");
    let codes: Vec<&str> = translations.iter().map(|t| t.code.as_str()).collect();
    assert!(codes.contains(&"WEB"));
    assert!(codes.contains(&"KJV"));

    // Smart reference parsing + passage lookup across two translations
    let r = parse_scripture_ref("1cor 13:4-7").expect("parse 1cor");
    let web_verses = get_passage(db.conn(), "WEB", &r).expect("web passage");
    let kjv_verses = get_passage(db.conn(), "KJV", &r).expect("kjv passage");
    assert_eq!(web_verses.len(), 4);
    assert_eq!(kjv_verses.len(), 4);
    assert!(web_verses[0].text.contains("Love is patient"));
    assert!(kjv_verses[0].text.contains("Charity suffereth long"));

    // Auto-pagination produces clean slides with citation footers
    let slides = paginate_scripture(&web_verses, PaginationBudget::default());
    assert!(slides.len() >= 2);
    assert_eq!(slides[0].label, "1 Corinthians 13:4-5");

    // FTS5 keyword search scoped by translation
    let web_hits = search_bible_verses(db.conn(), "WEB", "shepherd", 20).expect("fts web");
    assert!(!web_hits.is_empty());
    assert!(web_hits.iter().all(|v| v.translation_code == "WEB"));

    // Continuous chapter scroll
    let psalm23 = get_chapter_verses(db.conn(), "WEB", 19, 23).expect("chapter scroll");
    assert_eq!(psalm23.len(), 6);
}

#[test]
fn imports_openlyrics_song_and_external_xml_and_sqlite_bibles() {
    let mut db = Database::open_in_memory().expect("open db");

    // OpenLyrics song -> DB -> FTS5
    let openlyrics = r#"<song><properties><titles><title>Oceans of Grace</title></titles>
        <authors><author>Test Writer</author></authors><ccliNo>77777</ccliNo>
        <verseOrder>v1 c1</verseOrder></properties>
        <lyrics><verse name="v1"><lines>Deep calls unto deep</lines></verse>
        <verse name="c1"><lines>Your mercy overflows</lines></verse></lyrics></song>"#;
    let song = detect_and_import_song("oceans.xml", openlyrics).expect("import openlyrics");
    upsert_song(db.conn_mut(), &song).expect("persist song");
    let hits = search_songs(db.conn(), "overflows").expect("fts imported song");
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].ccli_number, "77777");

    // Zefania XML Bible import -> DB -> FTS5
    let zefania = r#"<XMLBIBLE biblename="ASV Sample"><BIBLEBOOK bnumber="43"><CHAPTER cnumber="3">
        <VERS vnumber="16">For God so loved the world, that he gave his only begotten Son.</VERS>
    </CHAPTER></BIBLEBOOK></XMLBIBLE>"#;
    let parsed = import_bible_xml("ASV", "ASV Sample", zefania).expect("parse zefania");
    import_parsed_bible(db.conn_mut(), &parsed).expect("store zefania");
    let asv_hits = search_bible_verses(db.conn(), "ASV", "begotten", 10).expect("asv fts");
    assert_eq!(asv_hits.len(), 1);
    assert_eq!(asv_hits[0].verse, 16);

    // External open SQLite Bible database import (Scrollmapper schema `t_ylt`)
    let tmp_path = std::env::temp_dir().join(format!(
        "sundaydeck-test-bible-{}.sqlite",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&tmp_path);
    {
        let ext = Connection::open(&tmp_path).expect("create external sqlite bible");
        ext.execute_batch(
            "CREATE TABLE t_ylt (id INTEGER PRIMARY KEY, b INTEGER, c INTEGER, v INTEGER, t TEXT);
             INSERT INTO t_ylt (b, c, v, t) VALUES (1, 1, 1, 'In the beginning of God preparing the heavens and the earth');",
        )
        .expect("seed external sqlite");
    }
    let imported = import_sqlite_bible(db.conn_mut(), &tmp_path, "YLT", "Young's Literal")
        .expect("import sqlite");
    let _ = std::fs::remove_file(&tmp_path);
    assert_eq!(imported.code, "YLT");

    let ylt_hits = search_bible_verses(db.conn(), "YLT", "preparing", 10).expect("ylt fts");
    assert_eq!(ylt_hits.len(), 1);
    assert_eq!(ylt_hits[0].book_name, "Genesis");
}

#[test]
fn pastor_plan_service_resolves_songs_bibles_and_persists_text_decks_in_sqlite() {
    use sundaydeck_core::{PlanItemKind, ServiceId, ServiceSchedule, sample_pastor_sermon_outline};
    use sundaydeck_db::plan_service::build_plan_from_text_sync;
    use sundaydeck_db::service_repo::{get_service, upsert_service};

    let mut db = Database::open_in_memory().expect("open db");
    seed_defaults_if_empty(db.conn_mut()).expect("seed");

    let plan = build_plan_from_text_sync(
        db.conn(),
        "sunday-sermon.docx",
        sample_pastor_sermon_outline(),
        "WEB",
    )
    .expect("build service plan");

    assert_eq!(plan.sermon_title, "Walking in Unshakable Peace");
    assert_eq!(plan.stats.songs_matched, 2);
    assert_eq!(plan.stats.scriptures_resolved, 3);
    assert!(
        plan.items
            .iter()
            .any(|i| i.kind == PlanItemKind::SermonPoint)
    );

    let schedule = ServiceSchedule {
        id: ServiceId::new_v7(),
        title: plan.sermon_title.clone(),
        service_date: Some("2026-11-23".to_owned()),
        notes: "Generated from pastor upload".to_owned(),
        items: plan.enabled_items(),
    };
    upsert_service(db.conn_mut(), &schedule).expect("persist generated schedule");

    let reloaded = get_service(db.conn(), schedule.id)
        .expect("load schedule")
        .expect("schedule exists");
    assert_eq!(reloaded.items.len(), schedule.items.len());
    assert_eq!(reloaded.title, "Walking in Unshakable Peace");
}
