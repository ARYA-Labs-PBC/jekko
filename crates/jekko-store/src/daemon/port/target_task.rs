use rusqlite::{params, Connection, OptionalExtension};

use super::rows::{PortPhaseRow, PortTargetRow, PortTaskRow};
use crate::daemon::support::{collect_rows, parse_json, parse_opt_json, serialize_opt};
use crate::error::{StoreError, StoreResult};

/// Insert or replace a port target row.
pub fn upsert_port_target(conn: &Connection, row: &PortTargetRow) -> StoreResult<()> {
    let last_parity = serialize_opt(&row.last_parity_report_json)?;
    let last_perf = serialize_opt(&row.last_perf_gap_json)?;
    conn.execute(
        "INSERT INTO daemon_port_target (
            id, run_id, target, replacement, target_repo, replacement_repo, request,
            status, current_phase_id, worker_cap, last_audit_score,
            last_parity_report_json, last_perf_gap_json, rollback_status,
            quarantine_status, time_created, time_updated
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)
        ON CONFLICT(id) DO UPDATE SET
            target = excluded.target,
            replacement = excluded.replacement,
            target_repo = excluded.target_repo,
            replacement_repo = excluded.replacement_repo,
            request = excluded.request,
            status = excluded.status,
            current_phase_id = excluded.current_phase_id,
            worker_cap = excluded.worker_cap,
            last_audit_score = excluded.last_audit_score,
            last_parity_report_json = excluded.last_parity_report_json,
            last_perf_gap_json = excluded.last_perf_gap_json,
            rollback_status = excluded.rollback_status,
            quarantine_status = excluded.quarantine_status,
            time_updated = excluded.time_updated",
        params![
            row.id,
            row.run_id,
            row.target,
            row.replacement,
            row.target_repo,
            row.replacement_repo,
            row.request,
            row.status,
            row.current_phase_id,
            row.worker_cap,
            row.last_audit_score,
            last_parity,
            last_perf,
            row.rollback_status,
            row.quarantine_status,
            row.time_created,
            row.time_updated,
        ],
    )?;
    Ok(())
}

/// Read a port target row.
pub fn get_port_target(conn: &Connection, id: &str) -> StoreResult<Option<PortTargetRow>> {
    conn.query_row(
        "SELECT id, run_id, target, replacement, target_repo, replacement_repo, request,
                status, current_phase_id, worker_cap, last_audit_score,
                last_parity_report_json, last_perf_gap_json, rollback_status,
                quarantine_status, time_created, time_updated
         FROM daemon_port_target WHERE id = ?1",
        params![id],
        port_target_from_row,
    )
    .optional()
    .map_err(StoreError::from)
}

/// List port targets for a run.
pub fn list_port_targets_for_run(
    conn: &Connection,
    run_id: &str,
) -> StoreResult<Vec<PortTargetRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, run_id, target, replacement, target_repo, replacement_repo, request,
                status, current_phase_id, worker_cap, last_audit_score,
                last_parity_report_json, last_perf_gap_json, rollback_status,
                quarantine_status, time_created, time_updated
         FROM daemon_port_target WHERE run_id = ?1 ORDER BY time_created ASC, id ASC",
    )?;
    let rows = stmt.query_map(params![run_id], port_target_from_row)?;
    collect_rows(rows)
}

/// Insert or replace a port phase row.
pub fn upsert_port_phase(conn: &Connection, row: &PortPhaseRow) -> StoreResult<()> {
    let plan = serialize_opt(&row.plan_json)?;
    let parity = serialize_opt(&row.last_parity_report_json)?;
    conn.execute(
        "INSERT INTO daemon_port_phase (
            id, run_id, target_id, ordinal, name, status, strategy, plan_json,
            task_count, last_audit_score, last_parity_report_json, time_created, time_updated
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
        ON CONFLICT(id) DO UPDATE SET
            ordinal = excluded.ordinal,
            name = excluded.name,
            status = excluded.status,
            strategy = excluded.strategy,
            plan_json = excluded.plan_json,
            task_count = excluded.task_count,
            last_audit_score = excluded.last_audit_score,
            last_parity_report_json = excluded.last_parity_report_json,
            time_updated = excluded.time_updated",
        params![
            row.id,
            row.run_id,
            row.target_id,
            row.ordinal,
            row.name,
            row.status,
            row.strategy,
            plan,
            row.task_count,
            row.last_audit_score,
            parity,
            row.time_created,
            row.time_updated,
        ],
    )?;
    Ok(())
}

/// List phases for a target.
pub fn list_port_phases_for_target(
    conn: &Connection,
    target_id: &str,
) -> StoreResult<Vec<PortPhaseRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, run_id, target_id, ordinal, name, status, strategy, plan_json,
                task_count, last_audit_score, last_parity_report_json, time_created, time_updated
         FROM daemon_port_phase WHERE target_id = ?1 ORDER BY ordinal ASC",
    )?;
    let rows = stmt.query_map(params![target_id], port_phase_from_row)?;
    collect_rows(rows)
}

/// Insert or replace a port task row.
pub fn upsert_port_task(conn: &Connection, row: &PortTaskRow) -> StoreResult<()> {
    let scope = serde_json::to_string(&row.write_scope)?;
    conn.execute(
        "INSERT INTO daemon_port_task (
            id, run_id, phase_id, title, status, worker_id, branch, write_scope_json,
            proof_lane, attempt_count, rollback_status, quarantine_reason, last_error,
            time_created, time_updated
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
        ON CONFLICT(id) DO UPDATE SET
            title = excluded.title,
            status = excluded.status,
            worker_id = excluded.worker_id,
            branch = excluded.branch,
            write_scope_json = excluded.write_scope_json,
            proof_lane = excluded.proof_lane,
            attempt_count = excluded.attempt_count,
            rollback_status = excluded.rollback_status,
            quarantine_reason = excluded.quarantine_reason,
            last_error = excluded.last_error,
            time_updated = excluded.time_updated",
        params![
            row.id,
            row.run_id,
            row.phase_id,
            row.title,
            row.status,
            row.worker_id,
            row.branch,
            scope,
            row.proof_lane,
            row.attempt_count,
            row.rollback_status,
            row.quarantine_reason,
            row.last_error,
            row.time_created,
            row.time_updated,
        ],
    )?;
    Ok(())
}

/// Read a port task row.
pub fn get_port_task(conn: &Connection, id: &str) -> StoreResult<Option<PortTaskRow>> {
    conn.query_row(
        "SELECT id, run_id, phase_id, title, status, worker_id, branch, write_scope_json,
                proof_lane, attempt_count, rollback_status, quarantine_reason, last_error,
                time_created, time_updated
         FROM daemon_port_task WHERE id = ?1",
        params![id],
        port_task_from_row,
    )
    .optional()
    .map_err(StoreError::from)
}

/// List tasks for a phase.
pub fn list_port_tasks_for_phase(
    conn: &Connection,
    phase_id: &str,
) -> StoreResult<Vec<PortTaskRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, run_id, phase_id, title, status, worker_id, branch, write_scope_json,
                proof_lane, attempt_count, rollback_status, quarantine_reason, last_error,
                time_created, time_updated
         FROM daemon_port_task WHERE phase_id = ?1 ORDER BY time_created ASC, id ASC",
    )?;
    let rows = stmt.query_map(params![phase_id], port_task_from_row)?;
    collect_rows(rows)
}

fn port_target_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<PortTargetRow> {
    let parity_text: Option<String> = row.get(11)?;
    let perf_text: Option<String> = row.get(12)?;
    Ok(PortTargetRow {
        id: row.get(0)?,
        run_id: row.get(1)?,
        target: row.get(2)?,
        replacement: row.get(3)?,
        target_repo: row.get(4)?,
        replacement_repo: row.get(5)?,
        request: row.get(6)?,
        status: row.get(7)?,
        current_phase_id: row.get(8)?,
        worker_cap: row.get(9)?,
        last_audit_score: row.get(10)?,
        last_parity_report_json: parse_opt_json(11, parity_text)?,
        last_perf_gap_json: parse_opt_json(12, perf_text)?,
        rollback_status: row.get(13)?,
        quarantine_status: row.get(14)?,
        time_created: row.get(15)?,
        time_updated: row.get(16)?,
    })
}

fn port_phase_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<PortPhaseRow> {
    let plan_text: Option<String> = row.get(7)?;
    let parity_text: Option<String> = row.get(10)?;
    Ok(PortPhaseRow {
        id: row.get(0)?,
        run_id: row.get(1)?,
        target_id: row.get(2)?,
        ordinal: row.get(3)?,
        name: row.get(4)?,
        status: row.get(5)?,
        strategy: row.get(6)?,
        plan_json: parse_opt_json(7, plan_text)?,
        task_count: row.get(8)?,
        last_audit_score: row.get(9)?,
        last_parity_report_json: parse_opt_json(10, parity_text)?,
        time_created: row.get(11)?,
        time_updated: row.get(12)?,
    })
}

fn port_task_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<PortTaskRow> {
    let scope_text: String = row.get(7)?;
    Ok(PortTaskRow {
        id: row.get(0)?,
        run_id: row.get(1)?,
        phase_id: row.get(2)?,
        title: row.get(3)?,
        status: row.get(4)?,
        worker_id: row.get(5)?,
        branch: row.get(6)?,
        write_scope: parse_json(7, &scope_text)?,
        proof_lane: row.get(8)?,
        attempt_count: row.get(9)?,
        rollback_status: row.get(10)?,
        quarantine_reason: row.get(11)?,
        last_error: row.get(12)?,
        time_created: row.get(13)?,
        time_updated: row.get(14)?,
    })
}
