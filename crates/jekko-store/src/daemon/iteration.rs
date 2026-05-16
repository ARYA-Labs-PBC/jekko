//! `daemon_iteration` table — one row per daemon iteration.

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::error::{StoreError, StoreResult};

/// Row in `daemon_iteration`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DaemonIterationRow {
    /// FK to `daemon_run.id`.
    pub run_id: String,
    /// Iteration number within the run.
    pub iteration: i64,
    /// Session id used for this iteration.
    pub session_id: String,
    /// Terminal reason tag.
    pub terminal_reason: String,
    /// Result payload (JSON).
    pub result_json: serde_json::Value,
    /// Token-usage payload (JSON), if any.
    pub token_usage_json: Option<serde_json::Value>,
    /// Computed cost.
    pub cost: Option<f64>,
    /// Checkpoint commit sha.
    pub checkpoint_sha: Option<String>,
    /// Creation timestamp (ms since epoch).
    pub time_created: i64,
    /// Last-update timestamp (ms since epoch).
    pub time_updated: i64,
}

/// Insert or replace a daemon_iteration row.
pub fn upsert_iteration(conn: &Connection, row: &DaemonIterationRow) -> StoreResult<()> {
    let result = serde_json::to_string(&row.result_json)?;
    let token = row
        .token_usage_json
        .as_ref()
        .map(serde_json::to_string)
        .transpose()?;
    conn.execute(
        "INSERT INTO daemon_iteration (
            run_id, iteration, session_id, terminal_reason, result_json,
            token_usage_json, cost, checkpoint_sha, time_created, time_updated
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
        ON CONFLICT(run_id, iteration) DO UPDATE SET
            session_id = excluded.session_id,
            terminal_reason = excluded.terminal_reason,
            result_json = excluded.result_json,
            token_usage_json = excluded.token_usage_json,
            cost = excluded.cost,
            checkpoint_sha = excluded.checkpoint_sha,
            time_updated = excluded.time_updated",
        params![
            row.run_id,
            row.iteration,
            row.session_id,
            row.terminal_reason,
            result,
            token,
            row.cost,
            row.checkpoint_sha,
            row.time_created,
            row.time_updated,
        ],
    )?;
    Ok(())
}

/// Read a daemon_iteration row.
pub fn get_iteration(
    conn: &Connection,
    run_id: &str,
    iteration: i64,
) -> StoreResult<Option<DaemonIterationRow>> {
    conn.query_row(
        "SELECT run_id, iteration, session_id, terminal_reason, result_json,
                token_usage_json, cost, checkpoint_sha, time_created, time_updated
         FROM daemon_iteration WHERE run_id = ?1 AND iteration = ?2",
        params![run_id, iteration],
        |row| {
            let result_text: String = row.get(4)?;
            let token_text: Option<String> = row.get(5)?;
            let result_json = serde_json::from_str(&result_text).map_err(|err| {
                rusqlite::Error::FromSqlConversionFailure(
                    4,
                    rusqlite::types::Type::Text,
                    Box::new(err),
                )
            })?;
            let token_usage_json = token_text
                .as_deref()
                .map(serde_json::from_str)
                .transpose()
                .map_err(|err| {
                    rusqlite::Error::FromSqlConversionFailure(
                        5,
                        rusqlite::types::Type::Text,
                        Box::new(err),
                    )
                })?;
            Ok(DaemonIterationRow {
                run_id: row.get(0)?,
                iteration: row.get(1)?,
                session_id: row.get(2)?,
                terminal_reason: row.get(3)?,
                result_json,
                token_usage_json,
                cost: row.get(6)?,
                checkpoint_sha: row.get(7)?,
                time_created: row.get(8)?,
                time_updated: row.get(9)?,
            })
        },
    )
    .optional()
    .map_err(StoreError::from)
}
