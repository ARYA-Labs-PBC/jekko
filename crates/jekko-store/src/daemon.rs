//! Daemon runtime tables.
//!
//! Ported from `packages/jekko/src/session/daemon.sql.ts`. All JSON columns
//! are treated as opaque [`serde_json::Value`] so the caller can decode into
//! their domain-specific shape at use-site.
//!
//! The implementation is split per-table under [`daemon`](self) submodules
//! (each module owns one row struct and its CRUD). All public types and
//! helpers are re-exported here so the original `jekko_store::daemon::*`
//! import paths continue to work unchanged.

pub mod artifact;
pub mod event;
pub mod iteration;
pub mod run;
pub mod task;
pub mod task_memory;
pub mod task_pass;
pub mod worker;

pub use artifact::{upsert_artifact, DaemonArtifactRow};
pub use event::{insert_event, list_events_for_run, DaemonEventRow};
pub use iteration::{get_iteration, upsert_iteration, DaemonIterationRow};
pub use run::{delete_run, get_run, upsert_run, DaemonRunRow};
pub use task::{delete_task, get_task, upsert_task, DaemonTaskRow};
pub use task_memory::{upsert_task_memory, DaemonTaskMemoryRow};
pub use task_pass::{get_task_pass, upsert_task_pass, DaemonTaskPassRow};
pub use worker::{upsert_worker, DaemonWorkerRow};
