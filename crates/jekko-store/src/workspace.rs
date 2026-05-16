//! `workspace` CRUD.
//!
//! Ported from `packages/jekko/src/control-plane/workspace.sql.ts`.

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::error::{StoreError, StoreResult};

/// Row in the `workspace` table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceRow {
    /// Workspace id.
    pub id: String,
    /// Workspace kind/type tag (free-form string in TS).
    #[serde(rename = "type")]
    pub kind: String,
    /// Human-friendly name (defaults to "").
    pub name: String,
    /// Optional branch.
    pub branch: Option<String>,
    /// Optional directory (worktree subpath).
    pub directory: Option<String>,
    /// Optional extra JSON payload.
    pub extra: Option<serde_json::Value>,
    /// FK to `project.id`.
    pub project_id: String,
}

/// Insert or replace a workspace row.
pub fn upsert(conn: &Connection, row: &WorkspaceRow) -> StoreResult<()> {
    let extra_json = row.extra.as_ref().map(serde_json::to_string).transpose()?;
    conn.execute(
        "INSERT INTO workspace (id, type, name, branch, directory, extra, project_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT(id) DO UPDATE SET
            type = excluded.type,
            name = excluded.name,
            branch = excluded.branch,
            directory = excluded.directory,
            extra = excluded.extra,
            project_id = excluded.project_id",
        params![
            row.id,
            row.kind,
            row.name,
            row.branch,
            row.directory,
            extra_json,
            row.project_id,
        ],
    )?;
    Ok(())
}

/// Read a workspace row by id.
pub fn get(conn: &Connection, id: &str) -> StoreResult<Option<WorkspaceRow>> {
    conn.query_row(
        "SELECT id, type, name, branch, directory, extra, project_id
         FROM workspace WHERE id = ?1",
        params![id],
        row_from_sql,
    )
    .optional()
    .map_err(StoreError::from)
}

/// List workspaces for a given project.
pub fn list_for_project(conn: &Connection, project_id: &str) -> StoreResult<Vec<WorkspaceRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, type, name, branch, directory, extra, project_id
         FROM workspace WHERE project_id = ?1 ORDER BY id ASC",
    )?;
    let rows = stmt.query_map(params![project_id], row_from_sql)?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

/// Delete a workspace row.
pub fn delete(conn: &Connection, id: &str) -> StoreResult<usize> {
    Ok(conn.execute("DELETE FROM workspace WHERE id = ?1", params![id])?)
}

fn row_from_sql(row: &rusqlite::Row<'_>) -> rusqlite::Result<WorkspaceRow> {
    let extra_text: Option<String> = row.get(5)?;
    let extra = extra_text
        .as_deref()
        .map(serde_json::from_str)
        .transpose()
        .map_err(|err| {
            rusqlite::Error::FromSqlConversionFailure(5, rusqlite::types::Type::Text, Box::new(err))
        })?;
    Ok(WorkspaceRow {
        id: row.get(0)?,
        kind: row.get(1)?,
        name: row.get(2)?,
        branch: row.get(3)?,
        directory: row.get(4)?,
        extra,
        project_id: row.get(6)?,
    })
}
