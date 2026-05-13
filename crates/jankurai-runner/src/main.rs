use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;

use jankurai_runner::bootstrap_check;
use jankurai_runner::runner::{RunnerConfig, run_once};

#[derive(Parser, Debug)]
#[command(
    name = "jankurai-runner",
    version,
    about = "Forever-runner that drains jankurai findings to zero across worktree workers."
)]
struct Cli {
    /// Repo root. Defaults to the current working directory.
    #[arg(long, default_value = ".")]
    repo: PathBuf,

    /// Unique run id. Used for branch / worktree / receipt namespacing. Random if omitted.
    #[arg(long, env = "JANKURAI_RUN_ID")]
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
}

fn main() {
    let cli = Cli::parse();
    let code = match dispatch(cli) {
        Ok(c) => c,
        Err(err) => {
            eprintln!("jankurai-runner: {err:#}");
            1
        }
    };
    std::process::exit(code);
}

fn dispatch(cli: Cli) -> Result<i32> {
    let repo = cli.repo.canonicalize().with_context(|| format!("canonicalize repo: {}", cli.repo.display()))?;

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

    let run_id = match cli.run_id {
        Some(id) => id,
        None => runner::random_run_id(),
    };
    let config = RunnerConfig {
        repo,
        run_id,
        pool_size: cli.pool_size,
        integration_branch: cli.integration_branch,
        allow_dirty: cli.allow_dirty,
        dry_run: cli.dry_run,
    };

    let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build()?;
    runtime.block_on(async move {
        if cli.once {
            run_once(&config).await
        } else {
            runner::run_forever(&config).await
        }
    })
}

use jankurai_runner::runner;
