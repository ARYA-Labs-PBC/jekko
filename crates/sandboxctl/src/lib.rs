//! sandboxctl — declarative sandbox-loop runtime.
//!
//! Public surface for embedding the spec parser and permission matcher in
//! other tools (e.g. jankurai integrations, the TUI). Binary entrypoint lives
//! in `src/main.rs`.

pub mod backend;
pub mod permission;
pub mod runid;
pub mod spec;
pub mod spec_types;
pub mod wrapper;
