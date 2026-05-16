//! `part` table CRUD.

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::error::{StoreError, StoreResult};

/// Row in the `part` table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PartRow {
    /// Part id.
    pub id: String,
    /// FK to `message.id`.
    pub message_id: String,
    /// FK to `session.id`.
    pub session_id: String,
    /// Creation timestamp (ms since epoch).
    pub time_created: i64,
    /// Last-update timestamp (ms since epoch).
    pub time_updated: i64,
    /// Free-form part payload.
    pub data: serde_json::Value,
}

/// Insert or replace a part row.
pub fn upsert_part(conn: &Connection, row: &PartRow) -> StoreResult<()> {
    let data = serde_json::to_string(&row.data)?;
    conn.execute(
        "INSERT INTO part (id, message_id, session_id, time_created, time_updated, data)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(id) DO UPDATE SET
            message_id = excluded.message_id,
            session_id = excluded.session_id,
            time_updated = excluded.time_updated,
            data = excluded.data",
        params![
            row.id,
            row.message_id,
            row.session_id,
            row.time_created,
            row.time_updated,
            data
        ],
    )?;
    Ok(())
}

/// Read a part row by id.
pub fn get_part(conn: &Connection, id: &str) -> StoreResult<Option<PartRow>> {
    conn.query_row(
        "SELECT id, message_id, session_id, time_created, time_updated, data
         FROM part WHERE id = ?1",
        params![id],
        part_from_row,
    )
    .optional()
    .map_err(StoreError::from)
}

/// List parts for a message.
pub fn list_parts(conn: &Connection, message_id: &str) -> StoreResult<Vec<PartRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, message_id, session_id, time_created, time_updated, data
         FROM part WHERE message_id = ?1 ORDER BY id ASC",
    )?;
    let rows = stmt.query_map(params![message_id], part_from_row)?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

/// Delete a part row.
pub fn delete_part(conn: &Connection, id: &str) -> StoreResult<usize> {
    Ok(conn.execute("DELETE FROM part WHERE id = ?1", params![id])?)
}

fn part_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<PartRow> {
    let data_text: String = row.get(5)?;
    let data: serde_json::Value = serde_json::from_str(&data_text).map_err(|err| {
        rusqlite::Error::FromSqlConversionFailure(5, rusqlite::types::Type::Text, Box::new(err))
    })?;
    Ok(PartRow {
        id: row.get(0)?,
        message_id: row.get(1)?,
        session_id: row.get(2)?,
        time_created: row.get(3)?,
        time_updated: row.get(4)?,
        data,
    })
}
