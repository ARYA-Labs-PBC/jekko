//! Build a `RunSummary` by folding events.jsonl + adjacent artifacts.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde_json::Value;

use super::types::{
    BalancerSummary, BudgetSummary, HaltReason, ManifestInfo, ModelCallStats, PipelineProgress,
    RunSummary, SignalRow,
};

/// Build a summary by reading the run directory's events.jsonl + adjacent
/// artifacts. Tolerant of missing/partial files — surfaces what exists.
pub fn build(run_dir: &Path) -> Result<RunSummary> {
    let run_id = run_dir
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();
    let mut summary = RunSummary::empty(&run_id);
    summary.populate_artifact_paths(run_dir);

    let events_path = run_dir.join("events.jsonl");
    if !events_path.exists() {
        // No events to fold — return a sparse summary the caller can still write.
        summary.terminal_status = "halted".to_string();
        summary.operator_next_steps =
            vec!["No events.jsonl found. Was the run started?".to_string()];
        return Ok(summary);
    }

    let raw = fs::read_to_string(&events_path)
        .with_context(|| format!("read {}", events_path.display()))?;

    let mut stats = ModelCallStats::default();
    let mut progress = PipelineProgress::default();
    let mut budget = BudgetSummary::default();
    let mut latencies: Vec<u64> = Vec::new();
    let mut last_state: Option<String> = None;
    let mut current_kind: Option<String> = None;
    let mut empty_streak: Option<EmptyStreakWitness> = None;
    let mut empty_streak_seen = false;
    let mut last_ts: Option<u64> = None;
    let mut first_ts: Option<u64> = None;
    let mut workflow: Option<String> = None;
    let _hero_judge_finalize: Option<Value> = None;
    let mut gates_observed: BTreeMap<String, String> = BTreeMap::new();
    let mut signal_counts: BTreeMap<&'static str, u64> = BTreeMap::new();
    let mut signal_evidence: BTreeMap<&'static str, Value> = BTreeMap::new();

    for (idx, line) in raw.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let event: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue, // skip malformed lines
        };
        let kind = event.get("kind").and_then(Value::as_str).unwrap_or("");
        let ts = event.get("ts").and_then(Value::as_u64);
        if let Some(t) = ts {
            if first_ts.is_none() {
                first_ts = Some(t);
            }
            last_ts = Some(t);
        }
        let data = event.get("data").cloned().unwrap_or(Value::Null);

        match kind {
            "run_started" => {
                workflow = data
                    .get("workflow")
                    .and_then(Value::as_str)
                    .map(String::from);
            }
            "reasoning_state" => {
                if let Some(state) = data.get("state").and_then(Value::as_str) {
                    last_state = Some(state.to_string());
                    if !progress.stages_reached.iter().any(|s| s == state) {
                        progress.stages_reached.push(state.to_string());
                    }
                    progress.deepest_stage = Some(state.to_string());
                }
            }
            "reasoning_artifact" => {
                if let Some(kind) = data.get("kind").and_then(Value::as_str) {
                    if !progress.artifacts_produced.iter().any(|a| a == kind) {
                        progress.artifacts_produced.push(kind.to_string());
                    }
                }
            }
            "model_attempt" => {
                stats.total_attempts += 1;
                current_kind = data.get("kind").and_then(Value::as_str).map(String::from);
                if let Some(k) = &current_kind {
                    *stats.by_kind.entry(k.clone()).or_insert(0) += 1;
                }
            }
            "model_attempt_outcome" => {
                let state = data
                    .get("state")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
                    .to_string();
                *stats.by_state.entry(state.clone()).or_insert(0) += 1;
                match state.as_str() {
                    "parsed" => stats.parsed_outcomes += 1,
                    "retryable_failure" => stats.retryable_failures += 1,
                    "final_block" => stats.final_blocks += 1,
                    _ => {}
                }
                if data.get("response_bytes").and_then(Value::as_u64) == Some(0) {
                    stats.empty_responses += 1;
                    let stage = data
                        .get("kind")
                        .and_then(Value::as_str)
                        .map(String::from)
                        .or_else(|| current_kind.clone());
                    let provider = data
                        .get("provider")
                        .and_then(Value::as_str)
                        .map(String::from);
                    let user = data
                        .get("credential_user_id")
                        .or_else(|| data.get("selected_credential_user_id"))
                        .and_then(Value::as_str)
                        .map(String::from);
                    empty_streak
                        .get_or_insert_with(|| EmptyStreakWitness::new(stage.clone()))
                        .observe(stage, provider, user, ts);
                } else if let Some(streak) = empty_streak.as_mut() {
                    streak.reset();
                }
                if let Some(user) = data
                    .get("credential_user_id")
                    .or_else(|| data.get("selected_credential_user_id"))
                    .and_then(Value::as_str)
                {
                    *stats.by_user.entry(user.to_string()).or_insert(0) += 1;
                }
                if let Some(provider) = data.get("provider").and_then(Value::as_str) {
                    *stats.by_provider.entry(provider.to_string()).or_insert(0) += 1;
                }
                if let Some(band) = data.get("quality_band").and_then(Value::as_str) {
                    *stats.by_quality_band.entry(band.to_string()).or_insert(0) += 1;
                }
                if let Some(latency) = data.get("latency_ms").and_then(Value::as_u64) {
                    latencies.push(latency);
                }
            }
            "live_budget" => {
                let used = data.get("used").and_then(Value::as_u64).unwrap_or(0);
                let remaining = data.get("remaining").and_then(Value::as_u64);
                budget.used = used;
                budget.remaining = remaining;
                if remaining == Some(0) {
                    budget.exhausted = true;
                }
            }
            "empty_response_streak" => {
                empty_streak_seen = true;
                *signal_counts.entry("empty_response_streak").or_insert(0) += 1;
                signal_evidence.insert("empty_response_streak", data.clone());
            }
            "run_finished" => {
                summary.terminal_status = "run_finished".to_string();
                progress.stages_completed.push("final_signoff".to_string());
            }
            "phase_finalized" => {
                if let Some(phase) = data.get("phase").and_then(Value::as_str) {
                    progress.stages_completed.push(phase.to_string());
                } else if let Some(stage) = &last_state {
                    progress.stages_completed.push(stage.clone());
                }
            }
            "proof_passed" => {
                gates_observed.insert("proof_gate".to_string(), "passed".to_string());
                *signal_counts.entry("proof_passed").or_insert(0) += 1;
            }
            "proof_failed" => {
                gates_observed.insert("proof_gate".to_string(), "failed".to_string());
                *signal_counts.entry("proof_failed").or_insert(0) += 1;
            }
            "parity_result" => {
                gates_observed.insert("parity_gate".to_string(), "passed".to_string());
                *signal_counts.entry("parity_result").or_insert(0) += 1;
            }
            "parity_gap" => {
                *signal_counts.entry("parity_gap").or_insert(0) += 1;
            }
            "judge_patch" => {
                *signal_counts.entry("judge_patch").or_insert(0) += 1;
            }
            "promotion_decision" => {
                *signal_counts.entry("promotion_decision").or_insert(0) += 1;
            }
            "audit_result" => {
                gates_observed.insert("jankurai_gate".to_string(), "passed".to_string());
            }
            "jankurai_regression" => {
                gates_observed.insert("jankurai_gate".to_string(), "failed".to_string());
                *signal_counts.entry("jankurai_regression").or_insert(0) += 1;
            }
            "worker_stall" | "worker_quarantine" => {
                *signal_counts
                    .entry("worker_stall_or_quarantine")
                    .or_insert(0) += 1;
            }
            "remediation_triggered" => {
                *signal_counts.entry("remediation_triggered").or_insert(0) += 1;
            }
            _ => {}
        }
        let _ = idx; // silence unused
    }

    if let Some(t0) = first_ts {
        summary.started_at = Some(t0);
    }
    if let Some(t1) = last_ts {
        summary.finished_at = Some(t1);
        summary.duration_seconds = first_ts.map(|t0| t1.saturating_sub(t0));
    }
    summary.pipeline = workflow.unwrap_or_else(|| "unknown".to_string());

    // Sort + percentile the latencies.
    if !latencies.is_empty() {
        latencies.sort_unstable();
        let p50 = latencies[latencies.len() / 2];
        let p95_idx = ((latencies.len() as f64) * 0.95) as usize;
        let p95 = latencies[p95_idx.min(latencies.len() - 1)];
        stats.latency_p50_ms = Some(p50);
        stats.latency_p95_ms = Some(p95);
    }

    summary.model_calls = stats;
    summary.pipeline_progress = progress;
    summary.budget = budget;
    summary.gates = gates_observed;

    // Halt reason — only if we did NOT see a RunFinished event.
    if summary.terminal_status != "run_finished" {
        if let Some(witness) = empty_streak.as_ref() {
            if empty_streak_seen || witness.count >= crate::empty_response_tracker::STREAK_THRESHOLD
            {
                summary.halt_reason = Some(HaltReason {
                    kind: "empty_response_streak".to_string(),
                    stage: witness.stage.clone(),
                    consecutive_attempts: Some(witness.count as u32),
                    providers_tried: witness.providers.iter().cloned().collect(),
                    users_tried: witness.users.iter().cloned().collect(),
                    summary: format!(
                        "Model returned response_bytes=0 across {} consecutive attempts at {}; \
                         declare quality_band:top20 on this stage's model_policy to escalate.",
                        witness.count,
                        witness.stage.as_deref().unwrap_or("unknown_stage")
                    ),
                });
            }
        }
        if summary.halt_reason.is_none() && summary.budget.exhausted {
            summary.halt_reason = Some(HaltReason {
                kind: "budget_exhausted".to_string(),
                stage: summary.pipeline_progress.deepest_stage.clone(),
                consecutive_attempts: None,
                providers_tried: Vec::new(),
                users_tried: Vec::new(),
                summary: format!(
                    "Live-call budget exhausted (used {}). Raise live_call_budget.max_calls \
                     in the manifest if more depth is needed.",
                    summary.budget.used
                ),
            });
            summary.terminal_status = "budget_exhausted".to_string();
        }
        if summary.halt_reason.is_none() && summary.model_calls.final_blocks > 0 {
            summary.halt_reason = Some(HaltReason {
                kind: "final_block".to_string(),
                stage: summary.pipeline_progress.deepest_stage.clone(),
                consecutive_attempts: None,
                providers_tried: Vec::new(),
                users_tried: Vec::new(),
                summary:
                    "A model attempt exhausted its 3-retry budget with a non-recoverable parse \
                     or timeout failure. Inspect the events.jsonl for the failing stage."
                        .to_string(),
            });
        }
    }

    // Canonical signals table — 12 originals + new ones.
    summary.signals_fired = canonical_signal_table(&signal_counts, &signal_evidence);

    // Manifest info heuristically lifted from the workflow label.
    if summary.pipeline.contains("hero_judge") || summary.pipeline.contains("super") {
        summary.manifest = Some(ManifestInfo {
            id: None,
            name: Some("hero-judge superreasoning".to_string()),
            path: Some("docs/ZYAL/examples/34-superreasoning-openqg-foundry.zyal".to_string()),
        });
    } else if summary.pipeline.contains("advanced_port") {
        summary.manifest = Some(ManifestInfo {
            id: None,
            name: Some("advanced reasoning port-run".to_string()),
            path: None,
        });
    }

    summary.operator_next_steps = recommend_next_steps(&summary);
    summary.balancer = BalancerSummary::default(); // populated by caller if it has snapshots

    Ok(summary)
}

fn canonical_signal_table(
    counts: &BTreeMap<&'static str, u64>,
    evidence: &BTreeMap<&'static str, Value>,
) -> Vec<SignalRow> {
    // Order matches the 12 canonical signals in OBSERVABILITY.md + new ones.
    let order: &[(&str, &str)] = &[
        ("1", "model_attempt_outcome_burst"),
        ("2", "balancer_no_rotation"),
        ("3", "parity_gap_open_growth"),
        ("4", "worker_stall_or_quarantine"),
        ("5", "live_budget_exhaustion"),
        ("6", "proof_failed_in_live_lane"),
        ("7", "provider_error_rate_explosion"),
        ("8", "latency_outlier_per_provider"),
        ("9", "jankurai_regression"),
        ("10", "heartbeat_silence"),
        ("11", "parity_result_no_evidence"),
        ("12", "judge_patch_without_proof"),
        ("empty_response_streak", "empty_response_streak"),
        ("proof_passed", "proof_passed"),
        ("parity_result", "parity_result"),
        ("parity_gap", "parity_gap"),
        ("judge_patch", "judge_patch"),
        ("promotion_decision", "promotion_decision"),
        ("remediation_triggered", "remediation_triggered"),
    ];
    let mut out = Vec::with_capacity(order.len());
    for (id, name) in order {
        let count = *counts.get(name).unwrap_or(&0);
        let evidence = evidence.get(name).cloned();
        out.push(SignalRow {
            id: id.to_string(),
            name: name.to_string(),
            count,
            evidence,
        });
    }
    out
}

fn recommend_next_steps(summary: &RunSummary) -> Vec<String> {
    let mut steps = Vec::new();
    if let Some(halt) = &summary.halt_reason {
        match halt.kind.as_str() {
            "empty_response_streak" => {
                steps.push(format!(
                    "Declare `quality_band: top20` on the `{}` stage's model_policy — \
                     see docs/ZYAL/MODEL_QUALITY_BAND.md.",
                    halt.stage.as_deref().unwrap_or("affected")
                ));
            }
            "budget_exhausted" => {
                steps.push(
                    "Raise live_call_budget.max_calls in the run's manifest, or split work \
                     across multiple runs."
                        .to_string(),
                );
            }
            "final_block" => {
                steps.push(
                    "Inspect events.jsonl filter \
                     `select(.kind==\"model_attempt_outcome\" and .data.state==\"final_block\")` \
                     for the failing stage; consider raising JEKKO_MODEL_CALL_TIMEOUT_SECS."
                        .to_string(),
                );
            }
            _ => {}
        }
    }
    if summary.gates.get("jankurai_gate").map(String::as_str) == Some("failed") {
        steps.push(
            "Jankurai audit regressed mid-run. Re-run audit + fix the new finding \
             before re-attempting."
                .to_string(),
        );
    }
    steps
}

struct EmptyStreakWitness {
    stage: Option<String>,
    count: usize,
    providers: std::collections::BTreeSet<String>,
    users: std::collections::BTreeSet<String>,
    first_ts: Option<u64>,
    last_ts: Option<u64>,
}

impl EmptyStreakWitness {
    fn new(stage: Option<String>) -> Self {
        Self {
            stage,
            count: 0,
            providers: std::collections::BTreeSet::new(),
            users: std::collections::BTreeSet::new(),
            first_ts: None,
            last_ts: None,
        }
    }
    fn observe(
        &mut self,
        stage: Option<String>,
        provider: Option<String>,
        user: Option<String>,
        ts: Option<u64>,
    ) {
        if self.stage.is_none() {
            self.stage = stage;
        }
        self.count += 1;
        if let Some(p) = provider {
            self.providers.insert(p);
        }
        if let Some(u) = user {
            self.users.insert(u);
        }
        if self.first_ts.is_none() {
            self.first_ts = ts;
        }
        self.last_ts = ts;
    }
    fn reset(&mut self) {
        self.count = 0;
        self.providers.clear();
        self.users.clear();
        self.first_ts = None;
        self.last_ts = None;
    }
}
