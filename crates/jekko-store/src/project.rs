//! `project` table CRUD.
//!
//! Ported from `packages/jekko/src/project/project.sql.ts`. JSON columns
//! (`sandboxes`, `commands`) are serialised via [`serde_json`].

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::error::{StoreError, StoreResult};

/// Row in the `project` table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectRow {
    /// Project ID (matches `project.id`).
    pub id: String,
    /// Worktree path.
    pub worktree: String,
    /// Detected VCS (`git` etc.), if any.
    pub vcs: Option<String>,
    /// Human-friendly name.
    pub name: Option<String>,
    /// Icon URL (auto-derived).
    pub icon_url: Option<String>,
    /// Icon URL override (user-set).
    pub icon_url_override: Option<String>,
    /// Icon color.
    pub icon_color: Option<String>,
    /// Creation timestamp (ms since epoch).
    pub time_created: i64,
    /// Last-update timestamp (ms since epoch).
    pub time_updated: i64,
    /// Initialization timestamp (ms since epoch), if any.
    pub time_initialized: Option<i64>,
    /// Configured sandboxes.
    pub sandboxes: Vec<String>,
    /// Optional command shortcuts.
    pub commands: Option<ProjectCommands>,
}

/// Optional project command shortcuts (e.g. `start`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectCommands {
    /// Command run on project boot.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start: Option<String>,
}

/// Insert or replace a project row.
pub fn upsert(conn: &Connection, row: &ProjectRow) -> StoreResult<()> {
    let sandboxes_json = serde_json::to_string(&row.sandboxes)?;
    let commands_json = row
        .commands
        .as_ref()
        .map(serde_json::to_string)
        .transpose()?;
    conn.execute(
        "INSERT INTO project (
            id, worktree, vcs, name, icon_url, icon_url_override, icon_color,
            time_created, time_updated, time_initialized, sandboxes, commands
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
        ON CONFLICT(id) DO UPDATE SET
            worktree = excluded.worktree,
            vcs = excluded.vcs,
            name = excluded.name,
            icon_url = excluded.icon_url,
            icon_url_override = excluded.icon_url_override,
            icon_color = excluded.icon_color,
            time_updated = excluded.time_updated,
            time_initialized = excluded.time_initialized,
            sandboxes = excluded.sandboxes,
            commands = excluded.commands",
        params![
            row.id,
            row.worktree,
            row.vcs,
            row.name,
            row.icon_url,
            row.icon_url_override,
            row.icon_color,
            row.time_created,
            row.time_updated,
            row.time_initialized,
            sandboxes_json,
            commands_json,
        ],
    )?;
    Ok(())
}

/// Read a project row by id.
pub fn get(conn: &Connection, id: &str) -> StoreResult<Option<ProjectRow>> {
    conn.query_row(
        "SELECT id, worktree, vcs, name, icon_url, icon_url_override, icon_color,
                time_created, time_updated, time_initialized, sandboxes, commands
         FROM project WHERE id = ?1",
        params![id],
        row_from_sql,
    )
    .optional()
    .map_err(StoreError::from)
}

/// List all project rows ordered by creation time.
pub fn list(conn: &Connection) -> StoreResult<Vec<ProjectRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, worktree, vcs, name, icon_url, icon_url_override, icon_color,
                time_created, time_updated, time_initialized, sandboxes, commands
         FROM project ORDER BY time_created ASC",
    )?;
    let rows = stmt.query_map([], row_from_sql)?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

/// Delete a project row. Returns the number of rows removed.
pub fn delete(conn: &Connection, id: &str) -> StoreResult<usize> {
    let n = conn.execute("DELETE FROM project WHERE id = ?1", params![id])?;
    Ok(n)
}

fn row_from_sql(row: &rusqlite::Row<'_>) -> rusqlite::Result<ProjectRow> {
    let sandboxes_json: String = row.get(10)?;
    let commands_json: Option<String> = row.get(11)?;
    let sandboxes: Vec<String> = serde_json::from_str(&sandboxes_json).map_err(|err| {
        rusqlite::Error::FromSqlConversionFailure(10, rusqlite::types::Type::Text, Box::new(err))
    })?;
    let commands = commands_json
        .as_deref()
        .map(serde_json::from_str::<ProjectCommands>)
        .transpose()
        .map_err(|err| {
            rusqlite::Error::FromSqlConversionFailure(
                11,
                rusqlite::types::Type::Text,
                Box::new(err),
            )
        })?;
    Ok(ProjectRow {
        id: row.get(0)?,
        worktree: row.get(1)?,
        vcs: row.get(2)?,
        name: row.get(3)?,
        icon_url: row.get(4)?,
        icon_url_override: row.get(5)?,
        icon_color: row.get(6)?,
        time_created: row.get(7)?,
        time_updated: row.get(8)?,
        time_initialized: row.get(9)?,
        sandboxes,
        commands,
    })
}
