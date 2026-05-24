//! Durable advanced-reasoning tables for ZYAL daemon runs.

use rusqlite::{params, Connection, OptionalExtension};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::error::{StoreError, StoreResult};

/// Row in `daemon_reasoning_artifact`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReasoningArtifactRow {
    /// Artifact id.
    pub id: String,
    /// Owning run id.
    pub run_id: String,
    /// Producer role.
    pub role: String,
    /// Artifact kind.
    pub kind: String,
    /// Title.
    pub title: String,
    /// Stored summary.
    pub summary: String,
    /// Evidence level.
    pub evidence_level: String,
    /// Calibrated confidence.
    pub confidence: f64,
    /// Structured payload.
    pub payload_json: Option<serde_json::Value>,
    /// Stable content hash.
    pub content_hash: String,
    /// Artifact status.
    pub status: String,
    /// Creation timestamp.
    pub time_created: i64,
    /// Last-update timestamp.
    pub time_updated: i64,
}

/// Row in `daemon_reasoning_edge`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReasoningEdgeRow {
    /// Owning run id.
    pub run_id: String,
    /// Source artifact id.
    pub src_artifact_id: String,
    /// Destination artifact id.
    pub dst_artifact_id: String,
    /// Edge kind.
    pub kind: String,
    /// Optional weight.
    pub weight: Option<f64>,
    /// Structured payload.
    pub payload_json: Option<serde_json::Value>,
    /// Creation timestamp.
    pub time_created: i64,
}

/// Row in `daemon_reasoning_lane`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReasoningLaneRow {
    /// Lane id.
    pub id: String,
    /// Owning run id.
    pub run_id: String,
    /// Lane role.
    pub role: String,
    /// Diversity strategy.
    pub strategy: String,
    /// Lane status.
    pub status: String,
    /// Produced artifact ids.
    pub artifact_ids: Vec<String>,
    /// Declared write scope.
    pub write_scope: Vec<String>,
    /// Worker id.
    pub worker_id: Option<String>,
    /// Lane confidence.
    pub confidence: f64,
    /// Creation timestamp.
    pub time_created: i64,
    /// Last-update timestamp.
    pub time_updated: i64,
}

/// Row in `daemon_memory_capsule`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryCapsuleRow {
    /// Capsule id.
    pub id: String,
    /// Owning run id.
    pub run_id: String,
    /// Source artifact id.
    pub artifact_id: String,
    /// Memory scope.
    pub scope: String,
    /// Capsule status.
    pub status: String,
    /// Stored summary.
    pub summary: String,
    /// Evidence level.
    pub evidence_level: String,
    /// Confidence.
    pub confidence: f64,
    /// Structured payload.
    pub payload_json: Option<serde_json::Value>,
    /// Stable content hash.
    pub content_hash: String,
    /// Creation timestamp.
    pub time_created: i64,
    /// Last-update timestamp.
    pub time_updated: i64,
}

/// Row in `daemon_model_reliability`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelReliabilityRow {
    /// Model id.
    pub model_id: String,
    /// Role.
    pub role: String,
    /// Task kind.
    pub task_kind: String,
    /// Success count.
    pub success_count: i64,
    /// Failure count.
    pub failure_count: i64,
    /// Winner count.
    pub winner_count: i64,
    /// Total latency.
    pub total_latency_ms: i64,
    /// Total cost.
    pub total_cost_usd: f64,
    /// Reliability score.
    pub score: f64,
    /// Creation timestamp.
    pub time_created: i64,
    /// Last-update timestamp.
    pub time_updated: i64,
}

/// Insert or replace a reasoning artifact.
pub fn upsert_reasoning_artifact(conn: &Connection, row: &ReasoningArtifactRow) -> StoreResult<()> {
    let payload = serialize_opt(&row.payload_json)?;
    conn.execute(
        "INSERT INTO daemon_reasoning_artifact (
            id, run_id, role, kind, title, summary, evidence_level, confidence,
            payload_json, content_hash, status, time_created, time_updated
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
        ON CONFLICT(id) DO UPDATE SET
            role = excluded.role,
            kind = excluded.kind,
            title = excluded.title,
            summary = excluded.summary,
            evidence_level = excluded.evidence_level,
            confidence = excluded.confidence,
            payload_json = excluded.payload_json,
            content_hash = excluded.content_hash,
            status = excluded.status,
            time_updated = excluded.time_updated",
        params![
            row.id,
            row.run_id,
            row.role,
            row.kind,
            row.title,
            row.summary,
            row.evidence_level,
            row.confidence,
            payload,
            row.content_hash,
            row.status,
            row.time_created,
            row.time_updated,
        ],
    )?;
    Ok(())
}

/// List artifacts for a run.
pub fn list_reasoning_artifacts_for_run(
    conn: &Connection,
    run_id: &str,
) -> StoreResult<Vec<ReasoningArtifactRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, run_id, role, kind, title, summary, evidence_level, confidence,
                payload_json, content_hash, status, time_created, time_updated
         FROM daemon_reasoning_artifact WHERE run_id = ?1 ORDER BY time_created ASC, id ASC",
    )?;
    let rows = stmt.query_map(params![run_id], reasoning_artifact_from_row)?;
    collect_rows(rows)
}

/// Insert or replace a reasoning edge.
pub fn upsert_reasoning_edge(conn: &Connection, row: &ReasoningEdgeRow) -> StoreResult<()> {
    let payload = serialize_opt(&row.payload_json)?;
    conn.execute(
        "INSERT OR REPLACE INTO daemon_reasoning_edge (
            run_id, src_artifact_id, dst_artifact_id, kind, weight, payload_json, time_created
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            row.run_id,
            row.src_artifact_id,
            row.dst_artifact_id,
            row.kind,
            row.weight,
            payload,
            row.time_created,
        ],
    )?;
    Ok(())
}

/// List reasoning edges for a run.
pub fn list_reasoning_edges_for_run(
    conn: &Connection,
    run_id: &str,
) -> StoreResult<Vec<ReasoningEdgeRow>> {
    let mut stmt = conn.prepare(
        "SELECT run_id, src_artifact_id, dst_artifact_id, kind, weight, payload_json, time_created
         FROM daemon_reasoning_edge WHERE run_id = ?1 ORDER BY src_artifact_id ASC, dst_artifact_id ASC",
    )?;
    let rows = stmt.query_map(params![run_id], reasoning_edge_from_row)?;
    collect_rows(rows)
}

/// Insert or replace a reasoning lane.
pub fn upsert_reasoning_lane(conn: &Connection, row: &ReasoningLaneRow) -> StoreResult<()> {
    let artifact_ids = serde_json::to_string(&row.artifact_ids)?;
    let write_scope = serde_json::to_string(&row.write_scope)?;
    conn.execute(
        "INSERT INTO daemon_reasoning_lane (
            id, run_id, role, strategy, status, artifact_ids_json, write_scope_json,
            worker_id, confidence, time_created, time_updated
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
        ON CONFLICT(id) DO UPDATE SET
            role = excluded.role,
            strategy = excluded.strategy,
            status = excluded.status,
            artifact_ids_json = excluded.artifact_ids_json,
            write_scope_json = excluded.write_scope_json,
            worker_id = excluded.worker_id,
            confidence = excluded.confidence,
            time_updated = excluded.time_updated",
        params![
            row.id,
            row.run_id,
            row.role,
            row.strategy,
            row.status,
            artifact_ids,
            write_scope,
            row.worker_id,
            row.confidence,
            row.time_created,
            row.time_updated,
        ],
    )?;
    Ok(())
}

/// List reasoning lanes for a run.
pub fn list_reasoning_lanes_for_run(
    conn: &Connection,
    run_id: &str,
) -> StoreResult<Vec<ReasoningLaneRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, run_id, role, strategy, status, artifact_ids_json, write_scope_json,
                worker_id, confidence, time_created, time_updated
         FROM daemon_reasoning_lane WHERE run_id = ?1 ORDER BY time_created ASC, id ASC",
    )?;
    let rows = stmt.query_map(params![run_id], reasoning_lane_from_row)?;
    collect_rows(rows)
}

/// Insert or replace a memory capsule.
pub fn upsert_memory_capsule(conn: &Connection, row: &MemoryCapsuleRow) -> StoreResult<()> {
    let payload = serialize_opt(&row.payload_json)?;
    conn.execute(
        "INSERT INTO daemon_memory_capsule (
            id, run_id, artifact_id, scope, status, summary, evidence_level,
            confidence, payload_json, content_hash, time_created, time_updated
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
        ON CONFLICT(id) DO UPDATE SET
            scope = excluded.scope,
            status = excluded.status,
            summary = excluded.summary,
            evidence_level = excluded.evidence_level,
            confidence = excluded.confidence,
            payload_json = excluded.payload_json,
            content_hash = excluded.content_hash,
            time_updated = excluded.time_updated",
        params![
            row.id,
            row.run_id,
            row.artifact_id,
            row.scope,
            row.status,
            row.summary,
            row.evidence_level,
            row.confidence,
            payload,
            row.content_hash,
            row.time_created,
            row.time_updated,
        ],
    )?;
    Ok(())
}

/// List memory capsules for a run.
pub fn list_memory_capsules_for_run(
    conn: &Connection,
    run_id: &str,
) -> StoreResult<Vec<MemoryCapsuleRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, run_id, artifact_id, scope, status, summary, evidence_level,
                confidence, payload_json, content_hash, time_created, time_updated
         FROM daemon_memory_capsule WHERE run_id = ?1 ORDER BY time_created ASC, id ASC",
    )?;
    let rows = stmt.query_map(params![run_id], memory_capsule_from_row)?;
    collect_rows(rows)
}

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

fn reasoning_artifact_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ReasoningArtifactRow> {
    let payload_text: Option<String> = row.get(8)?;
    Ok(ReasoningArtifactRow {
        id: row.get(0)?,
        run_id: row.get(1)?,
        role: row.get(2)?,
        kind: row.get(3)?,
        title: row.get(4)?,
        summary: row.get(5)?,
        evidence_level: row.get(6)?,
        confidence: row.get(7)?,
        payload_json: parse_opt_json(8, payload_text)?,
        content_hash: row.get(9)?,
        status: row.get(10)?,
        time_created: row.get(11)?,
        time_updated: row.get(12)?,
    })
}

fn reasoning_edge_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ReasoningEdgeRow> {
    let payload_text: Option<String> = row.get(5)?;
    Ok(ReasoningEdgeRow {
        run_id: row.get(0)?,
        src_artifact_id: row.get(1)?,
        dst_artifact_id: row.get(2)?,
        kind: row.get(3)?,
        weight: row.get(4)?,
        payload_json: parse_opt_json(5, payload_text)?,
        time_created: row.get(6)?,
    })
}

fn reasoning_lane_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ReasoningLaneRow> {
    let artifact_ids_text: String = row.get(5)?;
    let write_scope_text: String = row.get(6)?;
    Ok(ReasoningLaneRow {
        id: row.get(0)?,
        run_id: row.get(1)?,
        role: row.get(2)?,
        strategy: row.get(3)?,
        status: row.get(4)?,
        artifact_ids: parse_json(5, &artifact_ids_text)?,
        write_scope: parse_json(6, &write_scope_text)?,
        worker_id: row.get(7)?,
        confidence: row.get(8)?,
        time_created: row.get(9)?,
        time_updated: row.get(10)?,
    })
}

fn memory_capsule_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<MemoryCapsuleRow> {
    let payload_text: Option<String> = row.get(8)?;
    Ok(MemoryCapsuleRow {
        id: row.get(0)?,
        run_id: row.get(1)?,
        artifact_id: row.get(2)?,
        scope: row.get(3)?,
        status: row.get(4)?,
        summary: row.get(5)?,
        evidence_level: row.get(6)?,
        confidence: row.get(7)?,
        payload_json: parse_opt_json(8, payload_text)?,
        content_hash: row.get(9)?,
        time_created: row.get(10)?,
        time_updated: row.get(11)?,
    })
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
