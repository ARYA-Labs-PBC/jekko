//! cogcore — cognitive memory core.
//!
//! Deterministic, append-only, zero default dependencies. Standalone; does
//! not depend on the benchmark harness. The host wires cogcore into a
//! benchmark via a thin adapter that translates between its native types
//! and cogcore's public API.

pub mod canary;
pub mod concept;
pub mod config;
pub mod core;
pub mod fsrs;
pub mod hash;
pub mod hebb;
pub mod index;
pub mod ledger;
pub mod time;
pub mod topic;

pub use core::{
    pack_hash, ClaimModality, Core, FeedbackSignal, Intent, Outcome, PrivacyClass, RecallData,
    RecallQuery, Receipt, SourceRef, StoredEvent, Tombstone, Warning,
};
pub use time::BENCH_NOW;
