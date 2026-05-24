use rusqlite::{params, Connection};

use super::rows::{ParityCaseRow, ParityResultRow, ParityRunRow, PerfBudgetRow};
use crate::daemon::support::{collect_rows, parse_json, parse_opt_json, serialize_opt};
use crate::error::StoreResult;

/// Insert or replace a parity case row.
pub fn upsert_parity_case(conn: &Connection, row: &ParityCaseRow) -> StoreResult<()> {
    let tags = serde_json::to_string(&row.tags)?;
    let steps = serde_json::to_string(&row.steps_json)?;
    let perf = serialize_opt(&row.perf_json)?;
    conn.execute(
        "INSERT INTO daemon_parity_case (
            id, run_id, target_id, tags_json, target_kind, steps_json, perf_json,
            approved, time_created, time_updated
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
        ON CONFLICT(id) DO UPDATE SET
            tags_json = excluded.tags_json,
            target_kind = excluded.target_kind,
            steps_json = excluded.steps_json,
            perf_json = excluded.perf_json,
            approved = excluded.approved,
            time_updated = excluded.time_updated",
        params![
            row.id,
            row.run_id,
            row.target_id,
            tags,
            row.target_kind,
            steps,
            perf,
            row.approved as i64,
            row.time_created,
            row.time_updated,
        ],
    )?;
    Ok(())
}

/// List parity cases for a target.
pub fn list_parity_cases_for_target(
    conn: &Connection,
    target_id: &str,
) -> StoreResult<Vec<ParityCaseRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, run_id, target_id, tags_json, target_kind, steps_json, perf_json,
                approved, time_created, time_updated
         FROM daemon_parity_case WHERE target_id = ?1 ORDER BY id ASC",
    )?;
    let rows = stmt.query_map(params![target_id], parity_case_from_row)?;
    collect_rows(rows)
}

/// Insert or replace a parity run row.
pub fn upsert_parity_run(conn: &Connection, row: &ParityRunRow) -> StoreResult<()> {
    let summary = serialize_opt(&row.summary_json)?;
    conn.execute(
        "INSERT INTO daemon_parity_run (
            id, run_id, target_id, case_count, status, report_path, started_at,
            ended_at, summary_json, time_created, time_updated
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
        ON CONFLICT(id) DO UPDATE SET
            case_count = excluded.case_count,
            status = excluded.status,
            report_path = excluded.report_path,
            started_at = excluded.started_at,
            ended_at = excluded.ended_at,
            summary_json = excluded.summary_json,
            time_updated = excluded.time_updated",
        params![
            row.id,
            row.run_id,
            row.target_id,
            row.case_count,
            row.status,
            row.report_path,
            row.started_at,
            row.ended_at,
            summary,
            row.time_created,
            row.time_updated,
        ],
    )?;
    Ok(())
}

/// List parity runs for a target.
pub fn list_parity_runs_for_target(
    conn: &Connection,
    target_id: &str,
) -> StoreResult<Vec<ParityRunRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, run_id, target_id, case_count, status, report_path, started_at,
                ended_at, summary_json, time_created, time_updated
         FROM daemon_parity_run WHERE target_id = ?1 ORDER BY time_created DESC, id ASC",
    )?;
    let rows = stmt.query_map(params![target_id], parity_run_from_row)?;
    collect_rows(rows)
}

/// Insert a parity result row.
pub fn insert_parity_result(conn: &Connection, row: &ParityResultRow) -> StoreResult<()> {
    let perf = serialize_opt(&row.perf_json)?;
    conn.execute(
        "INSERT INTO daemon_parity_result (
            id, parity_run_id, case_id, target_name, status, skipped, duration_ms,
            perf_json, message, time_created
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            row.id,
            row.parity_run_id,
            row.case_id,
            row.target_name,
            row.status,
            row.skipped as i64,
            row.duration_ms,
            perf,
            row.message,
            row.time_created,
        ],
    )?;
    Ok(())
}

/// List parity results for a parity run.
pub fn list_parity_results_for_run(
    conn: &Connection,
    parity_run_id: &str,
) -> StoreResult<Vec<ParityResultRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, parity_run_id, case_id, target_name, status, skipped, duration_ms,
                perf_json, message, time_created
         FROM daemon_parity_result WHERE parity_run_id = ?1 ORDER BY case_id ASC, target_name ASC",
    )?;
    let rows = stmt.query_map(params![parity_run_id], parity_result_from_row)?;
    collect_rows(rows)
}

/// Insert or replace a performance budget row.
pub fn upsert_perf_budget(conn: &Connection, row: &PerfBudgetRow) -> StoreResult<()> {
    conn.execute(
        "INSERT INTO daemon_perf_budget (
            id, run_id, case_id, metric, max_ratio, baseline_value, candidate_value,
            status, time_created, time_updated
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
        ON CONFLICT(id) DO UPDATE SET
            metric = excluded.metric,
            max_ratio = excluded.max_ratio,
            baseline_value = excluded.baseline_value,
            candidate_value = excluded.candidate_value,
            status = excluded.status,
            time_updated = excluded.time_updated",
        params![
            row.id,
            row.run_id,
            row.case_id,
            row.metric,
            row.max_ratio,
            row.baseline_value,
            row.candidate_value,
            row.status,
            row.time_created,
            row.time_updated,
        ],
    )?;
    Ok(())
}

fn parity_case_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ParityCaseRow> {
    let tags_text: String = row.get(3)?;
    let steps_text: String = row.get(5)?;
    let perf_text: Option<String> = row.get(6)?;
    let approved: i64 = row.get(7)?;
    Ok(ParityCaseRow {
        id: row.get(0)?,
        run_id: row.get(1)?,
        target_id: row.get(2)?,
        tags: parse_json(3, &tags_text)?,
        target_kind: row.get(4)?,
        steps_json: parse_json(5, &steps_text)?,
        perf_json: parse_opt_json(6, perf_text)?,
        approved: approved != 0,
        time_created: row.get(8)?,
        time_updated: row.get(9)?,
    })
}

fn parity_result_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ParityResultRow> {
    let perf_text: Option<String> = row.get(7)?;
    let skipped: i64 = row.get(5)?;
    Ok(ParityResultRow {
        id: row.get(0)?,
        parity_run_id: row.get(1)?,
        case_id: row.get(2)?,
        target_name: row.get(3)?,
        status: row.get(4)?,
        skipped: skipped != 0,
        duration_ms: row.get(6)?,
        perf_json: parse_opt_json(7, perf_text)?,
        message: row.get(8)?,
        time_created: row.get(9)?,
    })
}

fn parity_run_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ParityRunRow> {
    let summary_text: Option<String> = row.get(8)?;
    Ok(ParityRunRow {
        id: row.get(0)?,
        run_id: row.get(1)?,
        target_id: row.get(2)?,
        case_count: row.get(3)?,
        status: row.get(4)?,
        report_path: row.get(5)?,
        started_at: row.get(6)?,
        ended_at: row.get(7)?,
        summary_json: parse_opt_json(8, summary_text)?,
        time_created: row.get(9)?,
        time_updated: row.get(10)?,
    })
}
