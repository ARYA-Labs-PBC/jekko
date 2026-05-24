use std::path::PathBuf;

use anyhow::{Context, Result};

use jankurai_runner::bootstrap_check;
use jankurai_runner::daemon_store;
use jankurai_runner::events::{EventKind, EventSink};
use jankurai_runner::hero_judge_runner::{read_hero_judge_runbook, run_hero_judge_run};
use jankurai_runner::model_client::{
    BudgetedModelClient, FakeModelClient, JekkoRuntimeModelClient, ModelClient,
};
use jankurai_runner::model_policy::ModelTaskKind;
use jankurai_runner::port_runner::{read_port_run_config, run_port_tick};
use jankurai_runner::runner::{self, run_once, RunnerConfig};

use super::cli::{Cli, HeroJudgeRunArgs, ModelSmokeArgs, PortRunArgs, RunnerCommand};
use super::hero_series::{hero_judge_client, run_hero_judge_series};

pub(crate) async fn dispatch(cli: Cli) -> Result<i32> {
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

fn stop_requested(path: Option<&PathBuf>) -> bool {
    path.is_some_and(|path| path.exists())
}
