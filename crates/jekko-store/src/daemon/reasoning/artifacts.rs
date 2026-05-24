use rusqlite::{params, Connection};

use crate::error::StoreResult;

use super::rows::{ReasoningArtifactRow, ReasoningEdgeRow, ReasoningLaneRow};
use crate::daemon::support::{collect_rows, parse_json, parse_opt_json, serialize_opt};

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
