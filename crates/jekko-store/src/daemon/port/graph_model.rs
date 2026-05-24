use rusqlite::{params, Connection};

use super::rows::{ModelOutcomeRow, RepoGraphEdgeRow, RepoGraphNodeRow};
use crate::daemon::support::{collect_rows, parse_opt_json, serialize_opt};
use crate::error::StoreResult;

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
