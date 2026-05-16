//! `session`, `message`, `part`, `pending`, `session_message`, `permission` CRUD.
//!
//! Ported from `packages/jekko/src/session/session.sql.ts`. JSON columns
//! (`summary_diffs`, `revert`, `permission`, `model`, `data`) are treated as
//! free-form [`serde_json::Value`] so callers can decode into typed structs
//! at use-site without coupling this crate to every variant.

mod message;
mod part;
mod pending;
mod permission;
mod session_message;
mod session_table;

pub use message::{delete_message, get_message, list_messages, upsert_message, MessageRow};
pub use part::{delete_part, get_part, list_parts, upsert_part, PartRow};
pub use pending::{delete_pending, list_pending, upsert_pending, PendingRow};
pub use permission::{delete_permission, get_permission, upsert_permission, PermissionRow};
pub use session_message::{
    get_session_message, list_session_messages, upsert_session_message, SessionMessageRow,
};
pub use session_table::{delete, get, list_for_project, upsert, SessionRow};
