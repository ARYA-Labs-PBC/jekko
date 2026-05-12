//! Headless jankurai forever-runner. Drives `jankurai audit` to zero by
//! classifying findings, scheduling parallel waves through a path-overlap DAG,
//! running work in isolated git worktrees, committing on green, rolling back
//! on red, and emitting an NDJSON event stream.
//!
//! At PR3 the crate ships standalone — no daemon-TS bridge yet. The
//! `runner::tick` loop is fully orchestrable in dry-run mode for tests; the
//! daemon-side glue lands in PR4 by tailing `agent/zyal/runner-events.jsonl`.

pub mod bootstrap_check;
pub mod classifier;
pub mod commit;
pub mod dag;
pub mod events;
pub mod locks;
pub mod receipts;
pub mod rollback;
pub mod runner;
pub mod worktree;
