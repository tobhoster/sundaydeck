//! Bulk song import regressions (plan C8, WP6 `merge_imported_songs`): importing a library must
//! never stage or take songs live, must sort once, and must notify once.
mod support;

use std::time::Instant;

use sundaydeck_core::{PresentationCommand, Song, SongId};
use support::{Rig, RigOptions, rig_with};

fn rig_live_with_direct_click() -> Rig {
    let mut r = rig_with(RigOptions {
        db: false,
        ..RigOptions::default()
    });
    r.go_live_with_first_song();
    r.dispatch(PresentationCommand::SetDirectClickLive(true));
    r
}

fn imported_copies(r: &mut Rig, titles: &[String]) -> Vec<Song> {
    let template = r
        .app
        .update(|cx| r.store.read(cx).songs.first().cloned())
        .expect("seeded song");
    titles
        .iter()
        .map(|title| {
            let mut song = template.clone();
            song.id = SongId::new_v7();
            song.title = title.clone();
            song
        })
        .collect()
}

fn preview_title(r: &mut Rig) -> Option<String> {
    r.app.update(|cx| {
        r.store
            .read(cx)
            .snapshot()
            .preview
            .as_ref()
            .map(|p| p.title.clone())
    })
}

#[test]
fn bulk_import_with_direct_click_live_leaves_live_and_preview_alone() {
    let mut r = rig_live_with_direct_click();
    let (live_before, preview_before) = (r.live_title(), preview_title(&mut r));
    assert!(live_before.is_some(), "fixture: a song is live");
    let songs_before = r.app.update(|cx| r.store.read(cx).songs.len());

    let titles: Vec<String> = (0..2000).map(|i| format!("Imported Hymn {i:05}")).collect();
    let songs = imported_copies(&mut r, &titles);
    let (notifies, _sub) = r.count_store_notifies();
    let t0 = Instant::now();
    r.app.update(|cx| {
        r.store
            .update(cx, |s, cx| s.merge_imported_songs(songs, cx))
    });
    let ms = t0.elapsed().as_secs_f64() * 1000.0;
    r.app.run_until_parked();
    println!("PERF import_2000_songs_ms={ms:.1}");

    assert_eq!(
        r.live_title(),
        live_before,
        "bulk import changed live output"
    );
    assert_eq!(
        preview_title(&mut r),
        preview_before,
        "bulk import restaged preview"
    );
    assert_eq!(
        notifies.get(),
        1,
        "bulk import must notify the store exactly once"
    );
    let songs_after = r.app.update(|cx| r.store.read(cx).songs.len());
    assert_eq!(songs_after, songs_before + 2000);
}

#[test]
fn bulk_import_upserts_by_id_sorts_case_insensitively_and_clears_the_search_filter() {
    let mut r = rig_live_with_direct_click();
    let titles = ["zion's hill", "Abide With Me (Imported)", "amazing love"].map(String::from);
    let mut songs = imported_copies(&mut r, &titles);
    let (existing_id, existing_count) = r.app.update(|cx| {
        r.store.update(cx, |s, cx| {
            s.set_song_filter("grace".to_owned(), Some(Vec::new()), cx);
            (s.songs[0].id, s.songs.len())
        })
    });
    let mut renamed = songs[0].clone();
    renamed.id = existing_id;
    renamed.title = "Bethel Revised".to_owned();
    songs.push(renamed);

    r.app.update(|cx| {
        r.store
            .update(cx, |s, cx| s.merge_imported_songs(songs, cx))
    });

    r.app.update(|cx| {
        let s = r.store.read(cx);
        assert_eq!(
            s.songs.len(),
            existing_count + 3,
            "an imported song with an existing id replaces it instead of duplicating it"
        );
        assert_eq!(
            s.song_by_id(existing_id).map(|song| song.title.as_str()),
            Some("Bethel Revised")
        );
        let lowered: Vec<String> = s
            .songs
            .iter()
            .map(|song| song.title.to_lowercase())
            .collect();
        assert!(
            lowered.windows(2).all(|pair| pair[0] <= pair[1]),
            "library must be sorted case-insensitively by title: {lowered:?}"
        );
        assert_eq!(s.filtered_song_ids(), None);
        assert!(s.song_search_query.is_empty());
    });
}
