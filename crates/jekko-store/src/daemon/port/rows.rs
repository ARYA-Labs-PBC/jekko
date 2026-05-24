use serde::{Deserialize, Serialize};

/// Row in `daemon_port_target`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PortTargetRow {
    /// Target id.
    pub id: String,
    /// Owning daemon run.
    pub run_id: String,
    /// Reference system being ported.
    pub target: String,
    /// Candidate replacement system.
    pub replacement: String,
    /// Reference repository path or URL.
    pub target_repo: Option<String>,
    /// Candidate repository path or URL.
    pub replacement_repo: Option<String>,
    /// Original user request.
    pub request: String,
    /// Workflow status.
    pub status: String,
    /// Current phase id.
    pub current_phase_id: Option<String>,
    /// Maximum worker count.
    pub worker_cap: i64,
    /// Last Jankurai score.
    pub last_audit_score: Option<f64>,
    /// Last parity report payload.
    pub last_parity_report_json: Option<serde_json::Value>,
    /// Last perf gap payload.
    pub last_perf_gap_json: Option<serde_json::Value>,
    /// Rollback status.
    pub rollback_status: String,
    /// Quarantine status.
    pub quarantine_status: String,
    /// Creation timestamp.
    pub time_created: i64,
    /// Last-update timestamp.
    pub time_updated: i64,
}

/// Row in `daemon_port_phase`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PortPhaseRow {
    /// Phase id.
    pub id: String,
    /// Owning daemon run.
    pub run_id: String,
    /// Owning target id.
    pub target_id: String,
    /// Phase order.
    pub ordinal: i64,
    /// Phase name.
    pub name: String,
    /// Phase status.
    pub status: String,
    /// Strategy tag.
    pub strategy: String,
    /// Finalized phase plan.
    pub plan_json: Option<serde_json::Value>,
    /// Number of tasks.
    pub task_count: i64,
    /// Last Jankurai score.
    pub last_audit_score: Option<f64>,
    /// Last parity report payload.
    pub last_parity_report_json: Option<serde_json::Value>,
    /// Creation timestamp.
    pub time_created: i64,
    /// Last-update timestamp.
    pub time_updated: i64,
}

/// Row in `daemon_port_task`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PortTaskRow {
    /// Task id.
    pub id: String,
    /// Owning daemon run.
    pub run_id: String,
    /// Owning phase id.
    pub phase_id: String,
    /// Task title.
    pub title: String,
    /// Task status.
    pub status: String,
    /// Assigned worker id.
    pub worker_id: Option<String>,
    /// Worker branch.
    pub branch: Option<String>,
    /// Declared write scope.
    pub write_scope: Vec<String>,
    /// Proof lane.
    pub proof_lane: Option<String>,
    /// Attempt count.
    pub attempt_count: i64,
    /// Rollback status.
    pub rollback_status: String,
    /// Quarantine reason.
    pub quarantine_reason: Option<String>,
    /// Last error.
    pub last_error: Option<String>,
    /// Creation timestamp.
    pub time_created: i64,
    /// Last-update timestamp.
    pub time_updated: i64,
}

/// Row in `daemon_parity_case`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParityCaseRow {
    /// Case id.
    pub id: String,
    /// Owning daemon run.
    pub run_id: String,
    /// Owning port target id.
    pub target_id: String,
    /// Case tags.
    pub tags: Vec<String>,
    /// Target adapter kind.
    pub target_kind: String,
    /// Target-switched case steps.
    pub steps_json: serde_json::Value,
    /// Performance budget payload.
    pub perf_json: Option<serde_json::Value>,
    /// Whether the case is approved for required gates.
    pub approved: bool,
    /// Creation timestamp.
    pub time_created: i64,
    /// Last-update timestamp.
    pub time_updated: i64,
}

/// Row in `daemon_parity_run`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParityRunRow {
    /// Parity run id.
    pub id: String,
    /// Owning daemon run.
    pub run_id: String,
    /// Owning port target id.
    pub target_id: String,
    /// Number of cases in the report.
    pub case_count: i64,
    /// Run status.
    pub status: String,
    /// Report path.
    pub report_path: Option<String>,
    /// Start timestamp.
    pub started_at: Option<i64>,
    /// End timestamp.
    pub ended_at: Option<i64>,
    /// Summary payload.
    pub summary_json: Option<serde_json::Value>,
    /// Creation timestamp.
    pub time_created: i64,
    /// Last-update timestamp.
    pub time_updated: i64,
}

/// Row in `daemon_parity_result`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParityResultRow {
    /// Result id.
    pub id: String,
    /// Owning parity run id.
    pub parity_run_id: String,
    /// Case id.
    pub case_id: String,
    /// Reference or candidate target name.
    pub target_name: String,
    /// Result status.
    pub status: String,
    /// Whether the case was skipped.
    pub skipped: bool,
    /// Duration in milliseconds.
    pub duration_ms: Option<i64>,
    /// Performance result payload.
    pub perf_json: Option<serde_json::Value>,
    /// Message.
    pub message: Option<String>,
    /// Creation timestamp.
    pub time_created: i64,
}

/// Row in `daemon_perf_budget`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PerfBudgetRow {
    /// Budget id.
    pub id: String,
    /// Owning daemon run.
    pub run_id: String,
    /// Case id.
    pub case_id: String,
    /// Metric name.
    pub metric: String,
    /// Maximum reference-to-candidate ratio.
    pub max_ratio: Option<f64>,
    /// Baseline metric value.
    pub baseline_value: Option<f64>,
    /// Candidate metric value.
    pub candidate_value: Option<f64>,
    /// Budget status.
    pub status: String,
    /// Creation timestamp.
    pub time_created: i64,
    /// Last-update timestamp.
    pub time_updated: i64,
}

/// Row in `daemon_repo_graph_node`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RepoGraphNodeRow {
    /// Node id.
    pub id: String,
    /// Owning daemon run.
    pub run_id: String,
    /// Node kind.
    pub kind: String,
    /// Stable key.
    pub key: String,
    /// Human-readable label.
    pub label: String,
    /// Node payload.
    pub payload_json: Option<serde_json::Value>,
    /// Creation timestamp.
    pub time_created: i64,
    /// Last-update timestamp.
    pub time_updated: i64,
}

/// Row in `daemon_repo_graph_edge`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RepoGraphEdgeRow {
    /// Owning daemon run.
    pub run_id: String,
    /// Source node id.
    pub src_node_id: String,
    /// Destination node id.
    pub dst_node_id: String,
    /// Edge kind.
    pub kind: String,
    /// Edge payload.
    pub payload_json: Option<serde_json::Value>,
    /// Creation timestamp.
    pub time_created: i64,
}

/// Row in `daemon_model_outcome`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelOutcomeRow {
    /// Outcome id.
    pub id: String,
    /// Owning daemon run.
    pub run_id: String,
    /// Task id.
    pub task_id: Option<String>,
    /// Model id.
    pub model_id: String,
    /// Model role.
    pub role: String,
    /// Cost in USD.
    pub cost_usd: Option<f64>,
    /// Latency in milliseconds.
    pub latency_ms: Option<i64>,
    /// Outcome status.
    pub status: String,
    /// Reviewer score.
    pub reviewer_score: Option<f64>,
    /// Whether this outcome became a winner.
    pub winner: bool,
    /// Extra outcome payload.
    pub payload_json: Option<serde_json::Value>,
    /// Creation timestamp.
    pub time_created: i64,
    /// Last-update timestamp.
    pub time_updated: i64,
}
