//! `daemon_task_memory` table — distilled memories attached to a task.

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use crate::error::StoreResult;

/// Row in `daemon_task_memory`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DaemonTaskMemoryRow {
    /// Memory id.
    pub id: String,
    /// FK to `daemon_run.id`.
    pub run_id: String,
    /// FK to `daemon_task.id`.
    pub task_id: String,
    /// Memory kind tag.
    pub kind: String,
    /// Title.
    pub title: String,
    /// Summary text.
    pub summary: String,
    /// Optional structured payload (JSON).
    pub payload_json: Option<serde_json::Value>,
    /// Source pass id.
    pub source_pass_id: Option<String>,
    /// Importance score 0..1.
    pub importance: f64,
    /// Confidence score 0..1.
    pub confidence: f64,
    /// Creation timestamp (ms since epoch).
    pub time_created: i64,
    /// Last-update timestamp (ms since epoch).
    pub time_updated: i64,
}

/// Insert or replace a daemon_task_memory row.
pub fn upsert_task_memory(conn: &Connection, row: &DaemonTaskMemoryRow) -> StoreResult<()> {
    let payload = row
        .payload_json
        .as_ref()
        .map(serde_json::to_string)
        .transpose()?;
    conn.execute(
        "INSERT INTO daemon_task_memory (
            id, run_id, task_id, kind, title, summary, payload_json,
            source_pass_id, importance, confidence, time_created, time_updated
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
        ON CONFLICT(id) DO UPDATE SET
            kind = excluded.kind,
            title = excluded.title,
            summary = excluded.summary,
            payload_json = excluded.payload_json,
            source_pass_id = excluded.source_pass_id,
            importance = excluded.importance,
            confidence = excluded.confidence,
            time_updated = excluded.time_updated",
        params![
            row.id,
            row.run_id,
            row.task_id,
            row.kind,
            row.title,
            row.summary,
            payload,
            row.source_pass_id,
            row.importance,
            row.confidence,
            row.time_created,
            row.time_updated,
        ],
    )?;
    Ok(())
}
