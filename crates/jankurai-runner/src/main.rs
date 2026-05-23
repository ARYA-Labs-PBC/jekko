use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand};
use serde::de::DeserializeOwned;

use jankurai_runner::bootstrap_check;
use jankurai_runner::daemon_store;
use jankurai_runner::events::{EventKind, EventSink};
use jankurai_runner::hashing::sha256_hex;
use jankurai_runner::hero_judge::{
    HeroJudgeLaneMetric, HeroJudgeQualityMetric, HeroJudgeRunSummary, HeroJudgeSeriesRow,
    HeroJudgeSeriesSummary,
};
use jankurai_runner::hero_judge_eval::{
    write_jsonl, write_lane_metrics_csv, write_quality_csv, write_series_summary_csv,
};
use jankurai_runner::hero_judge_runner::{read_hero_judge_runbook, run_hero_judge_run};
use jankurai_runner::model_client::{
    BudgetedModelClient, FakeModelClient, JekkoRuntimeModelClient, ModelClient,
};
use jankurai_runner::model_policy::ModelTaskKind;
use jankurai_runner::port_runner::{read_port_run_config, run_port_tick};
use jankurai_runner::runner::{run_once, RunnerConfig};

#[derive(Parser, Debug)]
#[command(
    name = "jankurai-runner",
    version,
    about = "Forever-runner that drains jankurai findings to zero across worktree workers."
)]
struct Cli {
    /// Repo root. Defaults to the current working directory.
    #[arg(long, default_value = ".", global = true)]
    repo: PathBuf,

    /// Unique run id. Used for branch / worktree / receipt namespacing. Random if omitted.
    #[arg(long, env = "JANKURAI_RUN_ID", global = true)]
    run_id: Option<String>,

    /// Worker pool size. Resolved to min(this, 20, jnoccio.spawn_batch_limit) at runtime.
    #[arg(long, default_value_t = 5)]
    pool_size: usize,

    /// Integration branch that worker branches rebase onto. Defaults to `zyal/<run_id>/integration`.
    #[arg(long)]
    integration_branch: Option<String>,

    /// Allow starting against a dirty working tree (will stash with audit trail).
    #[arg(long)]
    allow_dirty: bool,

    /// Do not invoke jankurai audit / git mutations. Useful in CI smoke tests.
    #[arg(long)]
    dry_run: bool,

    /// Run a single tick then exit (instead of looping forever).
    #[arg(long)]
    once: bool,

    /// Focused runner command. Omitted means the legacy jankurai tick loop.
    #[command(subcommand)]
    command: Option<RunnerCommand>,
}

#[derive(Subcommand, Debug)]
enum RunnerCommand {
    /// Exercise the model client and persist a model outcome receipt.
    ModelSmoke(ModelSmokeArgs),
    /// Run one durable generic port workflow tick.
    PortRun(PortRunArgs),
    /// Run one ZYAL Hero/Judge prompt-evolution workflow.
    HeroJudgeRun(HeroJudgeRunArgs),
}

#[derive(Args, Debug)]
struct ModelSmokeArgs {
    /// Prompt to send to the model client.
    #[arg(long)]
    prompt: String,
    /// Use the live Jekko runtime instead of the fake deterministic client.
    #[arg(long)]
    live: bool,
    /// Provider override for live calls.
    #[arg(long)]
    provider: Option<String>,
    /// Model override for live calls.
    #[arg(long)]
    model: Option<String>,
}

#[derive(Args, Debug)]
struct PortRunArgs {
    /// JSON or TOML port workflow config.
    #[arg(long)]
    config: PathBuf,
    /// Use the live Jekko runtime for planning.
    #[arg(long)]
    live: bool,
    /// Provider override for live calls.
    #[arg(long)]
    provider: Option<String>,
    /// Model override for live calls.
    #[arg(long)]
    model: Option<String>,
    /// Maximum ticks to run.
    #[arg(long)]
    max_ticks: Option<u64>,
    /// Seconds between ticks when running multiple ticks.
    #[arg(long, default_value_t = 30)]
    tick_interval_secs: u64,
    /// Stop when this file exists.
    #[arg(long)]
    stop_file: Option<PathBuf>,
    /// Run until stopped. Default for this binary remains one tick.
    #[arg(long)]
    forever: bool,
}

#[derive(Args, Debug, Clone)]
struct HeroJudgeRunArgs {
    /// ZYAL runbook path.
    #[arg(long)]
    zyal: PathBuf,
    /// Use live Jekko runtime model calls.
    #[arg(long)]
    live: bool,
    /// Provider override for live calls.
    #[arg(long)]
    provider: Option<String>,
    /// Model override for live calls.
    #[arg(long)]
    model: Option<String>,
    /// Override maximum generations for smoke/proof runs.
    #[arg(long)]
    max_generations: Option<usize>,
    /// Number of sequential trials to run for plot-ready series data.
    #[arg(long, default_value_t = 1)]
    runs: usize,
}

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    let cli = Cli::parse();
    let code = match dispatch(cli).await {
        Ok(c) => c,
        Err(err) => {
            eprintln!("jankurai-runner: {err:#}");
            1
        }
    };
    std::process::exit(code);
}

async fn dispatch(cli: Cli) -> Result<i32> {
    let repo = cli
        .repo
        .canonicalize()
        .with_context(|| format!("canonicalize repo: {}", cli.repo.display()))?;
    let run_id = match cli.run_id {
        Some(id) => id,
        None => runner::random_run_id(),
    };

    if let Some(command) = cli.command {
        return match command {
            RunnerCommand::ModelSmoke(args) => run_model_smoke(repo, run_id, args).await,
            RunnerCommand::PortRun(args) => run_port_command(repo, run_id, args).await,
            RunnerCommand::HeroJudgeRun(args) => run_hero_judge_command(repo, run_id, args).await,
        };
    }

    // Bootstrap precondition mirrors the TS detect.ts check from PR1.
    let readiness = bootstrap_check::is_ready(&repo);
    if !readiness.ok {
        eprintln!(
            "jankurai-runner: repo not bootstrap-ready ({} required canonical file{} missing). Run `jekko jankurai bootstrap --yes` first.",
            readiness.missing_required.len(),
            if readiness.missing_required.len() == 1 { "" } else { "s" },
        );
        for path in &readiness.missing_required {
            eprintln!("  - {path}");
        }
        return Ok(64);
    }

    let config = RunnerConfig {
        repo,
        run_id,
        pool_size: cli.pool_size,
        integration_branch: cli.integration_branch,
        allow_dirty: cli.allow_dirty,
        dry_run: cli.dry_run,
    };

    if cli.once {
        run_once(&config).await
    } else {
        runner::run_forever(&config).await
    }
}

async fn run_model_smoke(repo: PathBuf, run_id: String, args: ModelSmokeArgs) -> Result<i32> {
    let client: Box<dyn ModelClient> = if args.live {
        Box::new(JekkoRuntimeModelClient::new(args.provider, args.model))
    } else {
        Box::new(FakeModelClient::success("fake model smoke"))
    };
    let receipt = client
        .complete(ModelTaskKind::PhaseFinalize, &args.prompt, &repo)
        .await?;

    let db = daemon_store::open_db(&repo)?;
    daemon_store::ensure_daemon_run(
        &db,
        &repo,
        &run_id,
        serde_json::json!({"kind": "model_smoke", "prompt_len": args.prompt.len()}),
    )?;
    daemon_store::persist_model_receipt(&db, &run_id, &receipt)?;
    let sink = EventSink::open(&repo, &run_id)?;
    sink.emit(
        EventKind::ModelOutcome,
        serde_json::json!({
            "kind": receipt.kind,
            "provider": receipt.provider,
            "model": receipt.model,
            "success": receipt.success,
        }),
    )?;
    println!("{}", serde_json::to_string_pretty(&receipt)?);
    if receipt.success {
        Ok(0)
    } else {
        Ok(1)
    }
}

async fn run_port_command(repo: PathBuf, run_id: String, args: PortRunArgs) -> Result<i32> {
    let config = read_port_run_config(&args.config)?;
    if config.runtime.live_call_budget.require_live && !args.live {
        anyhow::bail!("port config requires live model calls; pass --live");
    }
    let client: Box<dyn ModelClient> = if args.live {
        let live = JekkoRuntimeModelClient::with_policy(
            args.provider,
            args.model,
            config.runtime.model_policy.clone(),
        );
        Box::new(BudgetedModelClient::new(
            live,
            config.runtime.live_call_budget.max_calls,
            config.runtime.live_call_budget.max_parallel,
            true,
        ))
    } else {
        Box::new(FakeModelClient::success("deterministic port plan"))
    };
    let max_ticks = args
        .max_ticks
        .unwrap_or(if args.forever { u64::MAX } else { 1 });
    let mut tick = 0_u64;
    loop {
        if stop_requested(args.stop_file.as_ref()) {
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "run_id": run_id,
                    "stopped": true,
                    "reason": "stop_file",
                    "ticks": tick,
                }))?
            );
            break;
        }
        let report = run_port_tick(&repo, &run_id, config.clone(), client.as_ref()).await?;
        println!("{}", serde_json::to_string_pretty(&report)?);
        tick += 1;
        if tick >= max_ticks {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_secs(args.tick_interval_secs)).await;
    }
    Ok(0)
}

async fn run_hero_judge_command(
    repo: PathBuf,
    run_id: String,
    args: HeroJudgeRunArgs,
) -> Result<i32> {
    let runbook = read_hero_judge_runbook(&args.zyal)?;
    if args.runs > 500 {
        anyhow::bail!("hero-judge-run --runs is capped at 500");
    }
    if args.runs > 1 {
        let report = run_hero_judge_series(&repo, &run_id, &args, runbook).await?;
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(0);
    }
    let client = hero_judge_client(&args, &runbook);
    let report = run_hero_judge_run(
        &repo,
        &run_id,
        &args.zyal,
        runbook,
        args.max_generations,
        args.live,
        client.as_ref(),
    )
    .await?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(0)
}

fn hero_judge_client(
    args: &HeroJudgeRunArgs,
    runbook: &jankurai_runner::hero_judge::HeroJudgeRunbook,
) -> Box<dyn ModelClient> {
    if args.live {
        let provider = args
            .provider
            .clone()
            .or_else(|| Some("jnoccio".to_string()));
        let model = args
            .model
            .clone()
            .or_else(|| Some("jnoccio-fusion".to_string()));
        let live = JekkoRuntimeModelClient::new(provider, model);
        Box::new(BudgetedModelClient::new(
            live,
            runbook.hero_judge.budgets.model_calls,
            runbook.hero_judge.population.max_parallel,
            true,
        ))
    } else {
        Box::new(FakeModelClient::success("deterministic hero judge"))
    }
}

async fn run_hero_judge_series(
    repo: &Path,
    series_id: &str,
    args: &HeroJudgeRunArgs,
    runbook: jankurai_runner::hero_judge::HeroJudgeRunbook,
) -> Result<HeroJudgeSeriesSummary> {
    let series_dir = repo
        .join(runbook.hero_judge.output_root())
        .join(format!("{series_id}-series"));
    fs::create_dir_all(&series_dir).with_context(|| format!("mkdir {}", series_dir.display()))?;
    let runs = run_series_trials(repo, series_id, args, runbook.clone()).await?;

    let mut quality_metrics = Vec::new();
    let mut lane_metrics = Vec::new();
    for summary in &runs {
        quality_metrics.extend(read_jsonl::<HeroJudgeQualityMetric>(
            &summary.quality_metrics_jsonl,
        )?);
        lane_metrics.extend(read_jsonl::<HeroJudgeLaneMetric>(
            &summary.lane_metrics_jsonl,
        )?);
    }

    let run_summaries_jsonl = series_dir.join("run_summaries.jsonl");
    let quality_metrics_jsonl = series_dir.join("quality_metrics.jsonl");
    let quality_metrics_csv = series_dir.join("quality_metrics.csv");
    let lane_metrics_jsonl = series_dir.join("lane_metrics.jsonl");
    let lane_metrics_csv = series_dir.join("lane_metrics.csv");
    let hero_metrics_csv = series_dir.join("hero_metrics.csv");
    let judge_metrics_csv = series_dir.join("judge_metrics.csv");
    let series_summary_csv = series_dir.join("series_summary.csv");
    let reviewer_index_json = series_dir.join("reviewer_index.json");
    let complete_ok = series_dir.join("complete.ok");
    let series_rows = series_rows(series_id, &runs, &quality_metrics, &lane_metrics)?;

    write_jsonl(&run_summaries_jsonl, &runs)?;
    write_jsonl(&quality_metrics_jsonl, &quality_metrics)?;
    write_quality_csv(&quality_metrics_csv, &quality_metrics)?;
    write_jsonl(&lane_metrics_jsonl, &lane_metrics)?;
    write_lane_metrics_csv(&lane_metrics_csv, &lane_metrics)?;
    write_lane_metrics_csv(
        &hero_metrics_csv,
        &filter_series_lanes(&lane_metrics, "hero"),
    )?;
    write_lane_metrics_csv(
        &judge_metrics_csv,
        &filter_series_lanes(&lane_metrics, "judge"),
    )?;
    write_series_summary_csv(&series_summary_csv, &series_rows)?;
    fs::write(
        &reviewer_index_json,
        serde_json::to_string_pretty(&serde_json::json!({
            "series_id": series_id,
            "run_count": runs.len(),
            "reviewer_packet_paths": runs
                .iter()
                .map(|run| run.reviewer_packet_json.display().to_string())
                .collect::<Vec<_>>(),
            "plot_files": {
                "quality_metrics_csv": quality_metrics_csv.display().to_string(),
                "lane_metrics_csv": lane_metrics_csv.display().to_string(),
                "hero_metrics_csv": hero_metrics_csv.display().to_string(),
                "judge_metrics_csv": judge_metrics_csv.display().to_string(),
                "series_summary_csv": series_summary_csv.display().to_string(),
            },
        }))?,
    )
    .with_context(|| format!("write {}", reviewer_index_json.display()))?;
    fs::write(&complete_ok, b"ok\n").with_context(|| format!("write {}", complete_ok.display()))?;

    Ok(HeroJudgeSeriesSummary {
        series_id: series_id.to_string(),
        output_dir: series_dir,
        run_count: runs.len(),
        runs,
        run_summaries_jsonl,
        quality_metrics_jsonl,
        quality_metrics_csv,
        lane_metrics_jsonl,
        lane_metrics_csv,
        hero_metrics_csv,
        judge_metrics_csv,
        series_summary_csv,
        reviewer_index_json,
        complete_ok,
    })
}

async fn run_series_trials(
    repo: &Path,
    series_id: &str,
    args: &HeroJudgeRunArgs,
    runbook: jankurai_runner::hero_judge::HeroJudgeRunbook,
) -> Result<Vec<HeroJudgeRunSummary>> {
    let parallelism = series_parallelism(args.runs);
    if parallelism == 1 {
        let mut runs = Vec::new();
        for trial in 1..=args.runs {
            let child_run_id = format!("{series_id}-trial-{trial:03}");
            let client = hero_judge_client(args, &runbook);
            runs.push(
                run_hero_judge_run(
                    repo,
                    &child_run_id,
                    &args.zyal,
                    runbook.clone(),
                    args.max_generations,
                    args.live,
                    client.as_ref(),
                )
                .await?,
            );
        }
        return Ok(runs);
    }

    let mut next_trial = 1;
    let mut completed = Vec::with_capacity(args.runs);
    let mut children: Vec<SeriesChild> = Vec::new();
    while next_trial <= args.runs || !children.is_empty() {
        while next_trial <= args.runs && children.len() < parallelism {
            let trial = next_trial;
            let child_run_id = format!("{series_id}-trial-{trial:03}");
            children.push(spawn_series_child(repo, &child_run_id, trial, args)?);
            next_trial += 1;
        }
        let mut finished = None;
        for (idx, child) in children.iter_mut().enumerate() {
            if child
                .child
                .try_wait()
                .with_context(|| format!("poll trial {}", child.trial))?
                .is_some()
            {
                finished = Some(idx);
                break;
            }
        }
        if let Some(idx) = finished {
            let child = children.swap_remove(idx);
            completed.push(read_series_child(child)?);
        } else {
            thread::sleep(Duration::from_millis(250));
        }
    }
    completed.sort_by_key(|(trial, _)| *trial);
    Ok(completed.into_iter().map(|(_, summary)| summary).collect())
}

struct SeriesChild {
    trial: usize,
    child: Child,
}

fn spawn_series_child(
    repo: &Path,
    child_run_id: &str,
    trial: usize,
    args: &HeroJudgeRunArgs,
) -> Result<SeriesChild> {
    let mut command = Command::new(std::env::current_exe().context("resolve current exe")?);
    command
        .arg("--repo")
        .arg(repo)
        .arg("--run-id")
        .arg(child_run_id)
        .arg("hero-judge-run")
        .arg("--zyal")
        .arg(&args.zyal);
    if args.live {
        command.arg("--live");
    }
    if let Some(provider) = args.provider.as_deref() {
        command.arg("--provider").arg(provider);
    }
    if let Some(model) = args.model.as_deref() {
        command.arg("--model").arg(model);
    }
    if let Some(max_generations) = args.max_generations {
        command
            .arg("--max-generations")
            .arg(max_generations.to_string());
    }
    let child = command
        .envs(series_child_env(trial))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("spawn trial {trial}"))?;
    Ok(SeriesChild { trial, child })
}

fn series_child_env(trial: usize) -> Vec<(&'static str, PathBuf)> {
    let Some(db) = std::env::var_os("JEKKO_DB").map(PathBuf::from) else {
        return Vec::new();
    };
    let child_db = match db.extension().and_then(|extension| extension.to_str()) {
        Some(extension) => db.with_extension(format!("trial-{trial:03}.{extension}")),
        None => db.with_file_name(format!("{}.trial-{trial:03}", db.display())),
    };
    vec![("JEKKO_DB", child_db)]
}

fn read_series_child(child: SeriesChild) -> Result<(usize, HeroJudgeRunSummary)> {
    let output = child
        .child
        .wait_with_output()
        .with_context(|| format!("wait trial {}", child.trial))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("trial {} failed: {}", child.trial, stderr.trim());
    }
    let summary: HeroJudgeRunSummary = serde_json::from_slice(&output.stdout)
        .with_context(|| format!("decode trial {} summary", child.trial))?;
    Ok((child.trial, summary))
}

fn series_parallelism(run_count: usize) -> usize {
    std::env::var("HERO_JUDGE_SERIES_PARALLEL")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(1)
        .min(run_count.max(1))
        .min(12)
}

fn series_rows(
    series_id: &str,
    runs: &[HeroJudgeRunSummary],
    quality_metrics: &[HeroJudgeQualityMetric],
    lane_metrics: &[HeroJudgeLaneMetric],
) -> Result<Vec<HeroJudgeSeriesRow>> {
    runs.iter()
        .enumerate()
        .map(|(index, run)| {
            let final_metric = quality_metrics
                .iter()
                .filter(|metric| metric.run_id == run.run_id)
                .max_by_key(|metric| metric.generation)
                .with_context(|| format!("missing quality metrics for {}", run.run_id))?;
            Ok(HeroJudgeSeriesRow {
                series_id: series_id.to_string(),
                trial_index: index + 1,
                run_id: run.run_id.clone(),
                generation: final_metric.generation,
                theory_quality_index: final_metric.theory_quality_index,
                question_quality_index: final_metric.question_quality_index,
                rubric_quality_index: final_metric.rubric_quality_index,
                judge_calibration_index: final_metric.judge_calibration_index,
                evidence_grounding_index: final_metric.evidence_grounding_index,
                verifier_confidence: final_metric.verifier_confidence,
                red_team_resilience: final_metric.red_team_resilience,
                promotion_score: final_metric.promotion_score,
                overall_quality_index: final_metric.overall_quality_index,
                delta_overall_quality: final_metric.delta_overall_quality,
                frontier_quality_index: final_metric.frontier_quality_index,
                delta_frontier_quality: final_metric.delta_frontier_quality,
                promoted: final_metric.promoted,
                frontier_winner: run.frontier_winner.clone(),
                model_calls_used: run.model_calls_used,
                model_call_budget: run.model_call_budget,
                search_receipt_count: run.search_receipt_count,
                hero_lane_mean: rounded(mean_lane_score(
                    &run.run_id,
                    final_metric.generation,
                    lane_metrics,
                    "hero",
                )),
                judge_lane_mean: rounded(mean_lane_score(
                    &run.run_id,
                    final_metric.generation,
                    lane_metrics,
                    "judge",
                )),
                quality_metrics_sha256: file_sha256(&run.quality_metrics_jsonl)?,
                lane_metrics_sha256: file_sha256(&run.lane_metrics_jsonl)?,
                reviewer_packet_sha256: file_sha256(&run.reviewer_packet_json)?,
                promotion_decision_sha256: file_sha256(&run.promotion_decision_json)?,
                search_receipts_sha256: file_sha256(&run.search_receipts_json)?,
            })
        })
        .collect()
}

fn mean_lane_score(
    run_id: &str,
    generation: usize,
    metrics: &[HeroJudgeLaneMetric],
    role_group: &str,
) -> f64 {
    let mut total = 0.0;
    let mut count = 0_usize;
    for metric in metrics {
        if metric.run_id == run_id
            && metric.generation == generation
            && metric.role_group == role_group
        {
            total += metric.score;
            count += 1;
        }
    }
    if count == 0 {
        0.0
    } else {
        total / count as f64
    }
}

fn file_sha256(path: &Path) -> Result<String> {
    let bytes = fs::read(path).with_context(|| format!("read {}", path.display()))?;
    Ok(sha256_hex(&bytes))
}

fn rounded(value: f64) -> f64 {
    (value * 1000.0).round() / 1000.0
}

fn read_jsonl<T: DeserializeOwned>(path: &Path) -> Result<Vec<T>> {
    let text = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    text.lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            serde_json::from_str(line).with_context(|| format!("decode {}", path.display()))
        })
        .collect()
}

fn filter_series_lanes(
    metrics: &[HeroJudgeLaneMetric],
    role_group: &str,
) -> Vec<HeroJudgeLaneMetric> {
    metrics
        .iter()
        .filter(|metric| metric.role_group == role_group)
        .cloned()
        .collect()
}

use jankurai_runner::runner;

fn stop_requested(path: Option<&PathBuf>) -> bool {
    path.is_some_and(|path| path.exists())
}
