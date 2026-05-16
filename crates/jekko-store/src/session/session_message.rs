//! `session_message` table CRUD.

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::error::{StoreError, StoreResult};

/// Row in the `session_message` table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionMessageRow {
    /// Session-message id.
    pub id: String,
    /// FK to `session.id`.
    pub session_id: String,
    /// Type tag.
    #[serde(rename = "type")]
    pub kind: String,
    /// Creation timestamp (ms since epoch).
    pub time_created: i64,
    /// Last-update timestamp (ms since epoch).
    pub time_updated: i64,
    /// Free-form payload.
    pub data: serde_json::Value,
}

/// Insert or replace a session_message row.
pub fn upsert_session_message(conn: &Connection, row: &SessionMessageRow) -> StoreResult<()> {
    let data = serde_json::to_string(&row.data)?;
    conn.execute(
        "INSERT INTO session_message (id, session_id, type, time_created, time_updated, data)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(id) DO UPDATE SET
            session_id = excluded.session_id,
            type = excluded.type,
            time_updated = excluded.time_updated,
            data = excluded.data",
        params![
            row.id,
            row.session_id,
            row.kind,
            row.time_created,
            row.time_updated,
            data
        ],
    )?;
    Ok(())
}

/// Read a session_message row.
pub fn get_session_message(conn: &Connection, id: &str) -> StoreResult<Option<SessionMessageRow>> {
    conn.query_row(
        "SELECT id, session_id, type, time_created, time_updated, data
         FROM session_message WHERE id = ?1",
        params![id],
        session_message_from_row,
    )
    .optional()
    .map_err(StoreError::from)
}

/// List session_message rows for a session, optionally filtered by type.
pub fn list_session_messages(
    conn: &Connection,
    session_id: &str,
    kind: Option<&str>,
) -> StoreResult<Vec<SessionMessageRow>> {
    if let Some(kind) = kind {
        let mut stmt = conn.prepare(
            "SELECT id, session_id, type, time_created, time_updated, data
             FROM session_message WHERE session_id = ?1 AND type = ?2
             ORDER BY time_created ASC",
        )?;
        let rows = stmt.query_map(params![session_id, kind], session_message_from_row)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    } else {
        let mut stmt = conn.prepare(
            "SELECT id, session_id, type, time_created, time_updated, data
             FROM session_message WHERE session_id = ?1
             ORDER BY time_created ASC",
        )?;
        let rows = stmt.query_map(params![session_id], session_message_from_row)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }
}

fn session_message_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<SessionMessageRow> {
    let data_text: String = row.get(5)?;
    let data: serde_json::Value = serde_json::from_str(&data_text).map_err(|err| {
        rusqlite::Error::FromSqlConversionFailure(5, rusqlite::types::Type::Text, Box::new(err))
    })?;
    Ok(SessionMessageRow {
        id: row.get(0)?,
        session_id: row.get(1)?,
        kind: row.get(2)?,
        time_created: row.get(3)?,
        time_updated: row.get(4)?,
        data,
    })
}
