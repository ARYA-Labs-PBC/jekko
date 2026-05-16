//! `event` + `event_sequence` (event-sourcing) CRUD.
//!
//! Ported from `packages/jekko/src/sync/event.sql.ts`. The `data` column is
//! treated as opaque JSON ([`serde_json::Value`]) so callers can decode into
//! whatever event variant they need.

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::error::{StoreError, StoreResult};

/// Row in the `event_sequence` table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventSequenceRow {
    /// Aggregate id (primary key).
    pub aggregate_id: String,
    /// Last-issued sequence number for the aggregate.
    pub seq: i64,
    /// Optional owner id (added in `20260504145000_add_sync_owner`).
    pub owner_id: Option<String>,
}

/// Row in the `event` table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventRow {
    /// Event id (primary key).
    pub id: String,
    /// Aggregate the event belongs to.
    pub aggregate_id: String,
    /// Per-aggregate sequence number.
    pub seq: i64,
    /// Event type tag.
    pub event_type: String,
    /// Event payload (free-form JSON).
    pub data: serde_json::Value,
}

/// Insert or replace a sequence row.
pub fn upsert_sequence(conn: &Connection, row: &EventSequenceRow) -> StoreResult<()> {
    conn.execute(
        "INSERT INTO event_sequence (aggregate_id, seq, owner_id)
         VALUES (?1, ?2, ?3)
         ON CONFLICT(aggregate_id) DO UPDATE SET
            seq = excluded.seq,
            owner_id = excluded.owner_id",
        params![row.aggregate_id, row.seq, row.owner_id],
    )?;
    Ok(())
}

/// Read a sequence row.
pub fn get_sequence(
    conn: &Connection,
    aggregate_id: &str,
) -> StoreResult<Option<EventSequenceRow>> {
    conn.query_row(
        "SELECT aggregate_id, seq, owner_id FROM event_sequence WHERE aggregate_id = ?1",
        params![aggregate_id],
        |row| {
            Ok(EventSequenceRow {
                aggregate_id: row.get(0)?,
                seq: row.get(1)?,
                owner_id: row.get(2)?,
            })
        },
    )
    .optional()
    .map_err(StoreError::from)
}

/// Append an event row.
pub fn insert_event(conn: &Connection, row: &EventRow) -> StoreResult<()> {
    let payload = serde_json::to_string(&row.data)?;
    conn.execute(
        "INSERT INTO event (id, aggregate_id, seq, type, data)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![row.id, row.aggregate_id, row.seq, row.event_type, payload],
    )?;
    Ok(())
}

/// Read an event row by id.
pub fn get_event(conn: &Connection, id: &str) -> StoreResult<Option<EventRow>> {
    conn.query_row(
        "SELECT id, aggregate_id, seq, type, data FROM event WHERE id = ?1",
        params![id],
        |row| {
            let payload: String = row.get(4)?;
            let data: serde_json::Value = serde_json::from_str(&payload).map_err(|err| {
                rusqlite::Error::FromSqlConversionFailure(
                    4,
                    rusqlite::types::Type::Text,
                    Box::new(err),
                )
            })?;
            Ok(EventRow {
                id: row.get(0)?,
                aggregate_id: row.get(1)?,
                seq: row.get(2)?,
                event_type: row.get(3)?,
                data,
            })
        },
    )
    .optional()
    .map_err(StoreError::from)
}

/// List events for an aggregate, optionally above a given sequence number.
pub fn list_events(
    conn: &Connection,
    aggregate_id: &str,
    after_seq: Option<i64>,
) -> StoreResult<Vec<EventRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, aggregate_id, seq, type, data
         FROM event
         WHERE aggregate_id = ?1 AND seq > ?2
         ORDER BY seq ASC",
    )?;
    let cursor = after_seq.unwrap_or(-1);
    let rows = stmt.query_map(params![aggregate_id, cursor], |row| {
        let payload: String = row.get(4)?;
        let data: serde_json::Value = serde_json::from_str(&payload).map_err(|err| {
            rusqlite::Error::FromSqlConversionFailure(4, rusqlite::types::Type::Text, Box::new(err))
        })?;
        Ok(EventRow {
            id: row.get(0)?,
            aggregate_id: row.get(1)?,
            seq: row.get(2)?,
            event_type: row.get(3)?,
            data,
        })
    })?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

/// Delete an event row. Returns the number of rows removed.
pub fn delete_event(conn: &Connection, id: &str) -> StoreResult<usize> {
    Ok(conn.execute("DELETE FROM event WHERE id = ?1", params![id])?)
}
