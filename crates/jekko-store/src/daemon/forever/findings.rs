use rusqlite::{params, Connection, OptionalExtension};

use crate::error::{StoreError, StoreResult};

use super::rows::{DaemonFindingBatchRow, DaemonFindingEdgeRow, DaemonFindingRow};
use crate::daemon::support::{collect_rows, parse_json, parse_opt_json};

/// Insert or replace a `daemon_finding` row.
pub fn upsert_finding(conn: &Connection, row: &DaemonFindingRow) -> StoreResult<()> {
    let paths = serde_json::to_string(&row.paths)?;
    conn.execute(
        "INSERT INTO daemon_finding (
            id, run_id, iteration, rule_id, fingerprint, severity, paths_json,
            cap, status, attempt_count, batch_id, last_error, time_created, time_updated
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
        ON CONFLICT(id) DO UPDATE SET
            iteration = excluded.iteration,
            rule_id = excluded.rule_id,
            fingerprint = excluded.fingerprint,
            severity = excluded.severity,
            paths_json = excluded.paths_json,
            cap = excluded.cap,
            status = excluded.status,
            attempt_count = excluded.attempt_count,
            batch_id = excluded.batch_id,
            last_error = excluded.last_error,
            time_updated = excluded.time_updated",
        params![
            row.id,
            row.run_id,
            row.iteration,
            row.rule_id,
            row.fingerprint,
            row.severity,
            paths,
            row.cap,
            row.status,
            row.attempt_count,
            row.batch_id,
            row.last_error,
            row.time_created,
            row.time_updated,
        ],
    )?;
    Ok(())
}

/// Read a `daemon_finding` row.
pub fn get_finding(conn: &Connection, id: &str) -> StoreResult<Option<DaemonFindingRow>> {
    conn.query_row(
        "SELECT id, run_id, iteration, rule_id, fingerprint, severity, paths_json,
                cap, status, attempt_count, batch_id, last_error, time_created, time_updated
         FROM daemon_finding WHERE id = ?1",
        params![id],
        finding_from_row,
    )
    .optional()
    .map_err(StoreError::from)
}

/// List findings for a run.
pub fn list_findings_for_run(
    conn: &Connection,
    run_id: &str,
) -> StoreResult<Vec<DaemonFindingRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, run_id, iteration, rule_id, fingerprint, severity, paths_json,
                cap, status, attempt_count, batch_id, last_error, time_created, time_updated
         FROM daemon_finding WHERE run_id = ?1 ORDER BY iteration ASC, id ASC",
    )?;
    let rows = stmt.query_map(params![run_id], finding_from_row)?;
    collect_rows(rows)
}

/// Insert or replace a `daemon_finding_batch` row.
pub fn upsert_finding_batch(conn: &Connection, row: &DaemonFindingBatchRow) -> StoreResult<()> {
    let result = row
        .result_json
        .as_ref()
        .map(serde_json::to_string)
        .transpose()?;
    conn.execute(
        "INSERT INTO daemon_finding_batch (
            id, run_id, wave_index, lane, worker_id, status, started_at,
            ended_at, result_json, time_created, time_updated
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
        ON CONFLICT(id) DO UPDATE SET
            wave_index = excluded.wave_index,
            lane = excluded.lane,
            worker_id = excluded.worker_id,
            status = excluded.status,
            started_at = excluded.started_at,
            ended_at = excluded.ended_at,
            result_json = excluded.result_json,
            time_updated = excluded.time_updated",
        params![
            row.id,
            row.run_id,
            row.wave_index,
            row.lane,
            row.worker_id,
            row.status,
            row.started_at,
            row.ended_at,
            result,
            row.time_created,
            row.time_updated,
        ],
    )?;
    Ok(())
}

/// List finding batches for a run.
pub fn list_finding_batches_for_run(
    conn: &Connection,
    run_id: &str,
) -> StoreResult<Vec<DaemonFindingBatchRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, run_id, wave_index, lane, worker_id, status, started_at,
                ended_at, result_json, time_created, time_updated
         FROM daemon_finding_batch WHERE run_id = ?1 ORDER BY wave_index ASC, id ASC",
    )?;
    let rows = stmt.query_map(params![run_id], finding_batch_from_row)?;
    collect_rows(rows)
}

/// Insert or replace a `daemon_finding_edge` row.
pub fn upsert_finding_edge(conn: &Connection, row: &DaemonFindingEdgeRow) -> StoreResult<()> {
    conn.execute(
        "INSERT OR REPLACE INTO daemon_finding_edge
         (run_id, parent_id, child_id, kind, time_created)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            row.run_id,
            row.parent_id,
            row.child_id,
            row.kind,
            row.time_created,
        ],
    )?;
    Ok(())
}

/// List finding edges for a run.
pub fn list_finding_edges_for_run(
    conn: &Connection,
    run_id: &str,
) -> StoreResult<Vec<DaemonFindingEdgeRow>> {
    let mut stmt = conn.prepare(
        "SELECT run_id, parent_id, child_id, kind, time_created
         FROM daemon_finding_edge WHERE run_id = ?1 ORDER BY parent_id ASC, child_id ASC",
    )?;
    let rows = stmt.query_map(params![run_id], |row| {
        Ok(DaemonFindingEdgeRow {
            run_id: row.get(0)?,
            parent_id: row.get(1)?,
            child_id: row.get(2)?,
            kind: row.get(3)?,
            time_created: row.get(4)?,
        })
    })?;
    collect_rows(rows)
}

fn finding_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<DaemonFindingRow> {
    let paths_text: String = row.get(6)?;
    Ok(DaemonFindingRow {
        id: row.get(0)?,
        run_id: row.get(1)?,
        iteration: row.get(2)?,
        rule_id: row.get(3)?,
        fingerprint: row.get(4)?,
        severity: row.get(5)?,
        paths: parse_json(6, &paths_text)?,
        cap: row.get(7)?,
        status: row.get(8)?,
        attempt_count: row.get(9)?,
        batch_id: row.get(10)?,
        last_error: row.get(11)?,
        time_created: row.get(12)?,
        time_updated: row.get(13)?,
    })
}

fn finding_batch_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<DaemonFindingBatchRow> {
    let result_text: Option<String> = row.get(8)?;
    Ok(DaemonFindingBatchRow {
        id: row.get(0)?,
        run_id: row.get(1)?,
        wave_index: row.get(2)?,
        lane: row.get(3)?,
        worker_id: row.get(4)?,
        status: row.get(5)?,
        started_at: row.get(6)?,
        ended_at: row.get(7)?,
        result_json: parse_opt_json(8, result_text)?,
        time_created: row.get(9)?,
        time_updated: row.get(10)?,
    })
}
