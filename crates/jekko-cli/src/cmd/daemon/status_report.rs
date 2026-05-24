use anyhow::{Context, Result};
use jekko_runtime::daemon_transport::{log_path, socket_path};

use super::control::{is_pid_alive, read_pid};
use super::metadata::{db_path, last_line, now_secs, read_metadata, DaemonMetadata};

pub(super) fn status() -> Result<()> {
    let metadata = read_metadata().ok();
    match read_pid() {
        Ok(pid) if is_pid_alive(pid) => {
            println!("status: running");
            println!("pid: {pid}");
            println!("socket: {}", socket_path()?.display());
            println!("log: {}", log_path()?.display());
            print_metadata_status(metadata.as_ref())?;
        }
        Ok(pid) => {
            println!("status: pidfile-present");
            println!("pid: {pid}");
            println!("log: {}", log_path()?.display());
            print_metadata_status(metadata.as_ref())?;
        }
        Err(_) => {
            println!("status: stopped");
            println!("socket: {}", socket_path()?.display());
            println!("log: {}", log_path()?.display());
            print_metadata_status(metadata.as_ref())?;
        }
    }
    Ok(())
}

fn print_metadata_status(metadata: Option<&DaemonMetadata>) -> Result<()> {
    let Some(metadata) = metadata else {
        return Ok(());
    };
    println!("kind: {}", metadata.kind);
    if let Some(run_id) = metadata.run_id.as_deref() {
        println!("run_id: {run_id}");
        if let Some(repo) = metadata.repo.as_ref() {
            println!(
                "events: {}",
                repo.join("target/zyal/runs")
                    .join(run_id)
                    .join("events.jsonl")
                    .display()
            );
        }
        print_durable_run_status(run_id)?;
    }
    Ok(())
}

fn print_durable_run_status(run_id: &str) -> Result<()> {
    let path = db_path();
    if !path.exists() {
        return Ok(());
    }
    let db =
        jekko_store::Db::open(&path).with_context(|| format!("open db at {}", path.display()))?;
    let conn = db.connection();
    if let Some(run) = jekko_store::daemon::get_run(conn, run_id)? {
        println!("durable_status: {}", run.status);
        println!("durable_phase: {}", run.phase);
        if let Some(proofs) = run.spec_json.get("proofs") {
            let active = ["redis_jedis_stage0", "reasoning_benchmark"]
                .iter()
                .copied()
                .filter(|key| proofs.get(*key).and_then(serde_json::Value::as_bool) == Some(true))
                .collect::<Vec<_>>();
            if !active.is_empty() {
                println!("current_proof: {}", active.join(","));
            }
        }
        let model_outcomes = jekko_store::daemon::list_model_outcomes_for_run(conn, run_id)?;
        if let Some(last) = model_outcomes.last() {
            println!("last_model_kind: {}", last.role);
        }
        let used = model_outcomes.len();
        println!("live_calls_used: {used}");
        if let Some(max_calls) = run
            .spec_json
            .get("live_call_budget")
            .and_then(|budget| budget.get("max_calls"))
            .and_then(serde_json::Value::as_u64)
        {
            println!(
                "live_calls_remaining: {}",
                max_calls.saturating_sub(used as u64)
            );
        }
        print_hero_judge_status(run.last_exit_result_json.as_ref());
    }
    print_port_reasoning_status(&db, run_id)
}

fn print_hero_judge_status(hero_judge: Option<&serde_json::Value>) {
    let Some(hero_judge) = hero_judge else {
        return;
    };
    if !(hero_judge.get("generation").is_some() && hero_judge.get("hero_lane_count").is_some()) {
        return;
    }
    println!(
        "hero_judge_generation: {}",
        hero_judge
            .get("generation")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0)
    );
    println!(
        "hero_lane_count: {}",
        hero_judge
            .get("hero_lane_count")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0)
    );
    println!(
        "judge_lane_count: {}",
        hero_judge
            .get("judge_lane_count")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0)
    );
    if let Some(winner) = hero_judge
        .get("frontier_winner")
        .and_then(serde_json::Value::as_str)
    {
        println!("frontier_winner: {winner}");
    }
    println!(
        "knowledge_entry_count: {}",
        hero_judge
            .get("knowledge_entry_count")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0)
    );
    println!(
        "search_receipt_count: {}",
        hero_judge
            .get("search_receipt_count")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0)
    );
    for key in [
        "quality_metrics_jsonl",
        "quality_metrics_csv",
        "quality_trend_json",
        "lane_metrics_jsonl",
        "lane_metrics_csv",
        "hero_metrics_csv",
        "judge_metrics_csv",
        "reviewer_packet_json",
    ] {
        if let Some(path) = hero_judge.get(key).and_then(serde_json::Value::as_str) {
            println!("{key}: {path}");
        }
    }
    if let Some(promoted) = hero_judge
        .get("last_promotion_decision")
        .and_then(|value| value.get("promoted"))
        .and_then(serde_json::Value::as_bool)
    {
        println!("last_promotion_decision_promoted: {promoted}");
    }
}

fn print_port_reasoning_status(db: &jekko_store::Db, run_id: &str) -> Result<()> {
    let conn = db.connection();
    let targets = jekko_store::daemon::list_port_targets_for_run(conn, run_id)?;
    if let Some(target) = targets.first() {
        println!("target: {} -> {}", target.target, target.replacement);
        println!("target_status: {}", target.status);
        if let Some(score) = target.last_audit_score {
            println!("last_jankurai_score: {score}");
        }
    }
    let mut stage_count = 0_usize;
    let mut parity_seed_count = 0_usize;
    for target in &targets {
        stage_count += jekko_store::daemon::list_port_phases_for_target(conn, &target.id)?.len();
        parity_seed_count +=
            jekko_store::daemon::list_parity_cases_for_target(conn, &target.id)?.len();
    }
    println!("current_stage_count: {stage_count}");
    println!("parity_seed_count: {parity_seed_count}");
    let lanes = jekko_store::daemon::list_reasoning_lanes_for_run(conn, run_id)?;
    let active_lanes = lanes
        .iter()
        .filter(|lane| !matches!(lane.status.as_str(), "complete" | "blocked" | "failed"))
        .count();
    if !lanes.is_empty() {
        println!("reasoning_lanes: {}", lanes.len());
        println!("active_lanes: {active_lanes}");
    }
    let artifacts = jekko_store::daemon::list_reasoning_artifacts_for_run(conn, run_id)?;
    if let Some(last) = artifacts.last() {
        println!("reasoning_state: {}", last.kind);
        println!("last_reasoning_artifact: {}", last.id);
    }
    let memory = jekko_store::daemon::list_memory_capsules_for_run(conn, run_id)?;
    if !memory.is_empty() {
        println!("memory_capsules: {}", memory.len());
    }
    let reliability = jekko_store::daemon::list_model_reliability(conn, None)?;
    if let Some(best) = reliability.first() {
        println!(
            "model_reliability_winner: {} {} score={:.3}",
            best.task_kind, best.model_id, best.score
        );
    }
    if let Some(benchmark) = artifacts
        .iter()
        .rev()
        .find(|artifact| artifact.kind == "reasoning_benchmark")
        .and_then(|artifact| artifact.payload_json.as_ref())
    {
        if let Some(winner) = benchmark.get("winner").and_then(serde_json::Value::as_str) {
            println!("benchmark_winner: {winner}");
        }
    }
    if let Ok(meta) = read_metadata() {
        if meta.run_id.as_deref() == Some(run_id) {
            if let Some(path) = meta.event_log_path() {
                if let Some(line) = last_line(&path) {
                    println!("last_event: {line}");
                }
            }
        }
    }
    print_parity_gaps(db, targets)
}

fn print_parity_gaps(
    db: &jekko_store::Db,
    targets: Vec<jekko_store::daemon::PortTargetRow>,
) -> Result<()> {
    let conn = db.connection();
    let mut parity_gaps = 0_usize;
    for target in targets {
        for parity_run in jekko_store::daemon::list_parity_runs_for_target(conn, &target.id)? {
            if let Some(summary) = parity_run.summary_json {
                parity_gaps += summary
                    .get("gaps")
                    .and_then(serde_json::Value::as_array)
                    .map(|gaps| gaps.len())
                    .unwrap_or(0);
            }
        }
    }
    if parity_gaps > 0 {
        println!("parity_gaps: {parity_gaps}");
    } else {
        println!("parity_gaps: 0");
    }
    Ok(())
}

pub(super) fn mark_durable_run_stopped(metadata: &DaemonMetadata, status: &str) -> Result<()> {
    let Some(run_id) = metadata.run_id.as_deref() else {
        return Ok(());
    };
    let path = db_path();
    if !path.exists() {
        return Ok(());
    }
    let db =
        jekko_store::Db::open(&path).with_context(|| format!("open db at {}", path.display()))?;
    let conn = db.connection();
    let Some(mut row) = jekko_store::daemon::get_run(conn, run_id)? else {
        return Ok(());
    };
    row.status = status.to_string();
    row.phase = status.to_string();
    row.stopped_at = Some(now_secs() as i64);
    row.time_updated = now_secs() as i64;
    jekko_store::daemon::upsert_run(conn, &row)?;
    Ok(())
}
