//! `permission` table CRUD.

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::error::{StoreError, StoreResult};

/// Row in the `permission` table (per-project permission ruleset).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PermissionRow {
    /// FK to `project.id` (also the primary key).
    pub project_id: String,
    /// Creation timestamp (ms since epoch).
    pub time_created: i64,
    /// Last-update timestamp (ms since epoch).
    pub time_updated: i64,
    /// Permission ruleset (JSON).
    pub data: serde_json::Value,
}

/// Insert or replace a permission row.
pub fn upsert_permission(conn: &Connection, row: &PermissionRow) -> StoreResult<()> {
    let data = serde_json::to_string(&row.data)?;
    conn.execute(
        "INSERT INTO permission (project_id, time_created, time_updated, data)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(project_id) DO UPDATE SET
            time_updated = excluded.time_updated,
            data = excluded.data",
        params![row.project_id, row.time_created, row.time_updated, data],
    )?;
    Ok(())
}

/// Read a permission row.
pub fn get_permission(conn: &Connection, project_id: &str) -> StoreResult<Option<PermissionRow>> {
    conn.query_row(
        "SELECT project_id, time_created, time_updated, data
         FROM permission WHERE project_id = ?1",
        params![project_id],
        |row| {
            let data_text: String = row.get(3)?;
            let data: serde_json::Value = serde_json::from_str(&data_text).map_err(|err| {
                rusqlite::Error::FromSqlConversionFailure(
                    3,
                    rusqlite::types::Type::Text,
                    Box::new(err),
                )
            })?;
            Ok(PermissionRow {
                project_id: row.get(0)?,
                time_created: row.get(1)?,
                time_updated: row.get(2)?,
                data,
            })
        },
    )
    .optional()
    .map_err(StoreError::from)
}

/// Delete a permission row.
pub fn delete_permission(conn: &Connection, project_id: &str) -> StoreResult<usize> {
    Ok(conn.execute(
        "DELETE FROM permission WHERE project_id = ?1",
        params![project_id],
    )?)
}
