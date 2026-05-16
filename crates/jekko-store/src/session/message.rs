//! `message` table CRUD.

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::error::{StoreError, StoreResult};

/// Row in the `message` table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MessageRow {
    /// Message id.
    pub id: String,
    /// FK to `session.id`.
    pub session_id: String,
    /// Creation timestamp (ms since epoch).
    pub time_created: i64,
    /// Last-update timestamp (ms since epoch).
    pub time_updated: i64,
    /// Free-form message payload.
    pub data: serde_json::Value,
}

/// Insert or replace a message row.
pub fn upsert_message(conn: &Connection, row: &MessageRow) -> StoreResult<()> {
    let data = serde_json::to_string(&row.data)?;
    conn.execute(
        "INSERT INTO message (id, session_id, time_created, time_updated, data)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(id) DO UPDATE SET
            session_id = excluded.session_id,
            time_updated = excluded.time_updated,
            data = excluded.data",
        params![
            row.id,
            row.session_id,
            row.time_created,
            row.time_updated,
            data
        ],
    )?;
    Ok(())
}

/// Read a message row by id.
pub fn get_message(conn: &Connection, id: &str) -> StoreResult<Option<MessageRow>> {
    conn.query_row(
        "SELECT id, session_id, time_created, time_updated, data FROM message WHERE id = ?1",
        params![id],
        message_from_row,
    )
    .optional()
    .map_err(StoreError::from)
}

/// List messages for a session ordered by `(time_created, id)`.
pub fn list_messages(conn: &Connection, session_id: &str) -> StoreResult<Vec<MessageRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, session_id, time_created, time_updated, data
         FROM message WHERE session_id = ?1 ORDER BY time_created ASC, id ASC",
    )?;
    let rows = stmt.query_map(params![session_id], message_from_row)?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

/// Delete a message row.
pub fn delete_message(conn: &Connection, id: &str) -> StoreResult<usize> {
    Ok(conn.execute("DELETE FROM message WHERE id = ?1", params![id])?)
}

fn message_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<MessageRow> {
    let data_text: String = row.get(4)?;
    let data: serde_json::Value = serde_json::from_str(&data_text).map_err(|err| {
        rusqlite::Error::FromSqlConversionFailure(4, rusqlite::types::Type::Text, Box::new(err))
    })?;
    Ok(MessageRow {
        id: row.get(0)?,
        session_id: row.get(1)?,
        time_created: row.get(2)?,
        time_updated: row.get(3)?,
        data,
    })
}
