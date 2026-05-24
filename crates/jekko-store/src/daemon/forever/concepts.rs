use rusqlite::{params, Connection, OptionalExtension};

use crate::error::{StoreError, StoreResult};

use super::rows::{DaemonConceptLinkRow, DaemonConceptRow};
use crate::daemon::support::{collect_rows, parse_opt_json};

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
