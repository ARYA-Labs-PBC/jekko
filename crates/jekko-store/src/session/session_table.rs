//! `session` table CRUD.

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::error::{StoreError, StoreResult};

/// Row in the `session` table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionRow {
    /// Session id.
    pub id: String,
    /// FK to `project.id`.
    pub project_id: String,
    /// FK to `workspace.id`, if any.
    pub workspace_id: Option<String>,
    /// FK to the parent `session.id`, if any.
    pub parent_id: Option<String>,
    /// URL-safe slug.
    pub slug: String,
    /// Workspace directory at the time of session creation.
    pub directory: String,
    /// Resolved session path (added in `20260428004200_add_session_path`).
    pub path: Option<String>,
    /// Title.
    pub title: String,
    /// Schema version tag.
    pub version: String,
    /// Public share URL, if any.
    pub share_url: Option<String>,
    /// Summary additions row count.
    pub summary_additions: Option<i64>,
    /// Summary deletions row count.
    pub summary_deletions: Option<i64>,
    /// Summary number of changed files.
    pub summary_files: Option<i64>,
    /// File-diff summary (JSON).
    pub summary_diffs: Option<serde_json::Value>,
    /// Revert pointer (JSON).
    pub revert: Option<serde_json::Value>,
    /// Permission ruleset (JSON).
    pub permission: Option<serde_json::Value>,
    /// Active agent.
    pub agent: Option<String>,
    /// Active model selection (JSON).
    pub model: Option<serde_json::Value>,
    /// Creation timestamp (ms since epoch).
    pub time_created: i64,
    /// Last-update timestamp (ms since epoch).
    pub time_updated: i64,
    /// Timestamp when this session entered the compacting state.
    pub time_compacting: Option<i64>,
    /// Archive timestamp.
    pub time_archived: Option<i64>,
}

/// Insert or replace a session row.
pub fn upsert(conn: &Connection, row: &SessionRow) -> StoreResult<()> {
    let summary_diffs = row
        .summary_diffs
        .as_ref()
        .map(serde_json::to_string)
        .transpose()?;
    let revert = row.revert.as_ref().map(serde_json::to_string).transpose()?;
    let permission = row
        .permission
        .as_ref()
        .map(serde_json::to_string)
        .transpose()?;
    let model = row.model.as_ref().map(serde_json::to_string).transpose()?;

    conn.execute(
        "INSERT INTO session (
            id, project_id, workspace_id, parent_id, slug, directory, path,
            title, version, share_url, summary_additions, summary_deletions,
            summary_files, summary_diffs, revert, permission, agent, model,
            time_created, time_updated, time_compacting, time_archived
        ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
            ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22
        )
        ON CONFLICT(id) DO UPDATE SET
            project_id = excluded.project_id,
            workspace_id = excluded.workspace_id,
            parent_id = excluded.parent_id,
            slug = excluded.slug,
            directory = excluded.directory,
            path = excluded.path,
            title = excluded.title,
            version = excluded.version,
            share_url = excluded.share_url,
            summary_additions = excluded.summary_additions,
            summary_deletions = excluded.summary_deletions,
            summary_files = excluded.summary_files,
            summary_diffs = excluded.summary_diffs,
            revert = excluded.revert,
            permission = excluded.permission,
            agent = excluded.agent,
            model = excluded.model,
            time_updated = excluded.time_updated,
            time_compacting = excluded.time_compacting,
            time_archived = excluded.time_archived",
        params![
            row.id,
            row.project_id,
            row.workspace_id,
            row.parent_id,
            row.slug,
            row.directory,
            row.path,
            row.title,
            row.version,
            row.share_url,
            row.summary_additions,
            row.summary_deletions,
            row.summary_files,
            summary_diffs,
            revert,
            permission,
            row.agent,
            model,
            row.time_created,
            row.time_updated,
            row.time_compacting,
            row.time_archived,
        ],
    )?;
    Ok(())
}

/// Read a session row by id.
pub fn get(conn: &Connection, id: &str) -> StoreResult<Option<SessionRow>> {
    conn.query_row(SELECT_SESSION_SQL, params![id], session_from_row)
        .optional()
        .map_err(StoreError::from)
}

/// List sessions for a project.
pub fn list_for_project(conn: &Connection, project_id: &str) -> StoreResult<Vec<SessionRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, project_id, workspace_id, parent_id, slug, directory, path,
                title, version, share_url, summary_additions, summary_deletions,
                summary_files, summary_diffs, revert, permission, agent, model,
                time_created, time_updated, time_compacting, time_archived
         FROM session WHERE project_id = ?1 ORDER BY time_created ASC",
    )?;
    let rows = stmt.query_map(params![project_id], session_from_row)?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

/// Delete a session row.
pub fn delete(conn: &Connection, id: &str) -> StoreResult<usize> {
    Ok(conn.execute("DELETE FROM session WHERE id = ?1", params![id])?)
}

const SELECT_SESSION_SQL: &str = "SELECT id, project_id, workspace_id, parent_id, slug, directory, path,\n                title, version, share_url, summary_additions, summary_deletions,\n                summary_files, summary_diffs, revert, permission, agent, model,\n                time_created, time_updated, time_compacting, time_archived\n         FROM session WHERE id = ?1";

fn session_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<SessionRow> {
    let summary_diffs_text: Option<String> = row.get(13)?;
    let revert_text: Option<String> = row.get(14)?;
    let permission_text: Option<String> = row.get(15)?;
    let model_text: Option<String> = row.get(17)?;

    let json = |idx: usize, text: Option<&str>| -> rusqlite::Result<Option<serde_json::Value>> {
        text.map(serde_json::from_str).transpose().map_err(|err| {
            rusqlite::Error::FromSqlConversionFailure(
                idx,
                rusqlite::types::Type::Text,
                Box::new(err),
            )
        })
    };

    Ok(SessionRow {
        id: row.get(0)?,
        project_id: row.get(1)?,
        workspace_id: row.get(2)?,
        parent_id: row.get(3)?,
        slug: row.get(4)?,
        directory: row.get(5)?,
        path: row.get(6)?,
        title: row.get(7)?,
        version: row.get(8)?,
        share_url: row.get(9)?,
        summary_additions: row.get(10)?,
        summary_deletions: row.get(11)?,
        summary_files: row.get(12)?,
        summary_diffs: json(13, summary_diffs_text.as_deref())?,
        revert: json(14, revert_text.as_deref())?,
        permission: json(15, permission_text.as_deref())?,
        agent: row.get(16)?,
        model: json(17, model_text.as_deref())?,
        time_created: row.get(18)?,
        time_updated: row.get(19)?,
        time_compacting: row.get(20)?,
        time_archived: row.get(21)?,
    })
}
