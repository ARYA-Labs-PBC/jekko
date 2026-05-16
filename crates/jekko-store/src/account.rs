//! `account`, `account_state`, `control_account` CRUD.
//!
//! Ported from `packages/jekko/src/account/account.sql.ts`.

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::error::{StoreError, StoreResult};

/// Row in the `account` table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountRow {
    /// Account id.
    pub id: String,
    /// User email.
    pub email: String,
    /// Endpoint URL.
    pub url: String,
    /// Access token (opaque).
    pub access_token: String,
    /// Refresh token (opaque).
    pub refresh_token: String,
    /// Optional token expiry (ms since epoch).
    pub token_expiry: Option<i64>,
    /// Creation timestamp (ms since epoch).
    pub time_created: i64,
    /// Last-update timestamp (ms since epoch).
    pub time_updated: i64,
}

/// Singleton row in the `account_state` table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountStateRow {
    /// Auto-increment id (always 1 in practice).
    pub id: i64,
    /// Active account id.
    pub active_account_id: Option<String>,
    /// Active organisation id.
    pub active_org_id: Option<String>,
}

/// Row in the `control_account` composite-key table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlAccountRow {
    /// User email (part of composite key).
    pub email: String,
    /// Endpoint URL (part of composite key).
    pub url: String,
    /// Access token (opaque).
    pub access_token: String,
    /// Refresh token (opaque).
    pub refresh_token: String,
    /// Optional token expiry (ms since epoch).
    pub token_expiry: Option<i64>,
    /// Whether this control account is active.
    pub active: bool,
    /// Creation timestamp (ms since epoch).
    pub time_created: i64,
    /// Last-update timestamp (ms since epoch).
    pub time_updated: i64,
}

/// Insert or replace an account row.
pub fn upsert(conn: &Connection, row: &AccountRow) -> StoreResult<()> {
    conn.execute(
        "INSERT INTO account (
            id, email, url, access_token, refresh_token, token_expiry,
            time_created, time_updated
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
        ON CONFLICT(id) DO UPDATE SET
            email = excluded.email,
            url = excluded.url,
            access_token = excluded.access_token,
            refresh_token = excluded.refresh_token,
            token_expiry = excluded.token_expiry,
            time_updated = excluded.time_updated",
        params![
            row.id,
            row.email,
            row.url,
            row.access_token,
            row.refresh_token,
            row.token_expiry,
            row.time_created,
            row.time_updated,
        ],
    )?;
    Ok(())
}

/// Read an account row by id.
pub fn get(conn: &Connection, id: &str) -> StoreResult<Option<AccountRow>> {
    conn.query_row(
        "SELECT id, email, url, access_token, refresh_token, token_expiry,
                time_created, time_updated
         FROM account WHERE id = ?1",
        params![id],
        |row| {
            Ok(AccountRow {
                id: row.get(0)?,
                email: row.get(1)?,
                url: row.get(2)?,
                access_token: row.get(3)?,
                refresh_token: row.get(4)?,
                token_expiry: row.get(5)?,
                time_created: row.get(6)?,
                time_updated: row.get(7)?,
            })
        },
    )
    .optional()
    .map_err(StoreError::from)
}

/// List all account rows ordered by creation time.
pub fn list(conn: &Connection) -> StoreResult<Vec<AccountRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, email, url, access_token, refresh_token, token_expiry,
                time_created, time_updated
         FROM account ORDER BY time_created ASC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(AccountRow {
            id: row.get(0)?,
            email: row.get(1)?,
            url: row.get(2)?,
            access_token: row.get(3)?,
            refresh_token: row.get(4)?,
            token_expiry: row.get(5)?,
            time_created: row.get(6)?,
            time_updated: row.get(7)?,
        })
    })?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

/// Delete an account row. Returns the number of rows removed.
pub fn delete(conn: &Connection, id: &str) -> StoreResult<usize> {
    Ok(conn.execute("DELETE FROM account WHERE id = ?1", params![id])?)
}

/// Fetch (or create) the singleton `account_state` row.
///
/// The TS code mirrors this with a default row keyed on `id = 1`.
pub fn read_state(conn: &Connection) -> StoreResult<Option<AccountStateRow>> {
    conn.query_row(
        "SELECT id, active_account_id, active_org_id FROM account_state WHERE id = 1",
        [],
        |row| {
            Ok(AccountStateRow {
                id: row.get(0)?,
                active_account_id: row.get(1)?,
                active_org_id: row.get(2)?,
            })
        },
    )
    .optional()
    .map_err(StoreError::from)
}

/// Upsert the singleton account state row.
pub fn write_state(conn: &Connection, state: &AccountStateRow) -> StoreResult<()> {
    conn.execute(
        "INSERT INTO account_state (id, active_account_id, active_org_id)
         VALUES (?1, ?2, ?3)
         ON CONFLICT(id) DO UPDATE SET
            active_account_id = excluded.active_account_id,
            active_org_id = excluded.active_org_id",
        params![state.id, state.active_account_id, state.active_org_id],
    )?;
    Ok(())
}

/// Insert or replace a `control_account` row (composite key: email, url).
pub fn upsert_control(conn: &Connection, row: &ControlAccountRow) -> StoreResult<()> {
    conn.execute(
        "INSERT INTO control_account (
            email, url, access_token, refresh_token, token_expiry, active,
            time_created, time_updated
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
        ON CONFLICT(email, url) DO UPDATE SET
            access_token = excluded.access_token,
            refresh_token = excluded.refresh_token,
            token_expiry = excluded.token_expiry,
            active = excluded.active,
            time_updated = excluded.time_updated",
        params![
            row.email,
            row.url,
            row.access_token,
            row.refresh_token,
            row.token_expiry,
            row.active as i64,
            row.time_created,
            row.time_updated,
        ],
    )?;
    Ok(())
}

/// Read a `control_account` row.
pub fn get_control(
    conn: &Connection,
    email: &str,
    url: &str,
) -> StoreResult<Option<ControlAccountRow>> {
    conn.query_row(
        "SELECT email, url, access_token, refresh_token, token_expiry, active,
                time_created, time_updated
         FROM control_account WHERE email = ?1 AND url = ?2",
        params![email, url],
        |row| {
            let active: i64 = row.get(5)?;
            Ok(ControlAccountRow {
                email: row.get(0)?,
                url: row.get(1)?,
                access_token: row.get(2)?,
                refresh_token: row.get(3)?,
                token_expiry: row.get(4)?,
                active: active != 0,
                time_created: row.get(6)?,
                time_updated: row.get(7)?,
            })
        },
    )
    .optional()
    .map_err(StoreError::from)
}
