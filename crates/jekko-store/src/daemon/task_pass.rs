//! `daemon_task_pass` table — one row per attempt against a daemon task.

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::error::{StoreError, StoreResult};

/// Row in `daemon_task_pass`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DaemonTaskPassRow {
    /// Pass id.
    pub id: String,
    /// FK to `daemon_run.id`.
    pub run_id: String,
    /// FK to `daemon_task.id`.
    pub task_id: String,
    /// Pass number within the task.
    pub pass_number: i64,
    /// Pass type tag.
    pub pass_type: String,
    /// Context mode tag.
    pub context_mode: String,
    /// Agent name driving the pass.
    pub agent: Option<String>,
    /// FK to `session.id`.
    pub session_id: Option<String>,
    /// Worker id.
    pub worker_id: Option<String>,
    /// Status tag.
    pub status: String,
    /// Start timestamp (ms).
    pub started_at: Option<i64>,
    /// End timestamp (ms).
    pub ended_at: Option<i64>,
    /// Worktree path.
    pub worktree_path: Option<String>,
    /// Worktree branch.
    pub worktree_branch: Option<String>,
    /// Cleanup status tag.
    pub cleanup_status: String,
    /// Input artifact ids (JSON), if any.
    pub input_artifact_ids_json: Option<serde_json::Value>,
    /// Output artifact ids (JSON), if any.
    pub output_artifact_ids_json: Option<serde_json::Value>,
    /// Result payload (JSON), if any.
    pub result_json: Option<serde_json::Value>,
    /// Score payload (JSON), if any.
    pub score_json: Option<serde_json::Value>,
    /// Error payload (JSON), if any.
    pub error_json: Option<serde_json::Value>,
    /// Creation timestamp (ms since epoch).
    pub time_created: i64,
    /// Last-update timestamp (ms since epoch).
    pub time_updated: i64,
}

/// Insert or replace a daemon_task_pass row.
pub fn upsert_task_pass(conn: &Connection, row: &DaemonTaskPassRow) -> StoreResult<()> {
    let input = row
        .input_artifact_ids_json
        .as_ref()
        .map(serde_json::to_string)
        .transpose()?;
    let output = row
        .output_artifact_ids_json
        .as_ref()
        .map(serde_json::to_string)
        .transpose()?;
    let result = row
        .result_json
        .as_ref()
        .map(serde_json::to_string)
        .transpose()?;
    let score = row
        .score_json
        .as_ref()
        .map(serde_json::to_string)
        .transpose()?;
    let error = row
        .error_json
        .as_ref()
        .map(serde_json::to_string)
        .transpose()?;

    conn.execute(
        "INSERT INTO daemon_task_pass (
            id, run_id, task_id, pass_number, pass_type, context_mode, agent,
            session_id, worker_id, status, started_at, ended_at, worktree_path,
            worktree_branch, cleanup_status, input_artifact_ids_json,
            output_artifact_ids_json, result_json, score_json, error_json,
            time_created, time_updated
        ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14,
            ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22
        )
        ON CONFLICT(id) DO UPDATE SET
            pass_number = excluded.pass_number,
            pass_type = excluded.pass_type,
            context_mode = excluded.context_mode,
            agent = excluded.agent,
            session_id = excluded.session_id,
            worker_id = excluded.worker_id,
            status = excluded.status,
            started_at = excluded.started_at,
            ended_at = excluded.ended_at,
            worktree_path = excluded.worktree_path,
            worktree_branch = excluded.worktree_branch,
            cleanup_status = excluded.cleanup_status,
            input_artifact_ids_json = excluded.input_artifact_ids_json,
            output_artifact_ids_json = excluded.output_artifact_ids_json,
            result_json = excluded.result_json,
            score_json = excluded.score_json,
            error_json = excluded.error_json,
            time_updated = excluded.time_updated",
        params![
            row.id,
            row.run_id,
            row.task_id,
            row.pass_number,
            row.pass_type,
            row.context_mode,
            row.agent,
            row.session_id,
            row.worker_id,
            row.status,
            row.started_at,
            row.ended_at,
            row.worktree_path,
            row.worktree_branch,
            row.cleanup_status,
            input,
            output,
            result,
            score,
            error,
            row.time_created,
            row.time_updated,
        ],
    )?;
    Ok(())
}

/// Read a daemon_task_pass row.
pub fn get_task_pass(conn: &Connection, id: &str) -> StoreResult<Option<DaemonTaskPassRow>> {
    conn.query_row(
        "SELECT id, run_id, task_id, pass_number, pass_type, context_mode, agent,
                session_id, worker_id, status, started_at, ended_at, worktree_path,
                worktree_branch, cleanup_status, input_artifact_ids_json,
                output_artifact_ids_json, result_json, score_json, error_json,
                time_created, time_updated
         FROM daemon_task_pass WHERE id = ?1",
        params![id],
        |row| {
            let json_opt =
                |idx: usize, text: Option<String>| -> rusqlite::Result<Option<serde_json::Value>> {
                    text.as_deref()
                        .map(serde_json::from_str)
                        .transpose()
                        .map_err(|err| {
                            rusqlite::Error::FromSqlConversionFailure(
                                idx,
                                rusqlite::types::Type::Text,
                                Box::new(err),
                            )
                        })
                };
            let input_text: Option<String> = row.get(15)?;
            let output_text: Option<String> = row.get(16)?;
            let result_text: Option<String> = row.get(17)?;
            let score_text: Option<String> = row.get(18)?;
            let error_text: Option<String> = row.get(19)?;
            Ok(DaemonTaskPassRow {
                id: row.get(0)?,
                run_id: row.get(1)?,
                task_id: row.get(2)?,
                pass_number: row.get(3)?,
                pass_type: row.get(4)?,
                context_mode: row.get(5)?,
                agent: row.get(6)?,
                session_id: row.get(7)?,
                worker_id: row.get(8)?,
                status: row.get(9)?,
                started_at: row.get(10)?,
                ended_at: row.get(11)?,
                worktree_path: row.get(12)?,
                worktree_branch: row.get(13)?,
                cleanup_status: row.get(14)?,
                input_artifact_ids_json: json_opt(15, input_text)?,
                output_artifact_ids_json: json_opt(16, output_text)?,
                result_json: json_opt(17, result_text)?,
                score_json: json_opt(18, score_text)?,
                error_json: json_opt(19, error_text)?,
                time_created: row.get(20)?,
                time_updated: row.get(21)?,
            })
        },
    )
    .optional()
    .map_err(StoreError::from)
}
