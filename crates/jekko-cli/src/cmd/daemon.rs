//! `jekko daemon` — background daemon management.
//!
//! Mirrors `packages/jekko/src/cli/cmd/daemon.ts`.

use std::fs::{self, OpenOptions};
use std::io::{Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use jekko_runtime::daemon_transport::{log_path, pid_path, socket_path};
use serde::{Deserialize, Serialize};

use crate::cli::GlobalOpts;

#[derive(Args, Debug)]
pub struct DaemonArgs {
    #[command(subcommand)]
    pub command: DaemonCommand,
}

#[derive(Subcommand, Debug)]
pub enum DaemonCommand {
    /// Start the daemon.
    Start(DaemonStartArgs),
    /// Stop the daemon.
    Stop,
    /// Print daemon status.
    Status,
    /// Tail daemon logs.
    Logs(DaemonLogsArgs),
}

#[derive(Args, Debug, Default)]
pub struct DaemonStartArgs {
    /// Detach into the background.
    #[arg(long)]
    pub detach: bool,
    /// Run the daemon loop in the foreground.
    #[arg(long, hide = true)]
    pub foreground: bool,
    /// Start a durable ZYAL port run from a JSON/TOML config.
    #[arg(long, value_name = "CONFIG")]
    pub port_run: Option<PathBuf>,
    /// Repository root for `--port-run`.
    #[arg(long, value_name = "PATH")]
    pub repo: Option<PathBuf>,
    /// Run id for `--port-run`.
    #[arg(long)]
    pub run_id: Option<String>,
    /// Use live model calls for `--port-run`.
    #[arg(long)]
    pub live: bool,
    /// Provider override for live `--port-run`.
    #[arg(long)]
    pub provider: Option<String>,
    /// Model override for live `--port-run`.
    #[arg(long)]
    pub model: Option<String>,
    /// Maximum port-run ticks.
    #[arg(long)]
    pub max_ticks: Option<u64>,
    /// Seconds between port-run ticks.
    #[arg(long, default_value_t = 30)]
    pub tick_interval_secs: u64,
    /// Stop-file path for the port runner.
    #[arg(long)]
    pub stop_file: Option<PathBuf>,
    /// Run the port runner until stopped.
    #[arg(long)]
    pub forever: bool,
}

#[derive(Args, Debug, Default)]
pub struct DaemonLogsArgs {
    /// Follow new log lines as they are appended.
    #[arg(long, short = 'f')]
    pub follow: bool,
    /// Number of trailing lines to print.
    #[arg(long, short = 'n', default_value_t = 80)]
    pub lines: usize,
}

pub fn run(_global: &GlobalOpts, args: &DaemonArgs) -> Result<()> {
    match &args.command {
        DaemonCommand::Start(opts) => start(opts),
        DaemonCommand::Stop => stop(),
        DaemonCommand::Status => status(),
        DaemonCommand::Logs(opts) => logs(opts),
    }
}

fn start(args: &DaemonStartArgs) -> Result<()> {
    if args.foreground {
        return foreground_loop();
    }
    if args.port_run.is_some() {
        return start_port_run(args);
    }
    let pid = read_pid().ok();
    if let Some(pid) = pid {
        if is_pid_alive(pid) {
            println!("jekko daemon already running (pid {pid})");
            return Ok(());
        }
    }
    prepare_daemon_dir()?;
    let log = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path()?)
        .context("open daemon log")?;
    let exe = std::env::current_exe().context("resolve current jekko executable")?;
    let child = Command::new(exe)
        .args(["daemon", "start", "--foreground"])
        .stdin(Stdio::null())
        .stdout(Stdio::from(log.try_clone()?))
        .stderr(Stdio::from(log))
        .spawn()
        .context("spawn daemon foreground child")?;
    write_pid(child.id())?;
    append_log(&format!("spawned daemon child pid={}", child.id()))?;
    println!("jekko daemon started");
    println!("pid: {}", child.id());
    println!("socket: {}", socket_path()?.display());
    println!("log: {}", log_path()?.display());
    Ok(())
}

fn stop() -> Result<()> {
    let pid = read_pid().ok();
    let metadata = read_metadata().ok();
    let path = pid_path()?;
    if path.exists() {
        fs::remove_file(&path).with_context(|| format!("remove {}", path.display()))?;
    }
    match pid {
        Some(pid) => {
            let _ = Command::new("kill")
                .arg("-TERM")
                .arg(pid.to_string())
                .status();
            append_log(&format!("stop requested for pid={pid}"))?;
            if let Some(meta) = metadata.as_ref() {
                mark_durable_run_stopped(meta, "stopped")?;
            }
            println!("jekko daemon stop requested (pid {pid})");
        }
        None => {
            println!("jekko daemon not running");
        }
    }
    Ok(())
}

fn status() -> Result<()> {
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

fn logs(args: &DaemonLogsArgs) -> Result<()> {
    let path = match read_metadata().ok().and_then(|meta| meta.event_log_path()) {
        Some(path) if path.exists() => path,
        _ => log_path()?,
    };
    let mut printed = print_tail(&path, args.lines)?;
    if args.follow {
        loop {
            thread::sleep(Duration::from_secs(1));
            let text = fs::read_to_string(&path).unwrap_or_default();
            let lines: Vec<&str> = text.lines().collect();
            for line in lines.iter().skip(printed) {
                println!("{line}");
            }
            printed = lines.len();
        }
    }
    Ok(())
}

fn start_port_run(args: &DaemonStartArgs) -> Result<()> {
    let config = args
        .port_run
        .as_ref()
        .context("--port-run requires a config path")?;
    let repo = match args.repo.clone() {
        Some(repo) => repo,
        None => std::env::current_dir().context("resolve current directory")?,
    };
    let run_id = args.run_id.clone().unwrap_or_else(random_run_id);
    prepare_daemon_dir()?;
    let log = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path()?)
        .context("open daemon log")?;
    let runner = resolve_runner_bin()?;
    let mut command = Command::new(&runner);
    command
        .arg("--repo")
        .arg(&repo)
        .arg("--run-id")
        .arg(&run_id)
        .arg("port-run")
        .arg("--config")
        .arg(config);
    if args.live {
        command.arg("--live");
    }
    if let Some(provider) = args.provider.as_deref() {
        command.arg("--provider").arg(provider);
    }
    if let Some(model) = args.model.as_deref() {
        command.arg("--model").arg(model);
    }
    if let Some(max_ticks) = args.max_ticks {
        command.arg("--max-ticks").arg(max_ticks.to_string());
    } else if args.forever || args.port_run.is_some() {
        command.arg("--forever");
    }
    command
        .arg("--tick-interval-secs")
        .arg(args.tick_interval_secs.to_string());
    if let Some(stop_file) = args.stop_file.as_ref() {
        command.arg("--stop-file").arg(stop_file);
    }
    let child = command
        .stdin(Stdio::null())
        .stdout(Stdio::from(log.try_clone()?))
        .stderr(Stdio::from(log))
        .spawn()
        .with_context(|| format!("spawn {}", runner.display()))?;
    write_pid(child.id())?;
    let meta = DaemonMetadata {
        pid: child.id(),
        kind: "port_run".to_string(),
        run_id: Some(run_id.clone()),
        repo: Some(repo.clone()),
        port_config: Some(config.clone()),
        started_at: now_secs(),
    };
    write_metadata(&meta)?;
    append_log(&format!(
        "spawned port run pid={} run_id={} repo={} config={}",
        child.id(),
        run_id,
        repo.display(),
        config.display()
    ))?;
    println!("jekko daemon started port run");
    println!("pid: {}", child.id());
    println!("run_id: {run_id}");
    println!("events: {}", meta.event_log_path().unwrap().display());
    println!("log: {}", log_path()?.display());
    Ok(())
}

fn foreground_loop() -> Result<()> {
    prepare_daemon_dir()?;
    let pid = std::process::id();
    write_pid(pid)?;
    append_log(&format!("daemon foreground loop started pid={pid}"))?;
    loop {
        thread::sleep(Duration::from_secs(1));
        match read_pid() {
            Ok(current) if current == pid && is_pid_alive(pid) => {}
            _ => break,
        }
    }
    append_log(&format!("daemon foreground loop stopped pid={pid}"))?;
    Ok(())
}

fn prepare_daemon_dir() -> Result<()> {
    if let Some(parent) = pid_path()?.parent() {
        fs::create_dir_all(parent).with_context(|| format!("mkdir {}", parent.display()))?;
    }
    Ok(())
}

fn write_pid(pid: u32) -> Result<()> {
    let path = pid_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("mkdir {}", parent.display()))?;
    }
    fs::write(&path, pid.to_string()).with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

fn metadata_path() -> Result<PathBuf> {
    let base = std::env::var_os("HOME")
        .map(PathBuf::from)
        .context("HOME is required for daemon metadata")?
        .join(".jekko");
    Ok(base.join("jekko-daemon.json"))
}

fn write_metadata(metadata: &DaemonMetadata) -> Result<()> {
    let path = metadata_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("mkdir {}", parent.display()))?;
    }
    fs::write(&path, serde_json::to_string_pretty(metadata)?)
        .with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

fn read_metadata() -> Result<DaemonMetadata> {
    let path = metadata_path()?;
    let text = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    Ok(serde_json::from_str(&text)?)
}

fn read_pid() -> Result<u32> {
    let text = fs::read_to_string(pid_path()?).context("read daemon pid")?;
    Ok(text.trim().parse::<u32>()?)
}

fn is_pid_alive(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }
    Command::new("sh")
        .arg("-c")
        .arg(format!("kill -0 {pid} 2>/dev/null"))
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn append_log(message: &str) -> Result<()> {
    let path = log_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("mkdir {}", parent.display()))?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .with_context(|| format!("open {}", path.display()))?;
    writeln!(file, "{} {message}", now_secs())?;
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
        if let Some(hero_judge) = run.last_exit_result_json.as_ref() {
            if hero_judge.get("generation").is_some() && hero_judge.get("hero_lane_count").is_some()
            {
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
        }
    }
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
    if let Some(meta) = read_metadata().ok() {
        if meta.run_id.as_deref() == Some(run_id) {
            if let Some(path) = meta.event_log_path() {
                if let Some(line) = last_line(&path)? {
                    println!("last_event: {line}");
                }
            }
        }
    }
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

fn mark_durable_run_stopped(metadata: &DaemonMetadata, status: &str) -> Result<()> {
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

fn db_path() -> PathBuf {
    if let Some(path) = std::env::var_os("JEKKO_DB") {
        return path.into();
    }
    match std::env::var_os("HOME") {
        Some(home) => PathBuf::from(home).join(".jekko").join("jekko.db"),
        None => PathBuf::from("jekko.db"),
    }
}

fn resolve_runner_bin() -> Result<PathBuf> {
    if let Some(path) = std::env::var_os("JANKURAI_RUNNER_BIN") {
        return Ok(path.into());
    }
    let current = std::env::current_exe().context("resolve current executable")?;
    if let Some(parent) = current.parent() {
        let sibling = parent.join("jankurai-runner");
        if sibling.exists() {
            return Ok(sibling);
        }
    }
    Ok(PathBuf::from("jankurai-runner"))
}

fn print_tail(path: &std::path::Path, limit: usize) -> Result<usize> {
    let mut file = match OpenOptions::new().read(true).open(path) {
        Ok(file) => file,
        Err(_) => return Ok(0),
    };
    let _ = file.seek(SeekFrom::Start(0));
    let text = fs::read_to_string(path).unwrap_or_default();
    let lines: Vec<&str> = text.lines().collect();
    let start = lines.len().saturating_sub(limit);
    for line in &lines[start..] {
        println!("{line}");
    }
    Ok(lines.len())
}

fn last_line(path: &std::path::Path) -> Result<Option<String>> {
    let text = fs::read_to_string(path).unwrap_or_default();
    Ok(text.lines().last().map(str::to_string))
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn random_run_id() -> String {
    format!("port-{}", now_secs())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DaemonMetadata {
    pid: u32,
    kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    run_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    repo: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    port_config: Option<PathBuf>,
    started_at: u64,
}

impl DaemonMetadata {
    fn event_log_path(&self) -> Option<PathBuf> {
        Some(
            self.repo
                .as_ref()?
                .join("target/zyal/runs")
                .join(self.run_id.as_ref()?)
                .join("events.jsonl"),
        )
    }
}

#[allow(dead_code)]
fn _path_ref(_: &Path) {}
