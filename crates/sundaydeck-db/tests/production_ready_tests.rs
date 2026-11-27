use sundaydeck_core::{
    CustomSlideLayout, CustomSlideSpec, HorizontalAlign, SlideId, SlideTransition, SongId,
};
use sundaydeck_db::production_repo::{
    CcliProjectionInput, count_ccli_projections, create_safe_backup, delete_custom_slide,
    export_ccli_csv, list_custom_slides, record_ccli_projection, upsert_custom_slide,
};
use sundaydeck_db::seed::seed_defaults_if_empty;
use sundaydeck_db::{Database, DbWorker};

#[test]
fn custom_slides_persist_across_database_reopen_and_delete_cleanly() {
    let tmp = std::env::temp_dir().join(format!("sundaydeck-prod-custom-{}.db", SlideId::new_v7()));
    let slide_id = SlideId::new_v7();
    {
        let mut db = Database::open(&tmp).expect("open temp db");
        seed_defaults_if_empty(db.conn_mut()).expect("seed");
        let seeded = list_custom_slides(db.conn()).expect("list seeded");
        assert!(seeded.len() >= 8, "seeded starter custom slides");

        let quote = CustomSlideSpec {
            id: slide_id,
            label: "Pastor Sunday Quote".to_owned(),
            layout: CustomSlideLayout::BigStatement {
                statement: "Faithful in little, faithful in much.".to_owned(),
                attribution: Some("Pastor Samuel".to_owned()),
                align: HorizontalAlign::Center,
            },
            transition_override: Some(SlideTransition::DEFAULT_ZOOM_DISSOLVE),
        };
        upsert_custom_slide(db.conn(), &quote).expect("upsert quote");
    }

    {
        let db = Database::open(&tmp).expect("reopen temp db");
        let loaded = list_custom_slides(db.conn()).expect("list reopened");
        assert!(
            loaded
                .iter()
                .any(|s| s.id == slide_id && s.label == "Pastor Sunday Quote"),
            "saved custom slide must survive app restart"
        );
        delete_custom_slide(db.conn(), slide_id).expect("delete quote");
        let after = list_custom_slides(db.conn()).expect("list after delete");
        assert!(!after.iter().any(|s| s.id == slide_id));
    }

    let _ = std::fs::remove_file(&tmp);
    let _ = std::fs::remove_file(tmp.with_extension("db-wal"));
    let _ = std::fs::remove_file(tmp.with_extension("db-shm"));
}

#[test]
fn ccli_log_dedupes_within_window_and_exports_safe_csv() {
    let mut db = Database::open_in_memory().expect("in-memory db");
    seed_defaults_if_empty(db.conn_mut()).expect("seed");

    let song_id = SongId::new_v7();
    let t0 = 1_760_000_000_000_i64;
    let first = CcliProjectionInput {
        song_id: Some(song_id),
        title: "Amazing Grace".to_owned(),
        author: "John Newton".to_owned(),
        ccli_number: "22025".to_owned(),
        copyright: "Public Domain".to_owned(),
        projected_at: t0,
    };
    assert!(record_ccli_projection(db.conn(), &first).expect("log first"));

    // Stepping to another verse 2 minutes later must NOT create a duplicate CCLI row.
    let mut dup = first.clone();
    dup.projected_at = t0 + 120_000;
    assert!(!record_ccli_projection(db.conn(), &dup).expect("dedupe second"));

    // Next service (2 hours later) SHOULD record a second projection row, with CSV injection guarded.
    let hostile = CcliProjectionInput {
        song_id: Some(SongId::new_v7()),
        title: "=CMD|'/C calc'!A0".to_owned(),
        author: "+Malicious, \"Author\"".to_owned(),
        ccli_number: "99999".to_owned(),
        copyright: "@Inject".to_owned(),
        projected_at: t0 + 7_200_000,
    };
    assert!(record_ccli_projection(db.conn(), &hostile).expect("log second"));
    assert_eq!(count_ccli_projections(db.conn()).expect("count"), 2);

    let csv_path = std::env::temp_dir().join(format!("sundaydeck-ccli-{}.csv", SlideId::new_v7()));
    let rows = export_ccli_csv(db.conn(), &csv_path).expect("export csv");
    assert_eq!(rows, 2);

    let csv = std::fs::read_to_string(&csv_path).expect("read csv");
    assert!(csv.starts_with("projected_at_utc,title,author,ccli_number,copyright\n"));
    assert!(csv.contains("\"Amazing Grace\",\"John Newton\",\"22025\",\"Public Domain\""));
    assert!(
        csv.contains(
            "\"'=CMD|'/C calc'!A0\",\"'+Malicious, \"\"Author\"\"\",\"99999\",\"'@Inject\""
        ),
        "formula-leading cells must be neutralized, got:\n{csv}"
    );
    let _ = std::fs::remove_file(&csv_path);
}

#[test]
fn safe_sqlite_backup_produces_consistent_standalone_copy() {
    let mut db = Database::open_in_memory().expect("in-memory db");
    seed_defaults_if_empty(db.conn_mut()).expect("seed");

    let backup_path =
        std::env::temp_dir().join(format!("sundaydeck-backup-{}.db", SlideId::new_v7()));
    let bytes = create_safe_backup(db.conn(), &backup_path).expect("vacuum backup");
    assert!(
        bytes > 16_384,
        "backup should contain full schema and seeded content"
    );

    let reopened = Database::open(&backup_path).expect("open backed-up db");
    let boot =
        sundaydeck_db::worker::load_bootstrap_sync(&reopened).expect("load backup bootstrap");
    assert!(boot.songs.len() >= 19);
    assert!(boot.custom_slides.len() >= 8);
    assert!(boot.themes.len() >= 12);

    // Overwriting an existing backup must also succeed atomically.
    let bytes2 = create_safe_backup(db.conn(), &backup_path).expect("second vacuum backup");
    assert!(bytes2 > 16_384);

    let _ = std::fs::remove_file(&backup_path);
    let _ = std::fs::remove_file(backup_path.with_extension("db-wal"));
    let _ = std::fs::remove_file(backup_path.with_extension("db-shm"));

    // Verify async DbWorker roundtrip for custom slides and CCLI logging too.
    let worker = DbWorker::spawn_in_memory().expect("spawn worker");
    let boot2 = block_on(worker.load_bootstrap()).expect("worker bootstrap");
    assert!(boot2.custom_slides.len() >= 8);
}

fn block_on<F: std::future::Future>(fut: F) -> F::Output {
    use std::task::{Context, Poll, Waker};
    let mut fut = std::pin::pin!(fut);
    let waker = Waker::noop();
    let mut cx = Context::from_waker(waker);
    loop {
        match fut.as_mut().poll(&mut cx) {
            Poll::Ready(out) => return out,
            Poll::Pending => std::thread::yield_now(),
        }
    }
}
