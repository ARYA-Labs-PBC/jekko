//! V2 session/event/message schema (the post-redesign event-sourced layer).
//!
//! Ported from `packages/jekko/src/v2/`. Effect's `Schema.Class` types become
//! plain serde structs. Tagged unions (`Schema.toTaggedUnion("type")`) become
//! `#[serde(tag = "type")]` enums; status unions (`Schema.toTaggedUnion("status")`)
//! become `#[serde(tag = "status")]` enums.

pub mod auth;
pub mod event;
pub mod model;
pub mod schema;
pub mod session;
pub mod session_event;
pub mod session_message;
pub mod tool_output;
