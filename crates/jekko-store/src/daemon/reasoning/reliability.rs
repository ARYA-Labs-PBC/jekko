use rusqlite::{params, Connection, OptionalExtension};

use crate::error::{StoreError, StoreResult};

use super::rows::ModelReliabilityRow;
use crate::daemon::support::collect_rows;

/// Insert or replace model reliability counters.
pub fn upsert_model_reliability(conn: &Connection, row: &ModelReliabilityRow) -> StoreResult<()> {
    conn.execute(
        "INSERT INTO daemon_model_reliability (
            model_id, role, task_kind, success_count, failure_count, winner_count,
            total_latency_ms, total_cost_usd, score, time_created, time_updated
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
        ON CONFLICT(model_id, role, task_kind) DO UPDATE SET
            success_count = excluded.success_count,
            failure_count = excluded.failure_count,
            winner_count = excluded.winner_count,
            total_latency_ms = excluded.total_latency_ms,
            total_cost_usd = excluded.total_cost_usd,
            score = excluded.score,
            time_updated = excluded.time_updated",
        params![
            row.model_id,
            row.role,
            row.task_kind,
            row.success_count,
            row.failure_count,
            row.winner_count,
            row.total_latency_ms,
            row.total_cost_usd,
            row.score,
            row.time_created,
            row.time_updated,
        ],
    )?;
    Ok(())
}

/// Add one model outcome to reliability counters.
#[allow(clippy::too_many_arguments)]
pub fn record_model_reliability_outcome(
    conn: &Connection,
    model_id: &str,
    role: &str,
    task_kind: &str,
    success: bool,
    winner: bool,
    latency_ms: i64,
    cost_usd: f64,
    now: i64,
) -> StoreResult<()> {
    let mut row =
        get_model_reliability(conn, model_id, role, task_kind)?.unwrap_or(ModelReliabilityRow {
            model_id: model_id.to_string(),
            role: role.to_string(),
            task_kind: task_kind.to_string(),
            success_count: 0,
            failure_count: 0,
            winner_count: 0,
            total_latency_ms: 0,
            total_cost_usd: 0.0,
            score: 0.0,
            time_created: now,
            time_updated: now,
        });
    if success {
        row.success_count += 1;
    } else {
        row.failure_count += 1;
    }
    if winner {
        row.winner_count += 1;
    }
    row.total_latency_ms = row.total_latency_ms.saturating_add(latency_ms.max(0));
    row.total_cost_usd += cost_usd.max(0.0);
    row.score = model_reliability_score(&row);
    row.time_updated = now;
    upsert_model_reliability(conn, &row)
}

/// Read one reliability row.
pub fn get_model_reliability(
    conn: &Connection,
    model_id: &str,
    role: &str,
    task_kind: &str,
) -> StoreResult<Option<ModelReliabilityRow>> {
    conn.query_row(
        "SELECT model_id, role, task_kind, success_count, failure_count, winner_count,
                total_latency_ms, total_cost_usd, score, time_created, time_updated
         FROM daemon_model_reliability
         WHERE model_id = ?1 AND role = ?2 AND task_kind = ?3",
        params![model_id, role, task_kind],
        model_reliability_from_row,
    )
    .optional()
    .map_err(StoreError::from)
}

/// List model reliability rows for a task kind. Empty task kind lists all rows.
pub fn list_model_reliability(
    conn: &Connection,
    task_kind: Option<&str>,
) -> StoreResult<Vec<ModelReliabilityRow>> {
    if let Some(task_kind) = task_kind {
        let mut stmt = conn.prepare(
            "SELECT model_id, role, task_kind, success_count, failure_count, winner_count,
                    total_latency_ms, total_cost_usd, score, time_created, time_updated
             FROM daemon_model_reliability WHERE task_kind = ?1 ORDER BY score DESC, model_id ASC",
        )?;
        let rows = stmt.query_map(params![task_kind], model_reliability_from_row)?;
        return collect_rows(rows);
    }
    let mut stmt = conn.prepare(
        "SELECT model_id, role, task_kind, success_count, failure_count, winner_count,
                total_latency_ms, total_cost_usd, score, time_created, time_updated
         FROM daemon_model_reliability ORDER BY score DESC, task_kind ASC, model_id ASC",
    )?;
    let rows = stmt.query_map([], model_reliability_from_row)?;
    collect_rows(rows)
}

fn model_reliability_score(row: &ModelReliabilityRow) -> f64 {
    let total = row.success_count + row.failure_count;
    if total <= 0 {
        return 0.0;
    }
    let success_rate = row.success_count as f64 / total as f64;
    let winner_bonus = row.winner_count as f64 / total as f64 * 0.15;
    (success_rate + winner_bonus).clamp(0.0, 1.0)
}

fn model_reliability_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ModelReliabilityRow> {
    Ok(ModelReliabilityRow {
        model_id: row.get(0)?,
        role: row.get(1)?,
        task_kind: row.get(2)?,
        success_count: row.get(3)?,
        failure_count: row.get(4)?,
        winner_count: row.get(5)?,
        total_latency_ms: row.get(6)?,
        total_cost_usd: row.get(7)?,
        score: row.get(8)?,
        time_created: row.get(9)?,
        time_updated: row.get(10)?,
    })
}
