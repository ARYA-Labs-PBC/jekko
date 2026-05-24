use rusqlite::{params, Connection};

use crate::error::StoreResult;

use super::rows::DaemonRegressionCycleRow;
use crate::daemon::support::{collect_rows, parse_opt_json};

/// Insert or replace a regression cycle row.
pub fn upsert_regression_cycle(
    conn: &Connection,
    row: &DaemonRegressionCycleRow,
) -> StoreResult<()> {
    let result = row
        .result_json
        .as_ref()
        .map(serde_json::to_string)
        .transpose()?;
    conn.execute(
        "INSERT INTO daemon_regression_cycle (
            id, run_id, iteration, baseline_score, current_score, hard_delta,
            soft_delta, caps_delta, status, result_json, time_created, time_updated
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
        ON CONFLICT(id) DO UPDATE SET
            iteration = excluded.iteration,
            baseline_score = excluded.baseline_score,
            current_score = excluded.current_score,
            hard_delta = excluded.hard_delta,
            soft_delta = excluded.soft_delta,
            caps_delta = excluded.caps_delta,
            status = excluded.status,
            result_json = excluded.result_json,
            time_updated = excluded.time_updated",
        params![
            row.id,
            row.run_id,
            row.iteration,
            row.baseline_score,
            row.current_score,
            row.hard_delta,
            row.soft_delta,
            row.caps_delta,
            row.status,
            result,
            row.time_created,
            row.time_updated,
        ],
    )?;
    Ok(())
}

/// List regression cycles for a run.
pub fn list_regression_cycles_for_run(
    conn: &Connection,
    run_id: &str,
) -> StoreResult<Vec<DaemonRegressionCycleRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, run_id, iteration, baseline_score, current_score, hard_delta,
                soft_delta, caps_delta, status, result_json, time_created, time_updated
         FROM daemon_regression_cycle WHERE run_id = ?1 ORDER BY iteration ASC",
    )?;
    let rows = stmt.query_map(params![run_id], regression_cycle_from_row)?;
    collect_rows(rows)
}

fn regression_cycle_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<DaemonRegressionCycleRow> {
    let result_text: Option<String> = row.get(9)?;
    Ok(DaemonRegressionCycleRow {
        id: row.get(0)?,
        run_id: row.get(1)?,
        iteration: row.get(2)?,
        baseline_score: row.get(3)?,
        current_score: row.get(4)?,
        hard_delta: row.get(5)?,
        soft_delta: row.get(6)?,
        caps_delta: row.get(7)?,
        status: row.get(8)?,
        result_json: parse_opt_json(9, result_text)?,
        time_created: row.get(10)?,
        time_updated: row.get(11)?,
    })
}
