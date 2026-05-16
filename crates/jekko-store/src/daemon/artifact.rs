//! `daemon_artifact` table — one row per artifact produced in a run.

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use crate::error::StoreResult;

/// Row in `daemon_artifact`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DaemonArtifactRow {
    /// Artifact id.
    pub id: String,
    /// FK to `daemon_run.id`.
    pub run_id: String,
    /// FK to `daemon_task.id`, if any.
    pub task_id: Option<String>,
    /// FK to `daemon_task_pass.id`, if any.
    pub pass_id: Option<String>,
    /// Artifact kind tag.
    pub kind: String,
    /// Path or reference string.
    pub path_or_ref: String,
    /// Optional commit sha.
    pub sha: Option<String>,
    /// Optional structured payload (JSON).
    pub payload_json: Option<serde_json::Value>,
    /// Creation timestamp (ms since epoch).
    pub time_created: i64,
    /// Last-update timestamp (ms since epoch).
    pub time_updated: i64,
}

/// Insert or replace a daemon_artifact row.
pub fn upsert_artifact(conn: &Connection, row: &DaemonArtifactRow) -> StoreResult<()> {
    let payload = row
        .payload_json
        .as_ref()
        .map(serde_json::to_string)
        .transpose()?;
    conn.execute(
        "INSERT INTO daemon_artifact (
            id, run_id, task_id, pass_id, kind, path_or_ref, sha, payload_json,
            time_created, time_updated
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
        ON CONFLICT(id) DO UPDATE SET
            task_id = excluded.task_id,
            pass_id = excluded.pass_id,
            kind = excluded.kind,
            path_or_ref = excluded.path_or_ref,
            sha = excluded.sha,
            payload_json = excluded.payload_json,
            time_updated = excluded.time_updated",
        params![
            row.id,
            row.run_id,
            row.task_id,
            row.pass_id,
            row.kind,
            row.path_or_ref,
            row.sha,
            payload,
            row.time_created,
            row.time_updated,
        ],
    )?;
    Ok(())
}
