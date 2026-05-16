//! Snapshot types fed to the Jankurai panel.

/// One Jankurai worker entry. Mirrors the `useZyalWorkers` row.
#[derive(Clone, Debug)]
pub struct JankuraiWorker {
    /// Worker identifier.
    pub id: String,
    /// Worker kind tag (e.g. `tail`, `score`, `wave`).
    pub kind: String,
}

/// Live Jankurai snapshot displayed by the panel.
#[derive(Clone, Debug, Default)]
pub struct JankuraiSnapshot {
    /// True when `jankurai` binary is found in `PATH`.
    pub jankurai_installed: bool,
    /// Current 0-100 score. `None` until first audit.
    pub score: Option<f64>,
    /// Decision label (`pass`, `fail`).
    pub decision: Option<String>,
    /// Conformance level (`A`, `B`, etc).
    pub conformance_level: Option<String>,
    /// Number of caps applied.
    pub caps_applied: Option<f64>,
    /// Hard findings count.
    pub hard_findings: Option<f64>,
    /// Soft findings count.
    pub soft_findings: Option<f64>,
    /// Auditor binary version.
    pub auditor_version: Option<String>,
    /// Score history for the sparkline (oldest first).
    pub history: Vec<f64>,
    /// Baseline score (main branch).
    pub baseline_score: Option<f64>,
    /// Baseline caps applied.
    pub baseline_caps: Option<f64>,
    /// Baseline hard findings.
    pub baseline_hard: Option<f64>,
    /// Baseline soft findings.
    pub baseline_soft: Option<f64>,
    /// Active workers.
    pub workers: Vec<JankuraiWorker>,
    /// Human-readable "23s" age string.
    pub last_run_age: Option<String>,
}
