//! Durable daemon-forever tables used by the Rust runner bridge.

use rusqlite::{params, Connection, OptionalExtension};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::error::{StoreError, StoreResult};

/// Row in `daemon_finding`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DaemonFindingRow {
    /// Finding id.
    pub id: String,
    /// Owning daemon run.
    pub run_id: String,
    /// Iteration that captured this finding.
    pub iteration: i64,
    /// Jankurai rule id.
    pub rule_id: String,
    /// Stable finding fingerprint.
    pub fingerprint: String,
    /// Severity label.
    pub severity: String,
    /// Paths touched by this finding.
    pub paths: Vec<String>,
    /// Cap id when the finding represents a cap.
    pub cap: Option<String>,
    /// Queue status.
    pub status: String,
    /// Attempt count.
    pub attempt_count: i64,
    /// Assigned batch id.
    pub batch_id: Option<String>,
    /// Last error, if any.
    pub last_error: Option<String>,
    /// Creation timestamp.
    pub time_created: i64,
    /// Last-update timestamp.
    pub time_updated: i64,
}

/// Row in `daemon_finding_batch`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DaemonFindingBatchRow {
    /// Batch id.
    pub id: String,
    /// Owning daemon run.
    pub run_id: String,
    /// Wave index.
    pub wave_index: i64,
    /// Dispatch lane.
    pub lane: String,
    /// Assigned worker id.
    pub worker_id: Option<String>,
    /// Batch status.
    pub status: String,
    /// Start timestamp.
    pub started_at: Option<i64>,
    /// End timestamp.
    pub ended_at: Option<i64>,
    /// Batch result JSON.
    pub result_json: Option<serde_json::Value>,
    /// Creation timestamp.
    pub time_created: i64,
    /// Last-update timestamp.
    pub time_updated: i64,
}

/// Row in `daemon_finding_edge`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DaemonFindingEdgeRow {
    /// Owning daemon run.
    pub run_id: String,
    /// Parent finding id.
    pub parent_id: String,
    /// Child finding id.
    pub child_id: String,
    /// Edge kind.
    pub kind: String,
    /// Creation timestamp.
    pub time_created: i64,
}

/// Row in `daemon_concept`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DaemonConceptRow {
    /// Row id.
    pub id: String,
    /// Owning daemon run.
    pub run_id: String,
    /// Stable concept id.
    pub concept_id: String,
    /// Human-readable definition.
    pub definition: String,
    /// Source concept or artifact refs.
    pub derived_from_json: Option<serde_json::Value>,
    /// Proof references.
    pub proof_refs_json: Option<serde_json::Value>,
    /// Confidence score.
    pub confidence: f64,
    /// Invalidation timestamp.
    pub invalidated_at: Option<i64>,
    /// Invalidation reason.
    pub invalidated_reason: Option<String>,
    /// Creation timestamp.
    pub time_created: i64,
    /// Last-update timestamp.
    pub time_updated: i64,
}

/// Row in `daemon_concept_link`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DaemonConceptLinkRow {
    /// Owning daemon run.
    pub run_id: String,
    /// Parent concept id.
    pub parent_concept: String,
    /// Child concept id.
    pub child_concept: String,
    /// Link relation.
    pub relation: String,
    /// Creation timestamp.
    pub time_created: i64,
}

/// Row in `daemon_regression_cycle`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DaemonRegressionCycleRow {
    /// Cycle id.
    pub id: String,
    /// Owning daemon run.
    pub run_id: String,
    /// Iteration number.
    pub iteration: i64,
    /// Baseline audit score.
    pub baseline_score: Option<f64>,
    /// Current audit score.
    pub current_score: Option<f64>,
    /// Hard finding delta.
    pub hard_delta: i64,
    /// Soft finding delta.
    pub soft_delta: i64,
    /// Cap delta.
    pub caps_delta: i64,
    /// Cycle status.
    pub status: String,
    /// Result payload.
    pub result_json: Option<serde_json::Value>,
    /// Creation timestamp.
    pub time_created: i64,
    /// Last-update timestamp.
    pub time_updated: i64,
}

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

/// Insert or replace a `daemon_concept` row.
pub fn upsert_concept(conn: &Connection, row: &DaemonConceptRow) -> StoreResult<()> {
    let derived = row
        .derived_from_json
        .as_ref()
        .map(serde_json::to_string)
        .transpose()?;
    let proof = row
        .proof_refs_json
        .as_ref()
        .map(serde_json::to_string)
        .transpose()?;
    conn.execute(
        "INSERT INTO daemon_concept (
            id, run_id, concept_id, definition, derived_from_json, proof_refs_json,
            confidence, invalidated_at, invalidated_reason, time_created, time_updated
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
        ON CONFLICT(id) DO UPDATE SET
            concept_id = excluded.concept_id,
            definition = excluded.definition,
            derived_from_json = excluded.derived_from_json,
            proof_refs_json = excluded.proof_refs_json,
            confidence = excluded.confidence,
            invalidated_at = excluded.invalidated_at,
            invalidated_reason = excluded.invalidated_reason,
            time_updated = excluded.time_updated",
        params![
            row.id,
            row.run_id,
            row.concept_id,
            row.definition,
            derived,
            proof,
            row.confidence,
            row.invalidated_at,
            row.invalidated_reason,
            row.time_created,
            row.time_updated,
        ],
    )?;
    Ok(())
}

/// Read a concept by run and concept id.
pub fn get_concept(
    conn: &Connection,
    run_id: &str,
    concept_id: &str,
) -> StoreResult<Option<DaemonConceptRow>> {
    conn.query_row(
        "SELECT id, run_id, concept_id, definition, derived_from_json, proof_refs_json,
                confidence, invalidated_at, invalidated_reason, time_created, time_updated
         FROM daemon_concept WHERE run_id = ?1 AND concept_id = ?2",
        params![run_id, concept_id],
        concept_from_row,
    )
    .optional()
    .map_err(StoreError::from)
}

/// List active concepts for a run.
pub fn list_active_concepts_for_run(
    conn: &Connection,
    run_id: &str,
) -> StoreResult<Vec<DaemonConceptRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, run_id, concept_id, definition, derived_from_json, proof_refs_json,
                confidence, invalidated_at, invalidated_reason, time_created, time_updated
         FROM daemon_concept
         WHERE run_id = ?1 AND invalidated_at IS NULL
         ORDER BY concept_id ASC",
    )?;
    let rows = stmt.query_map(params![run_id], concept_from_row)?;
    collect_rows(rows)
}

/// Insert or replace a concept link.
pub fn upsert_concept_link(conn: &Connection, row: &DaemonConceptLinkRow) -> StoreResult<()> {
    conn.execute(
        "INSERT OR REPLACE INTO daemon_concept_link
         (run_id, parent_concept, child_concept, relation, time_created)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            row.run_id,
            row.parent_concept,
            row.child_concept,
            row.relation,
            row.time_created,
        ],
    )?;
    Ok(())
}

/// List concept links for a run.
pub fn list_concept_links_for_run(
    conn: &Connection,
    run_id: &str,
) -> StoreResult<Vec<DaemonConceptLinkRow>> {
    let mut stmt = conn.prepare(
        "SELECT run_id, parent_concept, child_concept, relation, time_created
         FROM daemon_concept_link WHERE run_id = ?1 ORDER BY parent_concept ASC, child_concept ASC",
    )?;
    let rows = stmt.query_map(params![run_id], |row| {
        Ok(DaemonConceptLinkRow {
            run_id: row.get(0)?,
            parent_concept: row.get(1)?,
            child_concept: row.get(2)?,
            relation: row.get(3)?,
            time_created: row.get(4)?,
        })
    })?;
    collect_rows(rows)
}

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

fn concept_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<DaemonConceptRow> {
    let derived_text: Option<String> = row.get(4)?;
    let proof_text: Option<String> = row.get(5)?;
    Ok(DaemonConceptRow {
        id: row.get(0)?,
        run_id: row.get(1)?,
        concept_id: row.get(2)?,
        definition: row.get(3)?,
        derived_from_json: parse_opt_json(4, derived_text)?,
        proof_refs_json: parse_opt_json(5, proof_text)?,
        confidence: row.get(6)?,
        invalidated_at: row.get(7)?,
        invalidated_reason: row.get(8)?,
        time_created: row.get(9)?,
        time_updated: row.get(10)?,
    })
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
