use std::time::Instant;

use sundaydeck_core::ids::SUNDAYDECK_NAMESPACE;
use sundaydeck_core::{
    Arrangement, ArrangementId, HymnalEntry, ParsedBible, ParsedVerse, ReflowMode, SectionId,
    SectionKind, Song, SongId, SongSection,
};
use sundaydeck_db::Database;
use sundaydeck_db::bible_repo::{get_chapter_verses, import_parsed_bible, search_bible_verses};
use sundaydeck_db::seed::seed_defaults_if_empty;
use sundaydeck_db::song_repo::{search_songs, upsert_song};

/// Builds a synthetic 31,102-verse 66-book Bible with realistic verse text length and well-known
/// anchor passages to benchmark SQLite + FTS5 performance at real full-Bible scale offline.
fn build_full_scale_bible() -> ParsedBible {
    let mut verses = Vec::with_capacity(31_102);
    let mut count = 0usize;
    let total_target = 31_102usize;

    // Distribute 31,102 verses cleanly across all 66 canonical books (18..=20 chapters each).
    for book_number in 1u8..=66 {
        let chapters: u16 = if book_number == 19 { 150 } else { 18 };
        for chapter in 1u16..=chapters {
            if count >= total_target {
                break;
            }
            let verses_in_ch: u16 = if book_number == 66 && chapter == chapters {
                u16::try_from(total_target.saturating_sub(count)).unwrap_or(21)
            } else if book_number == 45 && chapter == 8 {
                39
            } else {
                24
            };
            for verse in 1u16..=verses_in_ch {
                if count >= total_target {
                    break;
                }
                let text = match (book_number, chapter, verse) {
                    (1, 1, 1) => {
                        "In the beginning God created the heavens and the earth.".to_owned()
                    }
                    (19, 23, 1) => "The LORD is my shepherd; I shall not want.".to_owned(),
                    (19, 23, 4) => "Even though I walk through the valley of the shadow of death, I will fear no evil, for You are with me.".to_owned(),
                    (43, 3, 16) => "For God so loved the world that He gave His one and only Son, that everyone who believes in Him shall not perish but have eternal life.".to_owned(),
                    (45, 8, 28) => "And we know that God works all things together for the good of those who love Him, who are called according to His purpose.".to_owned(),
                    _ => format!(
                        "And the people gathered in peace and gladness in book {book_number} chapter {chapter} verse {verse} praising the steadfast mercy of the Lord."
                    ),
                };
                verses.push(ParsedVerse {
                    book_number,
                    chapter,
                    verse,
                    text,
                });
                count += 1;
            }
        }
    }

    ParsedBible {
        code: "BSB".to_owned(),
        name: "Berean Standard Bible (Full 31k Benchmark)".to_owned(),
        language: "en".to_owned(),
        copyright: "Public Domain (CC0)".to_owned(),
        verses,
    }
}

fn synthetic_hymn(idx: usize) -> Song {
    let tag = format!("bench-hymn-{idx:04}");
    let id = SongId::from_v5(&SUNDAYDECK_NAMESPACE, tag.as_bytes());
    let sec_id = SectionId::from_v5(&SUNDAYDECK_NAMESPACE, format!("{tag}:v1").as_bytes());
    let arr_id = ArrangementId::from_v5(&SUNDAYDECK_NAMESPACE, format!("{tag}:arr").as_bytes());
    Song {
        id,
        title: format!("Classic Worship Hymn #{idx:04}"),
        alternate_title: format!("Hymnal Entry {idx}"),
        author: "Public Domain".to_owned(),
        copyright: "Public Domain".to_owned(),
        ccli_number: String::new(),
        default_theme_id: None,
        default_arrangement_id: Some(arr_id),
        default_reflow_mode: ReflowMode::ByStanza,
        primary_lang: "en".to_owned(),
        secondary_lang: None,
        hymnal_entries: vec![HymnalEntry {
            hymnal_code: "BH".to_owned(),
            hymnal_name: "Baptist Hymnal".to_owned(),
            hymn_number: u32::try_from(idx + 1).unwrap_or(1),
        }],
        traditions: vec!["Traditional Hymns".to_owned()],
        sections: vec![SongSection {
            id: sec_id,
            kind: SectionKind::Verse,
            label: "Verse 1".to_owned(),
            ordinal: 0,
            body: format!(
                "Sing praises with joyful hearts in stanza {idx}\nRejoice in mercy, grace, and enduring peace."
            ),
            secondary_body: None,
            stage_chords: None,
        }],
        arrangements: vec![Arrangement {
            id: arr_id,
            name: "Default".to_owned(),
            section_ids: vec![sec_id],
        }],
    }
}

#[test]
fn full_31k_bible_and_500_hymns_fts5_search_stays_instant() {
    let mut db = Database::open_in_memory().expect("open in-memory db");
    seed_defaults_if_empty(db.conn_mut()).expect("seed defaults");
    let bible = build_full_scale_bible();
    assert_eq!(bible.verses.len(), 31_102);

    let t_import = Instant::now();
    let tr = import_parsed_bible(db.conn_mut(), &bible).expect("import 31k bible");
    let import_ms = t_import.elapsed().as_secs_f64() * 1_000.0;
    println!("PERF bible_31k_import_ms={import_ms:.2}");

    // Populate 500 additional hymns alongside the seeded starter hymnal.
    for i in 0..500 {
        upsert_song(db.conn_mut(), &synthetic_hymn(i)).expect("upsert synthetic hymn");
    }

    // Benchmark FTS5 Bible phrase searches (median of 5 queries).
    let queries = [
        "shepherd",
        "valley of the shadow",
        "so loved the world",
        "heavens and the earth",
        "together for the good",
    ];
    let mut fts_times = Vec::with_capacity(queries.len());
    for q in queries {
        let t = Instant::now();
        let hits = search_bible_verses(db.conn(), &tr.code, q, 50).expect("fts bible search");
        fts_times.push(t.elapsed().as_secs_f64() * 1_000.0);
        assert!(!hits.is_empty(), "expected hits for '{q}'");
    }
    fts_times.sort_by(|a, b| a.total_cmp(b));
    let bible_fts_ms = fts_times[fts_times.len() / 2];
    println!("PERF bible_31k_fts_search_ms={bible_fts_ms:.3}");
    assert!(
        bible_fts_ms < 50.0,
        "31k-verse FTS5 search too slow: {bible_fts_ms:.2} ms"
    );

    // Benchmark full chapter load.
    let t_chap = Instant::now();
    let psalm_23 = get_chapter_verses(db.conn(), &tr.code, 19, 23).expect("load psalm 23");
    let chap_ms = t_chap.elapsed().as_secs_f64() * 1_000.0;
    assert!(!psalm_23.is_empty());
    println!("PERF bible_31k_chapter_load_ms={chap_ms:.3}");
    assert!(
        chap_ms < 20.0,
        "chapter load from 31k Bible too slow: {chap_ms:.2} ms"
    );

    // Benchmark 500-song FTS5 search + hymn number lookup.
    let t_song = Instant::now();
    let grace_hits = search_songs(db.conn(), "amazing grace").expect("fts song search");
    let song_fts_ms = t_song.elapsed().as_secs_f64() * 1_000.0;
    assert!(!grace_hits.is_empty());
    println!("PERF song_500_fts_search_ms={song_fts_ms:.3}");
    assert!(
        song_fts_ms < 25.0,
        "500-song FTS5 search too slow: {song_fts_ms:.2} ms"
    );
}
