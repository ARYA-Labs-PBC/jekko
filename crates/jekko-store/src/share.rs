//! `session_share` CRUD.
//!
//! Ported from `packages/jekko/src/share/share.sql.ts`. One row per session
//! (`session_id` is the primary key).

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::error::{StoreError, StoreResult};

/// Row in the `session_share` table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionShareRow {
    /// FK to `session.id` (also the primary key).
    pub session_id: String,
    /// Public id of the share.
    pub id: String,
    /// Secret token granting write/edit access.
    pub secret: String,
    /// Public URL.
    pub url: String,
    /// Creation timestamp (ms since epoch).
    pub time_created: i64,
    /// Last-update timestamp (ms since epoch).
    pub time_updated: i64,
}

/// Insert or replace a session share row.
pub fn upsert(conn: &Connection, row: &SessionShareRow) -> StoreResult<()> {
    conn.execute(
        "INSERT INTO session_share (session_id, id, secret, url, time_created, time_updated)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(session_id) DO UPDATE SET
            id = excluded.id,
            secret = excluded.secret,
            url = excluded.url,
            time_updated = excluded.time_updated",
        params![
            row.session_id,
            row.id,
            row.secret,
            row.url,
            row.time_created,
            row.time_updated,
        ],
    )?;
    Ok(())
}

/// Read a session share row by `session_id`.
pub fn get(conn: &Connection, session_id: &str) -> StoreResult<Option<SessionShareRow>> {
    conn.query_row(
        "SELECT session_id, id, secret, url, time_created, time_updated
         FROM session_share WHERE session_id = ?1",
        params![session_id],
        |row| {
            Ok(SessionShareRow {
                session_id: row.get(0)?,
                id: row.get(1)?,
                secret: row.get(2)?,
                url: row.get(3)?,
                time_created: row.get(4)?,
                time_updated: row.get(5)?,
            })
        },
    )
    .optional()
    .map_err(StoreError::from)
}

/// Delete a session share row.
pub fn delete(conn: &Connection, session_id: &str) -> StoreResult<usize> {
    Ok(conn.execute(
        "DELETE FROM session_share WHERE session_id = ?1",
        params![session_id],
    )?)
}
