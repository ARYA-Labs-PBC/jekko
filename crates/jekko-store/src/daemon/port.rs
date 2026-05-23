//! Durable generic-port workflow tables.

use rusqlite::{params, Connection, OptionalExtension};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::error::{StoreError, StoreResult};

/// Row in `daemon_port_target`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PortTargetRow {
    /// Target id.
    pub id: String,
    /// Owning daemon run.
    pub run_id: String,
    /// Reference system being ported.
    pub target: String,
    /// Candidate replacement system.
    pub replacement: String,
    /// Reference repository path or URL.
    pub target_repo: Option<String>,
    /// Candidate repository path or URL.
    pub replacement_repo: Option<String>,
    /// Original user request.
    pub request: String,
    /// Workflow status.
    pub status: String,
    /// Current phase id.
    pub current_phase_id: Option<String>,
    /// Maximum worker count.
    pub worker_cap: i64,
    /// Last Jankurai score.
    pub last_audit_score: Option<f64>,
    /// Last parity report payload.
    pub last_parity_report_json: Option<serde_json::Value>,
    /// Last perf gap payload.
    pub last_perf_gap_json: Option<serde_json::Value>,
    /// Rollback status.
    pub rollback_status: String,
    /// Quarantine status.
    pub quarantine_status: String,
    /// Creation timestamp.
    pub time_created: i64,
    /// Last-update timestamp.
    pub time_updated: i64,
}

/// Row in `daemon_port_phase`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PortPhaseRow {
    /// Phase id.
    pub id: String,
    /// Owning daemon run.
    pub run_id: String,
    /// Owning target id.
    pub target_id: String,
    /// Phase order.
    pub ordinal: i64,
    /// Phase name.
    pub name: String,
    /// Phase status.
    pub status: String,
    /// Strategy tag.
    pub strategy: String,
    /// Finalized phase plan.
    pub plan_json: Option<serde_json::Value>,
    /// Number of tasks.
    pub task_count: i64,
    /// Last Jankurai score.
    pub last_audit_score: Option<f64>,
    /// Last parity report payload.
    pub last_parity_report_json: Option<serde_json::Value>,
    /// Creation timestamp.
    pub time_created: i64,
    /// Last-update timestamp.
    pub time_updated: i64,
}

/// Row in `daemon_port_task`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PortTaskRow {
    /// Task id.
    pub id: String,
    /// Owning daemon run.
    pub run_id: String,
    /// Owning phase id.
    pub phase_id: String,
    /// Task title.
    pub title: String,
    /// Task status.
    pub status: String,
    /// Assigned worker id.
    pub worker_id: Option<String>,
    /// Worker branch.
    pub branch: Option<String>,
    /// Declared write scope.
    pub write_scope: Vec<String>,
    /// Proof lane.
    pub proof_lane: Option<String>,
    /// Attempt count.
    pub attempt_count: i64,
    /// Rollback status.
    pub rollback_status: String,
    /// Quarantine reason.
    pub quarantine_reason: Option<String>,
    /// Last error.
    pub last_error: Option<String>,
    /// Creation timestamp.
    pub time_created: i64,
    /// Last-update timestamp.
    pub time_updated: i64,
}

/// Row in `daemon_parity_case`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParityCaseRow {
    /// Case id.
    pub id: String,
    /// Owning daemon run.
    pub run_id: String,
    /// Owning port target id.
    pub target_id: String,
    /// Case tags.
    pub tags: Vec<String>,
    /// Target adapter kind.
    pub target_kind: String,
    /// Target-switched case steps.
    pub steps_json: serde_json::Value,
    /// Performance budget payload.
    pub perf_json: Option<serde_json::Value>,
    /// Whether the case is approved for required gates.
    pub approved: bool,
    /// Creation timestamp.
    pub time_created: i64,
    /// Last-update timestamp.
    pub time_updated: i64,
}

/// Row in `daemon_parity_run`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParityRunRow {
    /// Parity run id.
    pub id: String,
    /// Owning daemon run.
    pub run_id: String,
    /// Owning port target id.
    pub target_id: String,
    /// Number of cases in the report.
    pub case_count: i64,
    /// Run status.
    pub status: String,
    /// Report path.
    pub report_path: Option<String>,
    /// Start timestamp.
    pub started_at: Option<i64>,
    /// End timestamp.
    pub ended_at: Option<i64>,
    /// Summary payload.
    pub summary_json: Option<serde_json::Value>,
    /// Creation timestamp.
    pub time_created: i64,
    /// Last-update timestamp.
    pub time_updated: i64,
}

/// Row in `daemon_parity_result`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParityResultRow {
    /// Result id.
    pub id: String,
    /// Owning parity run id.
    pub parity_run_id: String,
    /// Case id.
    pub case_id: String,
    /// Reference or candidate target name.
    pub target_name: String,
    /// Result status.
    pub status: String,
    /// Whether the case was skipped.
    pub skipped: bool,
    /// Duration in milliseconds.
    pub duration_ms: Option<i64>,
    /// Performance result payload.
    pub perf_json: Option<serde_json::Value>,
    /// Message.
    pub message: Option<String>,
    /// Creation timestamp.
    pub time_created: i64,
}

/// Row in `daemon_perf_budget`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PerfBudgetRow {
    /// Budget id.
    pub id: String,
    /// Owning daemon run.
    pub run_id: String,
    /// Case id.
    pub case_id: String,
    /// Metric name.
    pub metric: String,
    /// Maximum reference-to-candidate ratio.
    pub max_ratio: Option<f64>,
    /// Baseline metric value.
    pub baseline_value: Option<f64>,
    /// Candidate metric value.
    pub candidate_value: Option<f64>,
    /// Budget status.
    pub status: String,
    /// Creation timestamp.
    pub time_created: i64,
    /// Last-update timestamp.
    pub time_updated: i64,
}

/// Row in `daemon_repo_graph_node`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RepoGraphNodeRow {
    /// Node id.
    pub id: String,
    /// Owning daemon run.
    pub run_id: String,
    /// Node kind.
    pub kind: String,
    /// Stable key.
    pub key: String,
    /// Human-readable label.
    pub label: String,
    /// Node payload.
    pub payload_json: Option<serde_json::Value>,
    /// Creation timestamp.
    pub time_created: i64,
    /// Last-update timestamp.
    pub time_updated: i64,
}

/// Row in `daemon_repo_graph_edge`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RepoGraphEdgeRow {
    /// Owning daemon run.
    pub run_id: String,
    /// Source node id.
    pub src_node_id: String,
    /// Destination node id.
    pub dst_node_id: String,
    /// Edge kind.
    pub kind: String,
    /// Edge payload.
    pub payload_json: Option<serde_json::Value>,
    /// Creation timestamp.
    pub time_created: i64,
}

/// Row in `daemon_model_outcome`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelOutcomeRow {
    /// Outcome id.
    pub id: String,
    /// Owning daemon run.
    pub run_id: String,
    /// Task id.
    pub task_id: Option<String>,
    /// Model id.
    pub model_id: String,
    /// Model role.
    pub role: String,
    /// Cost in USD.
    pub cost_usd: Option<f64>,
    /// Latency in milliseconds.
    pub latency_ms: Option<i64>,
    /// Outcome status.
    pub status: String,
    /// Reviewer score.
    pub reviewer_score: Option<f64>,
    /// Whether this outcome became a winner.
    pub winner: bool,
    /// Extra outcome payload.
    pub payload_json: Option<serde_json::Value>,
    /// Creation timestamp.
    pub time_created: i64,
    /// Last-update timestamp.
    pub time_updated: i64,
}

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

/// Insert or replace a repo graph node.
pub fn upsert_repo_graph_node(conn: &Connection, row: &RepoGraphNodeRow) -> StoreResult<()> {
    let payload = serialize_opt(&row.payload_json)?;
    conn.execute(
        "INSERT INTO daemon_repo_graph_node (
            id, run_id, kind, key, label, payload_json, time_created, time_updated
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
        ON CONFLICT(id) DO UPDATE SET
            kind = excluded.kind,
            key = excluded.key,
            label = excluded.label,
            payload_json = excluded.payload_json,
            time_updated = excluded.time_updated",
        params![
            row.id,
            row.run_id,
            row.kind,
            row.key,
            row.label,
            payload,
            row.time_created,
            row.time_updated,
        ],
    )?;
    Ok(())
}

/// Insert or replace a repo graph edge.
pub fn upsert_repo_graph_edge(conn: &Connection, row: &RepoGraphEdgeRow) -> StoreResult<()> {
    let payload = serialize_opt(&row.payload_json)?;
    conn.execute(
        "INSERT OR REPLACE INTO daemon_repo_graph_edge
         (run_id, src_node_id, dst_node_id, kind, payload_json, time_created)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            row.run_id,
            row.src_node_id,
            row.dst_node_id,
            row.kind,
            payload,
            row.time_created,
        ],
    )?;
    Ok(())
}

/// List repo graph nodes for a run.
pub fn list_repo_graph_nodes_for_run(
    conn: &Connection,
    run_id: &str,
) -> StoreResult<Vec<RepoGraphNodeRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, run_id, kind, key, label, payload_json, time_created, time_updated
         FROM daemon_repo_graph_node WHERE run_id = ?1 ORDER BY kind ASC, key ASC",
    )?;
    let rows = stmt.query_map(params![run_id], repo_graph_node_from_row)?;
    collect_rows(rows)
}

/// List repo graph edges for a run.
pub fn list_repo_graph_edges_for_run(
    conn: &Connection,
    run_id: &str,
) -> StoreResult<Vec<RepoGraphEdgeRow>> {
    let mut stmt = conn.prepare(
        "SELECT run_id, src_node_id, dst_node_id, kind, payload_json, time_created
         FROM daemon_repo_graph_edge WHERE run_id = ?1 ORDER BY src_node_id ASC, dst_node_id ASC",
    )?;
    let rows = stmt.query_map(params![run_id], repo_graph_edge_from_row)?;
    collect_rows(rows)
}

/// Insert or replace a model outcome row.
pub fn upsert_model_outcome(conn: &Connection, row: &ModelOutcomeRow) -> StoreResult<()> {
    let payload = serialize_opt(&row.payload_json)?;
    conn.execute(
        "INSERT INTO daemon_model_outcome (
            id, run_id, task_id, model_id, role, cost_usd, latency_ms, status,
            reviewer_score, winner, payload_json, time_created, time_updated
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
        ON CONFLICT(id) DO UPDATE SET
            task_id = excluded.task_id,
            model_id = excluded.model_id,
            role = excluded.role,
            cost_usd = excluded.cost_usd,
            latency_ms = excluded.latency_ms,
            status = excluded.status,
            reviewer_score = excluded.reviewer_score,
            winner = excluded.winner,
            payload_json = excluded.payload_json,
            time_updated = excluded.time_updated",
        params![
            row.id,
            row.run_id,
            row.task_id,
            row.model_id,
            row.role,
            row.cost_usd,
            row.latency_ms,
            row.status,
            row.reviewer_score,
            row.winner as i64,
            payload,
            row.time_created,
            row.time_updated,
        ],
    )?;
    Ok(())
}

/// List model outcomes for a run.
pub fn list_model_outcomes_for_run(
    conn: &Connection,
    run_id: &str,
) -> StoreResult<Vec<ModelOutcomeRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, run_id, task_id, model_id, role, cost_usd, latency_ms, status,
                reviewer_score, winner, payload_json, time_created, time_updated
         FROM daemon_model_outcome WHERE run_id = ?1 ORDER BY time_created ASC, id ASC",
    )?;
    let rows = stmt.query_map(params![run_id], model_outcome_from_row)?;
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

fn repo_graph_node_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<RepoGraphNodeRow> {
    let payload_text: Option<String> = row.get(5)?;
    Ok(RepoGraphNodeRow {
        id: row.get(0)?,
        run_id: row.get(1)?,
        kind: row.get(2)?,
        key: row.get(3)?,
        label: row.get(4)?,
        payload_json: parse_opt_json(5, payload_text)?,
        time_created: row.get(6)?,
        time_updated: row.get(7)?,
    })
}

fn repo_graph_edge_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<RepoGraphEdgeRow> {
    let payload_text: Option<String> = row.get(4)?;
    Ok(RepoGraphEdgeRow {
        run_id: row.get(0)?,
        src_node_id: row.get(1)?,
        dst_node_id: row.get(2)?,
        kind: row.get(3)?,
        payload_json: parse_opt_json(4, payload_text)?,
        time_created: row.get(5)?,
    })
}

fn model_outcome_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ModelOutcomeRow> {
    let winner: i64 = row.get(9)?;
    let payload_text: Option<String> = row.get(10)?;
    Ok(ModelOutcomeRow {
        id: row.get(0)?,
        run_id: row.get(1)?,
        task_id: row.get(2)?,
        model_id: row.get(3)?,
        role: row.get(4)?,
        cost_usd: row.get(5)?,
        latency_ms: row.get(6)?,
        status: row.get(7)?,
        reviewer_score: row.get(8)?,
        winner: winner != 0,
        payload_json: parse_opt_json(10, payload_text)?,
        time_created: row.get(11)?,
        time_updated: row.get(12)?,
    })
}

fn serialize_opt(value: &Option<serde_json::Value>) -> StoreResult<Option<String>> {
    value
        .as_ref()
        .map(serde_json::to_string)
        .transpose()
        .map_err(StoreError::from)
}

fn parse_json<T: DeserializeOwned>(idx: usize, text: &str) -> rusqlite::Result<T> {
    serde_json::from_str(text).map_err(|err| {
        rusqlite::Error::FromSqlConversionFailure(idx, rusqlite::types::Type::Text, Box::new(err))
    })
}

fn parse_opt_json<T: DeserializeOwned>(
    idx: usize,
    text: Option<String>,
) -> rusqlite::Result<Option<T>> {
    text.as_deref().map(|s| parse_json(idx, s)).transpose()
}

fn collect_rows<T, F>(rows: rusqlite::MappedRows<'_, F>) -> StoreResult<Vec<T>>
where
    F: FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<T>,
{
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}
