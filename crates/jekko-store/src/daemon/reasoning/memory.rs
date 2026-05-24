use rusqlite::{params, Connection};

use crate::error::StoreResult;

use super::rows::MemoryCapsuleRow;
use crate::daemon::support::{collect_rows, parse_opt_json, serialize_opt};

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
