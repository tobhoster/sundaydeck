use sundaydeck_core::defaults::default_sample_service;
use sundaydeck_db::Database;
use sundaydeck_db::seed::seed_defaults_if_empty;
use sundaydeck_db::service_repo::{reorder_service_items, upsert_service};
use sundaydeck_db::worker::load_bootstrap_sync;

/// A skipped (undeserializable) service item must survive a reorder and a save,
/// parked after the loaded items.
#[test]
fn skipped_service_item_survives_reorder_and_save() {
    let mut db = Database::open_in_memory().expect("open db");
    seed_defaults_if_empty(db.conn_mut()).expect("seed");
    let bad_item = default_sample_service().items[2].id.to_string();
    db.conn()
        .execute(
            "UPDATE service_items SET payload_json = '{\"not\":\"a payload\"}' WHERE id = ?1",
            [&bad_item],
        )
        .expect("corrupt service item");

    let boot = load_bootstrap_sync(&db).expect("bootstrap");
    let mut schedule = boot.services[0].clone();
    assert_eq!(schedule.items.len(), 4);
    schedule.items.reverse();
    let ids: Vec<_> = schedule.items.iter().map(|i| i.id).collect();

    reorder_service_items(db.conn_mut(), schedule.id, &ids).expect("reorder");
    assert_eq!(corrupt_position(&db, &bad_item), Some(4));

    upsert_service(db.conn_mut(), &schedule).expect("save");
    assert_eq!(corrupt_position(&db, &bad_item), Some(4));
    let payload: String = db
        .conn()
        .query_row(
            "SELECT payload_json FROM service_items WHERE id = ?1",
            [&bad_item],
            |row| row.get(0),
        )
        .expect("corrupt row payload");
    assert_eq!(payload, "{\"not\":\"a payload\"}");

    let reloaded = load_bootstrap_sync(&db).expect("reload");
    let reloaded_ids: Vec<_> = reloaded.services[0].items.iter().map(|i| i.id).collect();
    assert_eq!(reloaded_ids, ids);
}

fn corrupt_position(db: &Database, id: &str) -> Option<i64> {
    db.conn()
        .query_row(
            "SELECT position FROM service_items WHERE id = ?1",
            [id],
            |row| row.get(0),
        )
        .ok()
}
