//! `daemon_worker` table — one row per worker session in a run.

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use crate::error::StoreResult;

/// Row in `daemon_worker`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DaemonWorkerRow {
    /// Worker id.
    pub id: String,
    /// FK to `daemon_run.id`.
    pub run_id: String,
    /// Role tag.
    pub role: String,
    /// FK to `session.id`.
    pub session_id: Option<String>,
    /// Worktree path.
    pub worktree_path: Option<String>,
    /// Worktree branch.
    pub branch: Option<String>,
    /// Status tag.
    pub status: String,
    /// FK to `daemon_task.id` currently leased.
    pub lease_task_id: Option<String>,
    /// Last heartbeat timestamp (ms).
    pub last_heartbeat_at: Option<i64>,
    /// Pool id.
    pub pool_id: Option<String>,
    /// Batch id.
    pub batch_id: Option<String>,
    /// Last commit sha.
    pub last_commit_sha: Option<String>,
    /// Creation timestamp (ms since epoch).
    pub time_created: i64,
    /// Last-update timestamp (ms since epoch).
    pub time_updated: i64,
}

/// Insert or replace a daemon_worker row.
pub fn upsert_worker(conn: &Connection, row: &DaemonWorkerRow) -> StoreResult<()> {
    conn.execute(
        "INSERT INTO daemon_worker (
            id, run_id, role, session_id, worktree_path, branch, status,
            lease_task_id, last_heartbeat_at, pool_id, batch_id, last_commit_sha,
            time_created, time_updated
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
        ON CONFLICT(id) DO UPDATE SET
            role = excluded.role,
            session_id = excluded.session_id,
            worktree_path = excluded.worktree_path,
            branch = excluded.branch,
            status = excluded.status,
            lease_task_id = excluded.lease_task_id,
            last_heartbeat_at = excluded.last_heartbeat_at,
            pool_id = excluded.pool_id,
            batch_id = excluded.batch_id,
            last_commit_sha = excluded.last_commit_sha,
            time_updated = excluded.time_updated",
        params![
            row.id,
            row.run_id,
            row.role,
            row.session_id,
            row.worktree_path,
            row.branch,
            row.status,
            row.lease_task_id,
            row.last_heartbeat_at,
            row.pool_id,
            row.batch_id,
            row.last_commit_sha,
            row.time_created,
            row.time_updated,
        ],
    )?;
    Ok(())
}
