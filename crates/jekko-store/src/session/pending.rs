//! `pending` table CRUD.

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use crate::error::StoreResult;

/// Row in the `pending` table.
///
/// Composite primary key: (`session_id`, `position`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingRow {
    /// FK to `session.id`.
    pub session_id: String,
    /// Pending item text.
    pub content: String,
    /// Status tag (`pending`, `in_progress`, `completed`, …).
    pub status: String,
    /// Priority tag.
    pub priority: String,
    /// Position within the list.
    pub position: i64,
    /// Creation timestamp (ms since epoch).
    pub time_created: i64,
    /// Last-update timestamp (ms since epoch).
    pub time_updated: i64,
}

/// Insert or replace a pending row.
pub fn upsert_pending(conn: &Connection, row: &PendingRow) -> StoreResult<()> {
    conn.execute(
        "INSERT INTO pending (session_id, content, status, priority, position, time_created, time_updated)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT(session_id, position) DO UPDATE SET
            content = excluded.content,
            status = excluded.status,
            priority = excluded.priority,
            time_updated = excluded.time_updated",
        params![
            row.session_id,
            row.content,
            row.status,
            row.priority,
            row.position,
            row.time_created,
            row.time_updated,
        ],
    )?;
    Ok(())
}

/// List pending items for a session.
pub fn list_pending(conn: &Connection, session_id: &str) -> StoreResult<Vec<PendingRow>> {
    let mut stmt = conn.prepare(
        "SELECT session_id, content, status, priority, position, time_created, time_updated
         FROM pending WHERE session_id = ?1 ORDER BY position ASC",
    )?;
    let rows = stmt.query_map(params![session_id], |row| {
        Ok(PendingRow {
            session_id: row.get(0)?,
            content: row.get(1)?,
            status: row.get(2)?,
            priority: row.get(3)?,
            position: row.get(4)?,
            time_created: row.get(5)?,
            time_updated: row.get(6)?,
        })
    })?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

/// Delete pending rows for a session/position.
pub fn delete_pending(conn: &Connection, session_id: &str, position: i64) -> StoreResult<usize> {
    Ok(conn.execute(
        "DELETE FROM pending WHERE session_id = ?1 AND position = ?2",
        params![session_id, position],
    )?)
}
