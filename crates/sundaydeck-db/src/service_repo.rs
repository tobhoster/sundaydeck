use rusqlite::{Connection, OptionalExtension, params};
use sundaydeck_core::{
    ServiceId, ServiceItem, ServiceItemId, ServiceItemPayload, ServiceSchedule, ThemeId,
};

use crate::connection::now_epoch_ms;
use crate::error::DbError;

pub fn list_services(conn: &Connection) -> Result<Vec<ServiceSchedule>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT id, title, service_date, notes
         FROM services
         ORDER BY updated_at DESC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, String>(3)?,
        ))
    })?;

    let mut out = Vec::new();
    for row in rows {
        let (id_str, title, service_date, notes) = row?;
        let id = ServiceId::parse_str(&id_str)?;
        let items = load_service_items(conn, id)?;
        out.push(ServiceSchedule {
            id,
            title,
            service_date,
            notes,
            items,
        });
    }
    Ok(out)
}

pub fn get_service(conn: &Connection, id: ServiceId) -> Result<Option<ServiceSchedule>, DbError> {
    let header = conn
        .query_row(
            "SELECT id, title, service_date, notes FROM services WHERE id = ?1",
            [id.to_string()],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, String>(3)?,
                ))
            },
        )
        .optional()?;

    let Some((id_str, title, service_date, notes)) = header else {
        return Ok(None);
    };
    let parsed_id = ServiceId::parse_str(&id_str)?;
    let items = load_service_items(conn, parsed_id)?;
    Ok(Some(ServiceSchedule {
        id: parsed_id,
        title,
        service_date,
        notes,
        items,
    }))
}

pub fn upsert_service(conn: &mut Connection, schedule: &ServiceSchedule) -> Result<(), DbError> {
    schedule.validate()?;
    let now = now_epoch_ms();
    let service_id_str = schedule.id.to_string();

    let tx = conn.transaction()?;
    tx.execute(
        "INSERT INTO services (id, title, service_date, notes, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(id) DO UPDATE SET
             title = excluded.title,
             service_date = excluded.service_date,
             notes = excluded.notes,
             updated_at = excluded.updated_at",
        params![
            service_id_str,
            schedule.title,
            schedule.service_date,
            schedule.notes,
            now,
            now,
        ],
    )?;

    tx.execute(
        "DELETE FROM service_items WHERE service_id = ?1",
        [&service_id_str],
    )?;

    for (pos, item) in schedule.items.iter().enumerate() {
        let (song_id, arr_id, reflow_str) = match &item.payload {
            ServiceItemPayload::Song {
                song_id,
                arrangement_id,
                reflow_override,
            } => (
                Some(song_id.to_string()),
                arrangement_id.map(|id| id.to_string()),
                reflow_override.map(|r| r.as_db_str()),
            ),
            ServiceItemPayload::Scripture { .. }
            | ServiceItemPayload::Presentation { .. }
            | ServiceItemPayload::TextDeck { .. }
            | ServiceItemPayload::AnnouncementLoop { .. }
            | ServiceItemPayload::CustomSlide { .. }
            | ServiceItemPayload::CustomDeck { .. } => (None, None, None),
        };
        let payload_json = serde_json::to_string(&item.payload)?;

        tx.execute(
            "INSERT INTO service_items (
                 id, service_id, position, kind, title,
                 song_id, arrangement_id, theme_override_id, reflow_override, payload_json
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                item.id.to_string(),
                service_id_str,
                i64::try_from(pos).unwrap_or(0),
                item.payload.db_kind(),
                item.title,
                song_id,
                arr_id,
                item.theme_override_id.map(|id| id.to_string()),
                reflow_str,
                payload_json,
            ],
        )?;
    }

    tx.commit()?;
    Ok(())
}

/// Atomically reorders `service_items` inside a transaction without violating `UNIQUE(service_id, position)`.
pub fn reorder_service_items(
    conn: &mut Connection,
    service_id: ServiceId,
    ordered_ids: &[ServiceItemId],
) -> Result<(), DbError> {
    let service_id_str = service_id.to_string();
    let offset: i64 = 1_000_000;
    let tx = conn.transaction()?;

    for (idx, item_id) in ordered_ids.iter().enumerate() {
        let temp_pos = offset + i64::try_from(idx).unwrap_or(0);
        tx.execute(
            "UPDATE service_items SET position = ?1 WHERE service_id = ?2 AND id = ?3",
            params![temp_pos, &service_id_str, item_id.to_string()],
        )?;
    }
    for (idx, item_id) in ordered_ids.iter().enumerate() {
        let final_pos = i64::try_from(idx).unwrap_or(0);
        tx.execute(
            "UPDATE service_items SET position = ?1 WHERE service_id = ?2 AND id = ?3",
            params![final_pos, &service_id_str, item_id.to_string()],
        )?;
    }
    tx.execute(
        "UPDATE services SET updated_at = ?1 WHERE id = ?2",
        params![now_epoch_ms(), &service_id_str],
    )?;

    tx.commit()?;
    Ok(())
}

pub fn delete_service(conn: &Connection, id: ServiceId) -> Result<(), DbError> {
    conn.execute("DELETE FROM services WHERE id = ?1", [id.to_string()])?;
    Ok(())
}

fn load_service_items(
    conn: &Connection,
    service_id: ServiceId,
) -> Result<Vec<ServiceItem>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT id, title, theme_override_id, payload_json
         FROM service_items
         WHERE service_id = ?1
         ORDER BY position ASC",
    )?;
    let rows = stmt.query_map([service_id.to_string()], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, String>(3)?,
        ))
    })?;

    let mut items = Vec::new();
    for row in rows {
        let (id_str, title, theme_str, payload_json) = row?;
        let payload: ServiceItemPayload = serde_json::from_str(&payload_json)?;
        items.push(ServiceItem {
            id: ServiceItemId::parse_str(&id_str)?,
            title,
            theme_override_id: theme_str.as_deref().map(ThemeId::parse_str).transpose()?,
            payload,
        });
    }
    Ok(items)
}
