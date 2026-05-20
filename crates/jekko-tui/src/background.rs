//! Background-job manager (T-BG-COUNT-MANAGER).
//!
//! Tracks long-running spawned processes (background shell commands, daemons,
//! detached terminals, etc.) so `/ps`, `/stop`, and the working-strip UI can
//! report on them. The manager owns metadata + a cancellation token per job;
//! the actual spawn lives with the caller (e.g. a `PlainCommand` runner).
//!
//! Lifecycle:
//!   1. Caller invokes [`BackgroundJobManager::register`] just before/after
//!      spawning a process. Gets back a [`JobId`] + a [`CancellationToken`]
//!      cloned for the runner to poll.
//!   2. When the process exits, the caller invokes
//!      [`BackgroundJobManager::finalize`] with the resulting status.
//!   3. UI surfaces ([`crate::inline_runtime`] slash dispatch, working strip)
//!      read [`BackgroundJobManager::list`] / [`BackgroundJobManager::count`].
//!   4. The runtime periodically calls
//!      [`BackgroundJobManager::sweep_completed`] to trim stale entries from
//!      the list so `/ps` stays focused on what's interesting.
//!
//! Explicit registration only — auto-promotion of long-running tools into the
//! manager is a separate follow-up (see report at end of T-BG-COUNT-MANAGER).

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::engine::cancel::CancellationToken;

/// Stable monotonic identifier assigned by the manager. Surfaced in `/ps` and
/// accepted by `/stop <id>` to target a specific job.
pub type JobId = u64;

/// Read-only snapshot of one job for display.
///
/// `elapsed` is computed at snapshot time relative to `started_at`, so callers
/// don't need to clone an [`Instant`] just to format the duration.
#[derive(Debug, Clone)]
pub struct JobSummary {
    pub id: JobId,
    pub name: String,
    pub started_at: Instant,
    pub elapsed: Duration,
    pub pid: Option<u32>,
    pub status: JobStatus,
}

/// Job lifecycle state. `Failed` carries a brief error message so `/ps` can
/// display "failed: <msg>" without losing the failure reason on the floor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobStatus {
    Running,
    Completed,
    Cancelled,
    /// Brief error message — display-only; the full error trail belongs in
    /// the transcript.
    Failed(String),
}

impl JobStatus {
    fn is_terminal(&self) -> bool {
        !matches!(self, JobStatus::Running)
    }
}

/// Internal record. Holds the cancellation token clone so `stop()` can fire
/// it without going back to the caller.
struct BackgroundJob {
    id: JobId,
    name: String,
    started_at: Instant,
    pid: Option<u32>,
    status: JobStatus,
    cancel: CancellationToken,
    /// Wall-clock instant at which the job transitioned to a terminal state
    /// (Completed/Cancelled/Failed). Used by [`BackgroundJobManager::sweep_completed`]
    /// to age out finished entries.
    finished_at: Option<Instant>,
}

/// Thread-safe background-job registry. Inexpensive to clone via `Arc` when a
/// caller wants to hand the manager to a spawner — the `Mutex` covers the
/// entire job table so reads + writes are mutually exclusive but the critical
/// sections are O(N) over a tiny N (humans rarely run >10 background jobs).
#[derive(Default)]
pub struct BackgroundJobManager {
    inner: Mutex<ManagerInner>,
}

#[derive(Default)]
struct ManagerInner {
    next_id: JobId,
    jobs: HashMap<JobId, BackgroundJob>,
}

impl BackgroundJobManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a new job. Caller is responsible for the actual spawn (e.g.
    /// via `PlainCommand`) — this manager just tracks metadata + lifecycle.
    ///
    /// Returns the assigned id (monotonically increasing from 1) and a
    /// freshly-minted [`CancellationToken`] cloned for the runner to poll.
    pub fn register(&self, name: String, pid: Option<u32>) -> (JobId, CancellationToken) {
        let token = CancellationToken::new();
        let mut inner = self.inner.lock().expect("bg manager mutex poisoned");
        inner.next_id += 1;
        let id = inner.next_id;
        inner.jobs.insert(
            id,
            BackgroundJob {
                id,
                name,
                started_at: Instant::now(),
                pid,
                status: JobStatus::Running,
                cancel: token.clone(),
                finished_at: None,
            },
        );
        (id, token)
    }

    /// Mark a job as completed (or failed). Idempotent — repeated calls keep
    /// the original `finished_at` timestamp so sweeps don't reset.
    pub fn finalize(&self, id: JobId, status: JobStatus) {
        let mut inner = self.inner.lock().expect("bg manager mutex poisoned");
        if let Some(job) = inner.jobs.get_mut(&id) {
            if job.finished_at.is_none() && status.is_terminal() {
                job.finished_at = Some(Instant::now());
            }
            job.status = status;
        }
    }

    /// Cancel a job by id. Fires the cancellation token (the runner's poll
    /// loop is responsible for noticing + tearing down the child) and flips
    /// the status to [`JobStatus::Cancelled`]. Returns `true` if the id was
    /// found (idempotent — calling twice is fine; second call is a no-op).
    pub fn stop(&self, id: JobId) -> bool {
        let mut inner = self.inner.lock().expect("bg manager mutex poisoned");
        let Some(job) = inner.jobs.get_mut(&id) else {
            return false;
        };
        job.cancel.cancel_hard();
        if job.finished_at.is_none() {
            job.finished_at = Some(Instant::now());
        }
        job.status = JobStatus::Cancelled;
        true
    }

    /// Snapshot of current jobs for display. Sorted by id so `/ps` output is
    /// stable across calls (ids are monotonic — natural chronological order).
    pub fn list(&self) -> Vec<JobSummary> {
        let inner = self.inner.lock().expect("bg manager mutex poisoned");
        let now = Instant::now();
        let mut summaries: Vec<JobSummary> = inner
            .jobs
            .values()
            .map(|job| JobSummary {
                id: job.id,
                name: job.name.clone(),
                started_at: job.started_at,
                elapsed: now.saturating_duration_since(job.started_at),
                pid: job.pid,
                status: job.status.clone(),
            })
            .collect();
        summaries.sort_by_key(|s| s.id);
        summaries
    }

    /// Count of RUNNING jobs only (not completed/cancelled/failed). Drives
    /// `InlineRuntimeOptions::background_count` so the working strip only
    /// counts live work.
    pub fn count(&self) -> usize {
        let inner = self.inner.lock().expect("bg manager mutex poisoned");
        inner
            .jobs
            .values()
            .filter(|job| job.status == JobStatus::Running)
            .count()
    }

    /// Sweep completed/cancelled/failed jobs whose `finished_at` is older
    /// than `retain` from the table. Running jobs are never removed. Call
    /// periodically from the runtime tick.
    pub fn sweep_completed(&self, retain: Duration) {
        let mut inner = self.inner.lock().expect("bg manager mutex poisoned");
        let now = Instant::now();
        inner.jobs.retain(|_, job| match job.finished_at {
            Some(finished) => now.saturating_duration_since(finished) < retain,
            None => true,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::cancel::CancelLevel;

    #[test]
    fn manager_registers_and_assigns_unique_ids() {
        let mgr = BackgroundJobManager::new();
        let (id_a, _) = mgr.register("alpha".into(), None);
        let (id_b, _) = mgr.register("beta".into(), Some(4242));
        assert_ne!(id_a, id_b);
        assert!(id_b > id_a, "ids monotonically increase");
        assert_eq!(mgr.list().len(), 2);
    }

    #[test]
    fn register_returns_cancellation_token() {
        let mgr = BackgroundJobManager::new();
        let (_, token) = mgr.register("worker".into(), None);
        // Fresh token starts un-cancelled.
        assert_eq!(token.level(), CancelLevel::None);
        assert!(!token.is_cancelled());
    }

    #[test]
    fn finalize_updates_status() {
        let mgr = BackgroundJobManager::new();
        let (id, _) = mgr.register("worker".into(), None);
        mgr.finalize(id, JobStatus::Completed);
        let jobs = mgr.list();
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].status, JobStatus::Completed);
    }

    #[test]
    fn stop_signals_cancellation_token() {
        let mgr = BackgroundJobManager::new();
        let (id, token) = mgr.register("worker".into(), None);
        assert!(!token.is_cancelled());
        assert!(mgr.stop(id));
        // The token clone the runner held now reports cancelled.
        assert!(token.is_cancelled());
        assert_eq!(token.level(), CancelLevel::Hard);
        // Status flipped to Cancelled in the registry.
        let jobs = mgr.list();
        assert_eq!(jobs[0].status, JobStatus::Cancelled);
    }

    #[test]
    fn stop_unknown_id_returns_false() {
        let mgr = BackgroundJobManager::new();
        assert!(!mgr.stop(9999), "unknown id must yield false");
    }

    #[test]
    fn list_returns_all_jobs_in_id_order() {
        let mgr = BackgroundJobManager::new();
        let (id_a, _) = mgr.register("alpha".into(), None);
        let (id_b, _) = mgr.register("beta".into(), None);
        let (id_c, _) = mgr.register("gamma".into(), None);
        let jobs = mgr.list();
        let ids: Vec<JobId> = jobs.iter().map(|j| j.id).collect();
        assert_eq!(ids, vec![id_a, id_b, id_c]);
        let names: Vec<&str> = jobs.iter().map(|j| j.name.as_str()).collect();
        assert_eq!(names, vec!["alpha", "beta", "gamma"]);
    }

    #[test]
    fn count_excludes_completed_jobs() {
        let mgr = BackgroundJobManager::new();
        let (a, _) = mgr.register("a".into(), None);
        let (_b, _) = mgr.register("b".into(), None);
        let (c, _) = mgr.register("c".into(), None);
        assert_eq!(mgr.count(), 3);
        mgr.finalize(a, JobStatus::Completed);
        assert_eq!(mgr.count(), 2);
        mgr.finalize(c, JobStatus::Failed("oom".into()));
        assert_eq!(mgr.count(), 1);
    }

    #[test]
    fn sweep_completed_removes_old_entries() {
        let mgr = BackgroundJobManager::new();
        let (running, _) = mgr.register("running".into(), None);
        let (done, _) = mgr.register("done".into(), None);
        mgr.finalize(done, JobStatus::Completed);
        // Sweep with a very short retain so the `done` job ages out, but the
        // `running` one survives. Sleep just past the threshold.
        std::thread::sleep(Duration::from_millis(20));
        mgr.sweep_completed(Duration::from_millis(10));
        let remaining: Vec<JobId> = mgr.list().into_iter().map(|j| j.id).collect();
        assert!(remaining.contains(&running));
        assert!(!remaining.contains(&done), "completed job should be swept");
    }

    #[test]
    fn sweep_keeps_recent_completed_entries() {
        // Sanity check that the retain window is honored (don't sweep a
        // job that just finished).
        let mgr = BackgroundJobManager::new();
        let (id, _) = mgr.register("just-done".into(), None);
        mgr.finalize(id, JobStatus::Completed);
        mgr.sweep_completed(Duration::from_secs(60));
        assert_eq!(mgr.list().len(), 1);
    }

    #[test]
    fn finalize_unknown_id_is_noop() {
        let mgr = BackgroundJobManager::new();
        // Should not panic / poison the mutex.
        mgr.finalize(404, JobStatus::Completed);
        assert!(mgr.list().is_empty());
    }
}
