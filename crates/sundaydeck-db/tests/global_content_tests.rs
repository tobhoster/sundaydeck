use rusqlite::Connection;
use sundaydeck_core::{
    BilingualDisplayMode, HymnalEntry, LiturgyTemplate, PackBiblePayload, SdeckPack,
    SdeckPackManifest, SongImporter, UniversalSongImporter, detect_and_import_bible,
    export_sdeck_pack, import_chordpro_song, import_freeshow_or_videopsalm_json,
    import_numbered_hymnal_text, import_sdeck_pack, paginate_parallel_scripture,
    parse_scripture_ref, public_domain_liturgies, reflow_song_bilingual, sample_wesley_hymn_pd,
    strip_rtf_to_plain_text,
};
use sundaydeck_db::bible_repo::{get_passage, import_parsed_bible, search_bible_verses};
use sundaydeck_db::content_pack_repo::{install_sdeck_pack, list_content_packs, list_liturgies};
use sundaydeck_db::legacy_importers::{import_easyworship_sqlite, import_openlp_sqlite};
use sundaydeck_db::song_repo::{get_song, search_songs, upsert_song};
use sundaydeck_db::Database;

#[test]
fn catholic_73_book_usfm_import_and_multilingual_reference_parsing() {
    // 73-book Catholic + Spanish + Yoruba + French localized book aliases
    let sir = parse_scripture_ref("Sirach 2:1").expect("Sirach 2:1");
    assert_eq!((sir.book_number, sir.chapter, sir.verse_start), (70, 2, Some(1)));

    let wis = parse_scripture_ref("Wisdom 3:1-4").expect("Wisdom 3:1-4");
    assert_eq!(
        (wis.book_number, wis.chapter, wis.verse_start, wis.verse_end),
        (69, 3, Some(1), Some(4))
    );

    let sab = parse_scripture_ref("Sabiduría 3:1").expect("Spanish Sabiduría");
    assert_eq!((sab.book_number, sab.chapter, sab.verse_start), (69, 3, Some(1)));

    let eclo = parse_scripture_ref("eclo 2:1").expect("Spanish Eclesiástico");
    assert_eq!((eclo.book_number, eclo.chapter, eclo.verse_start), (70, 2, Some(1)));

    let sal = parse_scripture_ref("Sal 23:1-3").expect("Spanish Salmos");
    assert_eq!((sal.book_number, sal.chapter, sal.verse_start, sal.verse_end), (19, 23, Some(1), Some(3)));

    let yoruba_ps = parse_scripture_ref("Orin Dafidi 23:1").expect("Yoruba Psalm");
    assert_eq!((yoruba_ps.book_number, yoruba_ps.chapter, yoruba_ps.verse_start), (19, 23, Some(1)));

    let ap = parse_scripture_ref("Ap 21:4").expect("Spanish Apocalipsis");
    assert_eq!((ap.book_number, ap.chapter, ap.verse_start), (66, 21, Some(4)));

    // USFM parser handles Deuterocanonical \id WIS + \id SIR and strips footnotes cleanly
    let usfm = r#"
\id WIS Catholic Public Domain Version
\c 3
\v 1 But the souls of the just are in the hand of God,\f + \fr 3:1 \ft Footnote removed\f* and the torment of death shall not touch them.
\v 2 In the sight of the unwise, they seemed to die.
\id SIR Catholic Public Domain Version
\c 2
\v 1 Son, when you come to the service of God, stand in justice and in fear, and prepare your soul for temptation.
"#;
    let parsed = detect_and_import_bible("CPDV", "Catholic Public Domain Version", "cpdv.usfm", usfm)
        .expect("parse USFM 73-book");
    assert_eq!(parsed.verses.len(), 3);

    let mut db = Database::open_in_memory().expect("open db");
    import_parsed_bible(db.conn_mut(), &parsed).expect("import 73-book bible");
    let loaded_wis = get_passage(db.conn(), "CPDV", &wis).expect("query Wisdom");
    assert_eq!(loaded_wis.len(), 2);
    assert!(loaded_wis[0].text.contains("hand of God"));
    assert!(!loaded_wis[0].text.contains("Footnote removed"));

    let loaded_sir = get_passage(db.conn(), "CPDV", &sir).expect("query Sirach");
    assert_eq!(loaded_sir.len(), 1);
    assert!(loaded_sir[0].text.contains("prepare your soul"));
}

#[test]
fn diacritic_insensitive_fts5_search_matches_yoruba_and_spanish_without_accents() {
    let mut db = Database::open_in_memory().expect("open db");
    let yoruba_usfm = r#"
\id PSA Bibeli Mimo
\c 23
\v 1 Olúwa ni olùṣọ́-àgùntàn mi; èmi kì yóò ṣe aláìní. Ọlọ́run mi ń tọ́ mi sọ́nà.
\v 2 Bíbélì Mímọ́ fi Ẹlẹ́rìí òtítọ́ hàn fún gbogbo ènìyàn.
"#;
    let yo_parsed = detect_and_import_bible("YCB", "Bíbélì Mímọ́", "yoruba.usfm", yoruba_usfm)
        .expect("parse yoruba");
    import_parsed_bible(db.conn_mut(), &yo_parsed).expect("import yoruba");

    let es_usfm = r#"
\id PRO Reina-Valera 1909
\c 3
\v 5 Fíate de Jehová de todo tu corazón, y no estribes en tu propia sabiduría.
"#;
    let es_parsed = detect_and_import_bible("RV1909", "Reina-Valera 1909", "rv1909.usfm", es_usfm)
        .expect("parse spanish");
    import_parsed_bible(db.conn_mut(), &es_parsed).expect("import spanish");

    // Plain ASCII queries match Yoruba diacritics & underdots
    let h1 = search_bible_verses(db.conn(), "YCB", "Olorun", 10).expect("Olorun search");
    assert_eq!(h1.len(), 1);
    assert!(h1[0].text.contains("Ọlọ́run"));

    let h2 = search_bible_verses(db.conn(), "YCB", "Bibeli Mimo", 10).expect("Bibeli Mimo search");
    assert_eq!(h2.len(), 1);
    assert!(h2[0].text.contains("Bíbélì Mímọ́"));

    let h3 = search_bible_verses(db.conn(), "YCB", "Eleri", 10).expect("Eleri search");
    assert_eq!(h3.len(), 1);
    assert!(h3[0].text.contains("Ẹlẹ́rìí"));

    // Plain ASCII queries match Spanish accents
    let s1 = search_bible_verses(db.conn(), "RV1909", "Corazon Sabiduria", 10).expect("Spanish search");
    assert_eq!(s1.len(), 1);
    assert!(s1[0].text.contains("corazón"));

    // Parallel dual-translation pagination (Yoruba + English)
    let en_usfm = r#"
\id PSA World English Bible
\c 23
\v 1 Yahweh is my shepherd; I shall lack nothing.
"#;
    let en_parsed = detect_and_import_bible("WEB", "World English Bible", "web.usfm", en_usfm)
        .expect("parse english");
    import_parsed_bible(db.conn_mut(), &en_parsed).expect("import english");

    let ps23 = parse_scripture_ref("Psalm 23:1").expect("ps 23:1");
    let yo_v = get_passage(db.conn(), "YCB", &ps23).expect("yo verse");
    let en_v = get_passage(db.conn(), "WEB", &ps23).expect("en verse");
    let parallel_slides = paginate_parallel_scripture(&yo_v, &en_v);
    assert_eq!(parallel_slides.len(), 1);
    let body = parallel_slides[0].content.primary_text();
    assert!(body.contains("Olúwa ni olùṣọ́-àgùntàn mi"));
    assert!(body.contains("Yahweh is my shepherd"));
}

#[test]
fn hymn_number_instant_jump_and_bilingual_stacked_reflow() {
    let mut db = Database::open_in_memory().expect("open db");

    // Import numbered CAC, CCC, and MHB hymns via bulk numbered hymnal parser (with bilingual lines!)
    let bulk_txt = "\
CAC 120 — Iyin Fun Olorun
Ọlọ́run wa tí ó ga jùlọ || Our God who reigns on high
Àwa ń yìn Ọ́ lónìí || We lift our praise today

CCC 315 — Mimo Mimo Mimo
Mímọ́ mímọ́ mímọ́ Olúwa || Holy holy holy Lord
Ayé àti ọ̀run kún fún ògo Rẹ || Earth and heaven are full of Your glory

MHB 1 — O For a Thousand Tongues
O for a thousand tongues to sing
My great Redeemer's praise
";
    let imported = import_numbered_hymnal_text(bulk_txt).expect("parse bulk hymnal");
    assert_eq!(imported.len(), 3);
    for song in &imported {
        upsert_song(db.conn_mut(), song).expect("save hymn");
    }

    // Exact hymnal-code + number jumps must return exact match at index 0
    let cac_hits = search_songs(db.conn(), "CAC 120").expect("CAC 120 jump");
    assert!(!cac_hits.is_empty());
    assert_eq!(cac_hits[0].title, "Iyin Fun Olorun");

    let ccc_hits = search_songs(db.conn(), "ccc 315").expect("ccc 315 jump");
    assert!(!ccc_hits.is_empty());
    assert_eq!(ccc_hits[0].title, "Mimo Mimo Mimo");

    let hash_hits = search_songs(db.conn(), "#315").expect("#315 jump");
    assert!(!hash_hits.is_empty());
    assert_eq!(hash_hits[0].title, "Mimo Mimo Mimo");

    let mhb_hits = search_songs(db.conn(), "MHB 1").expect("MHB 1 jump");
    assert!(!mhb_hits.is_empty());
    assert_eq!(mhb_hits[0].title, "O For a Thousand Tongues");

    // Round-trip hymnal entries & bilingual lyrics from SQLite
    let loaded_cac = get_song(db.conn(), cac_hits[0].id)
        .expect("load song")
        .expect("exists");
    assert_eq!(
        loaded_cac.hymnal_entries,
        vec![HymnalEntry {
            hymnal_code: "CAC".to_owned(),
            hymnal_name: "CAC".to_owned(),
            hymn_number: 120,
        }]
    );

    // Bilingual display modes
    let stacked = reflow_song_bilingual(&loaded_cac, None, None, BilingualDisplayMode::BilingualStacked);
    assert!(stacked[0].content.primary_text().contains("Ọlọ́run wa"));
    assert!(stacked[0].content.primary_text().contains("Our God who reigns on high"));

    let primary_only = reflow_song_bilingual(&loaded_cac, None, None, BilingualDisplayMode::PrimaryOnly);
    assert!(primary_only[0].content.primary_text().contains("Ọlọ́run wa"));
    assert!(!primary_only[0].content.primary_text().contains("Our God who reigns on high"));

    let secondary_only = reflow_song_bilingual(&loaded_cac, None, None, BilingualDisplayMode::SecondaryOnly);
    assert!(secondary_only[0].content.primary_text().contains("Our God who reigns on high"));
    assert!(!secondary_only[0].content.primary_text().contains("Ọlọ́run wa"));
}

#[test]
fn easyworship_rtf_openlp_sqlite_chordpro_and_freeshow_importers() {
    let dir = std::env::temp_dir().join(format!(
        "sundaydeck-legacy-import-{}",
        std::process::id()
    ));
    let _ = std::fs::create_dir_all(&dir);

    // 1. EasyWorship Songs.db + SongWords.db with RTF lyrics
    let ew_songs = dir.join("Songs.db");
    let ew_words = dir.join("SongWords.db");
    let _ = std::fs::remove_file(&ew_songs);
    let _ = std::fs::remove_file(&ew_words);
    {
        let sc = Connection::open(&ew_songs).expect("ew songs");
        sc.execute_batch(
            "CREATE TABLE song (rowid INTEGER PRIMARY KEY, title TEXT, author TEXT, copyright TEXT);
             INSERT INTO song VALUES (1, 'Holy Holy Holy', 'Reginald Heber', 'Public Domain');",
        )
        .expect("seed ew songs");
        let wc = Connection::open(&ew_words).expect("ew words");
        wc.execute_batch(
            "CREATE TABLE word (song_id INTEGER, words TEXT);
             INSERT INTO word VALUES (1, '{\\rtf1\\ansi{\\fonttbl\\f0 Arial;}\\fs28 Holy, holy, holy!\\par Lord God Almighty!\\par\\par Early in the morning\\par Our song shall rise to Thee}');",
        )
        .expect("seed ew words");
    }
    let ew_out = import_easyworship_sqlite(&ew_songs, &ew_words).expect("import ew");
    assert_eq!(ew_out.len(), 1);
    assert_eq!(ew_out[0].title, "Holy Holy Holy");
    assert_eq!(ew_out[0].sections.len(), 2);
    assert!(ew_out[0].sections[0].body.contains("Lord God Almighty!"));
    assert!(!ew_out[0].sections[0].body.contains("\\rtf1"));
    assert_eq!(strip_rtf_to_plain_text("{\\rtf1\\ansi Hello\\par World}"), "Hello\nWorld");

    // 2. OpenLP songs.sqlite with XML <verse> payloads
    let olp_path = dir.join("openlp_songs.sqlite");
    let _ = std::fs::remove_file(&olp_path);
    {
        let oc = Connection::open(&olp_path).expect("openlp db");
        oc.execute_batch(
            "CREATE TABLE songs (id INTEGER PRIMARY KEY, title TEXT, lyrics TEXT, copyright TEXT, ccli_number TEXT);
             CREATE TABLE authors (id INTEGER PRIMARY KEY, display_name TEXT);
             CREATE TABLE authors_songs (author_id INTEGER, song_id INTEGER);
             INSERT INTO authors VALUES (1, 'Isaac Watts');
             INSERT INTO songs VALUES (1, 'When I Survey the Wondrous Cross',
                 '<song><lyrics><verse type=\"v\" label=\"1\"><![CDATA[When I survey the wondrous cross\nOn which the Prince of glory died]]></verse><verse type=\"c\" label=\"1\"><![CDATA[Love so amazing, so divine]]></verse></lyrics></song>',
                 'Public Domain', '27893');
             INSERT INTO authors_songs VALUES (1, 1);",
        )
        .expect("seed openlp");
    }
    let olp_out = import_openlp_sqlite(&olp_path).expect("import openlp");
    assert_eq!(olp_out.len(), 1);
    assert_eq!(olp_out[0].author, "Isaac Watts");
    assert_eq!(olp_out[0].sections.len(), 2);
    assert!(olp_out[0].sections[1].body.contains("Love so amazing"));

    // 3. ChordPro strips sanctuary chords & retains stage_chords
    let cho = "{title: Doxology}\n{artist: Thomas Ken}\n[G]Praise God from [D]whom all [Em]blessings [C]flow\n";
    let cho_song = import_chordpro_song(cho).expect("chordpro");
    assert_eq!(cho_song.sections[0].body, "Praise God from whom all blessings flow");
    assert!(
        cho_song.sections[0]
            .stage_chords
            .as_deref()
            .unwrap_or("")
            .contains("[G]Praise")
    );

    // 4. FreeShow JSON via UniversalSongImporter
    let freeshow = r#"{"name":"Be Thou My Vision","slides":{"s1":{"items":[{"lines":[{"text":[{"value":"Be Thou my vision, O Lord of my heart"}]}]}]}}}"#;
    let fs_songs = import_freeshow_or_videopsalm_json(freeshow).expect("freeshow");
    assert_eq!(fs_songs[0].title, "Be Thou My Vision");
    assert!(UniversalSongImporter.import_text("song.cho", cho).is_ok());

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn sdeck_pack_roundtrip_installs_bibles_hymns_liturgies_and_license_attribution() {
    let dir = std::env::temp_dir().join(format!("sundaydeck-pack-test-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let pack_path = dir.join("global-starter.sdeck-pack");

    let pack = SdeckPack {
        manifest: SdeckPackManifest {
            schema_version: 1,
            pack_id: "sundaydeck-catholic-methodist-pd".to_owned(),
            name: "Public Domain Global Starter Pack".to_owned(),
            version: "1.0.0".to_owned(),
            tradition: "Catholic & Methodist".to_owned(),
            languages: vec!["en".to_owned(), "es".to_owned()],
            license: "Public Domain".to_owned(),
            attribution: "Classic PD Hymns & Traditional Mass Responses".to_owned(),
            source_url: "https://sundaydeck.org/packs".to_owned(),
            bible_count: 1,
            song_count: 1,
            liturgy_count: 2,
        },
        bibles: vec![PackBiblePayload {
            code: "CPDV".to_owned(),
            name: "Catholic Public Domain Version".to_owned(),
            language: "en".to_owned(),
            copyright: "Public Domain".to_owned(),
            verses: vec![(
                69,
                3,
                1,
                "But the souls of the just are in the hand of God.".to_owned(),
            )],
        }],
        songs: vec![sample_wesley_hymn_pd()],
        liturgies: public_domain_liturgies(),
    };

    export_sdeck_pack(&pack_path, &pack).expect("export .sdeck-pack");
    let loaded_pack = import_sdeck_pack(&pack_path).expect("import .sdeck-pack");
    assert_eq!(loaded_pack.manifest.pack_id, pack.manifest.pack_id);
    assert_eq!(loaded_pack.manifest.license, "Public Domain");

    let mut db = Database::open_in_memory().expect("open db");
    let report = install_sdeck_pack(db.conn_mut(), &loaded_pack).expect("install pack");
    assert_eq!(report.bibles_installed, 1);
    assert_eq!(report.songs_installed, 1);
    assert!(report.liturgies_installed >= 1);

    let packs = list_content_packs(db.conn()).expect("list packs");
    assert_eq!(packs.len(), 1);
    assert_eq!(packs[0].license, "Public Domain");

    let liturgies: Vec<LiturgyTemplate> = list_liturgies(db.conn()).expect("list liturgies");
    assert!(!liturgies.is_empty());
    assert!(liturgies[0].lines.len() >= 2);

    // MHB 1 from Charles Wesley starter hymn should jump immediately
    let wesley_hits = search_songs(db.conn(), "MHB 1").expect("MHB 1 lookup");
    assert!(!wesley_hits.is_empty());
    assert!(wesley_hits[0].title.contains("Thousand Tongues"));

    // Verify all 4 Python-built Public Domain starter packs load and install cleanly if present
    let starter_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../dist/starter-packs");
    if starter_dir.is_dir() {
        let mut pack_count = 0usize;
        for entry in std::fs::read_dir(&starter_dir).expect("read starter-packs dir") {
            let path = entry.expect("dir entry").path();
            if path.extension().and_then(|e| e.to_str()) == Some("sdeck-pack") {
                let starter = import_sdeck_pack(&path).expect("import python starter pack");
                assert_eq!(starter.manifest.license, "Public Domain");
                install_sdeck_pack(db.conn_mut(), &starter).expect("install python starter pack");
                pack_count += 1;
            }
        }
        assert!(pack_count >= 4, "expected 4 starter packs in dist/starter-packs");
    }

    let _ = std::fs::remove_dir_all(&dir);
}
