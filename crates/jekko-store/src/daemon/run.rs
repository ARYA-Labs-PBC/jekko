//! `daemon_run` table — one row per daemon invocation.

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::error::{StoreError, StoreResult};

/// Row in `daemon_run` — one entry per daemon invocation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DaemonRunRow {
    /// Run id.
    pub id: String,
    /// Root session id (the user-visible one).
    pub root_session_id: String,
    /// Currently active session id.
    pub active_session_id: String,
    /// High-level status.
    pub status: String,
    /// Current phase tag.
    pub phase: String,
    /// Spec snapshot (JSON).
    pub spec_json: serde_json::Value,
    /// Hash of the spec for quick equality checks.
    pub spec_hash: String,
    /// Iteration counter.
    pub iteration: i64,
    /// Epoch counter (bumped on full reset).
    pub epoch: i64,
    /// Last failure message, if any.
    pub last_error: Option<String>,
    /// Last exit-result payload (JSON), if any.
    pub last_exit_result_json: Option<serde_json::Value>,
    /// Stop timestamp (ms since epoch).
    pub stopped_at: Option<i64>,
    /// Creation timestamp (ms since epoch).
    pub time_created: i64,
    /// Last-update timestamp (ms since epoch).
    pub time_updated: i64,
}

/// Insert or replace a daemon_run row.
pub fn upsert_run(conn: &Connection, row: &DaemonRunRow) -> StoreResult<()> {
    let spec = serde_json::to_string(&row.spec_json)?;
    let last_exit = row
        .last_exit_result_json
        .as_ref()
        .map(serde_json::to_string)
        .transpose()?;
    conn.execute(
        "INSERT INTO daemon_run (
            id, root_session_id, active_session_id, status, phase, spec_json,
            spec_hash, iteration, epoch, last_error, last_exit_result_json,
            stopped_at, time_created, time_updated
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
        ON CONFLICT(id) DO UPDATE SET
            root_session_id = excluded.root_session_id,
            active_session_id = excluded.active_session_id,
            status = excluded.status,
            phase = excluded.phase,
            spec_json = excluded.spec_json,
            spec_hash = excluded.spec_hash,
            iteration = excluded.iteration,
            epoch = excluded.epoch,
            last_error = excluded.last_error,
            last_exit_result_json = excluded.last_exit_result_json,
            stopped_at = excluded.stopped_at,
            time_updated = excluded.time_updated",
        params![
            row.id,
            row.root_session_id,
            row.active_session_id,
            row.status,
            row.phase,
            spec,
            row.spec_hash,
            row.iteration,
            row.epoch,
            row.last_error,
            last_exit,
            row.stopped_at,
            row.time_created,
            row.time_updated,
        ],
    )?;
    Ok(())
}

/// Read a daemon_run row.
pub fn get_run(conn: &Connection, id: &str) -> StoreResult<Option<DaemonRunRow>> {
    conn.query_row(
        "SELECT id, root_session_id, active_session_id, status, phase, spec_json,
                spec_hash, iteration, epoch, last_error, last_exit_result_json,
                stopped_at, time_created, time_updated
         FROM daemon_run WHERE id = ?1",
        params![id],
        |row| {
            let spec_text: String = row.get(5)?;
            let last_exit_text: Option<String> = row.get(10)?;
            let spec_json = serde_json::from_str(&spec_text).map_err(|err| {
                rusqlite::Error::FromSqlConversionFailure(
                    5,
                    rusqlite::types::Type::Text,
                    Box::new(err),
                )
            })?;
            let last_exit_result_json = last_exit_text
                .as_deref()
                .map(serde_json::from_str)
                .transpose()
                .map_err(|err| {
                    rusqlite::Error::FromSqlConversionFailure(
                        10,
                        rusqlite::types::Type::Text,
                        Box::new(err),
                    )
                })?;
            Ok(DaemonRunRow {
                id: row.get(0)?,
                root_session_id: row.get(1)?,
                active_session_id: row.get(2)?,
                status: row.get(3)?,
                phase: row.get(4)?,
                spec_json,
                spec_hash: row.get(6)?,
                iteration: row.get(7)?,
                epoch: row.get(8)?,
                last_error: row.get(9)?,
                last_exit_result_json,
                stopped_at: row.get(11)?,
                time_created: row.get(12)?,
                time_updated: row.get(13)?,
            })
        },
    )
    .optional()
    .map_err(StoreError::from)
}

/// Delete a daemon_run row.
pub fn delete_run(conn: &Connection, id: &str) -> StoreResult<usize> {
    Ok(conn.execute("DELETE FROM daemon_run WHERE id = ?1", params![id])?)
}
