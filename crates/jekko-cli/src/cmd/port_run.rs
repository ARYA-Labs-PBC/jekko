//! `jekko port-run --super <manifest>` — Phase H integration glue.
//!
//! Ties together the Phase F1+F2+F4+F5 super-agent kernel pieces so an
//! operator can drive a 12-stage ZYAL SuperWorkflow end-to-end from the CLI:
//!
//!   zyalc compiles a SuperWorkflow `.zyal` manifest -> JSON
//!     -> [`zyal_supervisor::SuperWorkflow`] validates + plans execution waves
//!     -> [`SupervisorStore`] persists per-phase state
//!     -> this command walks the waves, marking phases complete.
//!
//! Two per-phase modes:
//!
//! - **Stub mode (default).** Each phase is marked `Running` then immediately
//!   `Complete` with a synthetic summary. Useful for exercising the schema
//!   and the dependency walk without burning model tokens.
//! - **Live mode (`--live`).** Each phase spawns
//!   `jekko run --ephemeral --json --agent plan --cwd <repo> <prompt>` as a
//!   subprocess via `tokio::process::Command`. The captured stdout becomes
//!   the phase `summary`. Live mode refuses to run unless `JEKKO_ZYAL_LIVE=1`
//!   is set and `CI` is not `true`, so it is opt-in for interactive
//!   operators only.
//!
//! Modes:
//! - `--super <PATH>` -> compile + persist + walk waves.
//! - `--dry-run`      -> print the wave plan as JSON without persisting.
//! - `--resume <ID>`  -> reopen a run, reset in-flight `Running` phases to
//!                       `Pending`, then walk remaining waves.
//! - `--status <ID>`  -> print persisted phase + task rows as JSON; no state
//!                       changes.

use std::path::PathBuf;

use anyhow::{anyhow, bail, Context, Result};
use clap::Args;
use serde::Serialize;
use zyal_supervisor::{execution_layers, validate_manifest, PhaseStatus, SuperWorkflow, SupervisorStore};

use crate::cli::GlobalOpts;

mod parse;
mod walk;

use parse::{load_manifest, open_store};
use walk::walk_waves;
/// `jekko port-run` arguments. The Phase H scaffold focuses on the
/// `--super` path; legacy port-run flags are not surfaced here.
#[derive(Args, Debug, Default)]
pub struct PortRunArgs {
    /// Path to a SuperWorkflow manifest. May be a `.zyal` source (compiled
    /// via `zyalc` on demand) or a pre-compiled `.json` manifest. Required
    /// unless `--resume` or `--status` is set.
    #[arg(long = "super", value_name = "MANIFEST")]
    pub super_manifest: Option<PathBuf>,

    /// Override the supervisor database path. Defaults to
    /// `~/.jekko/zyal-supervisor.sqlite`. `--dry-run` ignores this and uses
    /// an in-memory store.
    #[arg(long, value_name = "PATH")]
    pub db: Option<PathBuf>,

    /// Override the run id. When omitted, the store derives one from the
    /// manifest id + a millisecond timestamp.
    #[arg(long = "run-id", value_name = "ID")]
    pub run_id: Option<String>,

    /// Print the planned execution waves as JSON without persisting any
    /// state. Mutually exclusive with `--resume`.
    #[arg(long)]
    pub dry_run: bool,

    /// Resume an existing run. Reads the manifest back out of the run row,
    /// resets `Running` phases to `Pending`, and walks from the lowest
    /// incomplete wave. Mutually exclusive with the positional manifest.
    #[arg(long, value_name = "RUN_ID")]
    pub resume: Option<String>,

    /// Print persisted phase + task rows for a run as JSON. Exits 0 without
    /// touching state. Mutually exclusive with `--super`/`--resume`.
    #[arg(long, value_name = "RUN_ID")]
    pub status: Option<String>,

    /// Hard cap on stages: stop after `N` phases reach `Complete` and mark
    /// the rest `Blocked` with summary `"stopped at max_stages"`. The cap
    /// is also surfaced in the dry-run plan JSON for downstream tools.
    #[arg(long = "max-stages", value_name = "N")]
    pub max_stages: Option<u32>,

    /// Wall-clock budget in hours: when the cumulative wall time exceeds
    /// this value the orchestrator stops before starting the next wave and
    /// marks remaining phases `Blocked` with summary
    /// `"stopped at time_budget"`. Also surfaced in the dry-run plan JSON.
    #[arg(long = "time-budget-hours", value_name = "H")]
    pub time_budget_hours: Option<f64>,

    /// Live mode: invoke `jekko run --ephemeral --json --agent plan` per
    /// phase via a `tokio::process::Command` subprocess. Refuses to run
    /// unless `JEKKO_ZYAL_LIVE=1` is set and `CI` is not `true`. Default
    /// off (stays in stub mode).
    #[arg(long)]
    pub live: bool,

    /// Per-phase subprocess timeout in seconds for `--live` mode. The
    /// subprocess is killed and the phase is marked `Failed` once the
    /// timeout fires. Defaults to 300 seconds.
    #[arg(long = "per-phase-timeout-secs", value_name = "N", default_value_t = 300)]
    pub per_phase_timeout_secs: u64,
}

/// Entry point invoked from `main.rs`.
pub fn run(_global: &GlobalOpts, args: &PortRunArgs) -> Result<()> {
    validate_arg_combination(args)?;
    // Live-mode gating happens up front so accidental invocations fail fast,
    // before any persistent state is opened. `--status` is purely read-only,
    // so we let it through without forcing operators to set the live env.
    if args.live && args.status.is_none() {
        gate_live_mode()?;
    }

    if let Some(run_id) = args.status.as_deref() {
        return run_status(args, run_id);
    }

    if let Some(run_id) = args.resume.as_deref() {
        return run_resume(args, run_id);
    }

    let manifest_path = args
        .super_manifest
        .as_deref()
        .ok_or_else(|| anyhow!("--super <MANIFEST> is required (or use --resume / --status)"))?;
    let manifest = load_manifest(manifest_path)?;
    validate_manifest(&manifest).map_err(|err| anyhow!("manifest validation failed: {err}"))?;

    if args.dry_run {
        return emit_dry_run_plan(&manifest, args);
    }

    let store = open_store(args, /* in_memory */ false)?;
    let run_id = init_or_use_run_id(&store, &manifest, args.run_id.as_deref())?;
    walk_waves(&store, &manifest, &run_id, args)
}

/// Validate `--live` preconditions. Refuses CI environments and requires the
/// `JEKKO_ZYAL_LIVE=1` opt-in so accidental invocations from automation can
/// not spend tokens. Called only when `args.live` is set.
fn gate_live_mode() -> Result<()> {
    if env_is_truthy("CI") {
        bail!(
            "--live refuses to run when CI=true; unset CI or run interactively to use live mode"
        );
    }
    if !env_is_truthy("JEKKO_ZYAL_LIVE") {
        bail!(
            "--live requires JEKKO_ZYAL_LIVE=1 (opt-in guard against accidental live runs)"
        );
    }
    Ok(())
}

fn env_is_truthy(name: &str) -> bool {
    match std::env::var(name) {
        Ok(v) => {
            let v = v.trim().to_ascii_lowercase();
            matches!(v.as_str(), "1" | "true" | "yes" | "on")
        }
        Err(_) => false,
    }
}

fn validate_arg_combination(args: &PortRunArgs) -> Result<()> {
    let mode_count = [
        args.super_manifest.is_some(),
        args.resume.is_some(),
        args.status.is_some(),
    ]
    .iter()
    .filter(|x| **x)
    .count();
    if mode_count == 0 {
        bail!("provide one of --super <MANIFEST>, --resume <RUN_ID>, or --status <RUN_ID>");
    }
    if mode_count > 1 {
        bail!("--super, --resume, and --status are mutually exclusive");
    }
    if args.dry_run && args.resume.is_some() {
        bail!("--dry-run is mutually exclusive with --resume");
    }
    if args.dry_run && args.status.is_some() {
        bail!("--dry-run is mutually exclusive with --status");
    }
    Ok(())
}


/// Initialize a fresh run row from `manifest`. If `requested` is `Some`, the
/// caller supplied an explicit run id; we still let the store synthesize the
/// derived id and only log the requested value as a tag in `summary`.
/// Honoring an explicit `--run-id` end-to-end requires `store::init_run` to
/// accept an override — a follow-up. The scaffold returns the store's id so
/// the durable schema invariants stay intact.
fn init_or_use_run_id(
    store: &SupervisorStore,
    manifest: &SuperWorkflow,
    requested: Option<&str>,
) -> Result<String> {
    let run_id = store
        .init_run(manifest)
        .context("init supervisor run row")?;
    if let Some(req) = requested {
        if req != run_id {
            eprintln!(
                "jekko port-run: requested run id `{req}` not honored; using store-derived `{run_id}`"
            );
        }
    }
    Ok(run_id)
}

fn emit_dry_run_plan(manifest: &SuperWorkflow, args: &PortRunArgs) -> Result<()> {
    let waves = execution_layers(manifest)
        .map_err(|err| anyhow!("plan execution layers failed: {err}"))?;
    let synthetic_run_id = args
        .run_id
        .clone()
        .unwrap_or_else(|| format!("{}-dry-run", manifest.id));
    let plan = DryRunPlan {
        run_id: synthetic_run_id,
        manifest_id: manifest.id.clone(),
        manifest_name: manifest.name.clone(),
        waves,
        max_stages: args.max_stages,
        time_budget_hours: args.time_budget_hours,
    };
    println!("{}", serde_json::to_string_pretty(&plan)?);
    Ok(())
}

#[derive(Serialize)]
struct DryRunPlan {
    run_id: String,
    manifest_id: String,
    manifest_name: String,
    waves: Vec<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_stages: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    time_budget_hours: Option<f64>,
}

fn run_status(args: &PortRunArgs, run_id: &str) -> Result<()> {
    let store = open_store(args, false)?;
    let conn = store.connection();

    let mut phase_stmt = conn.prepare(
        "SELECT phase_id, name, objective, depends_on_json, status, summary, \
                started_at, completed_at, updated_at \
         FROM zyal_super_phases WHERE run_id = ?1 ORDER BY phase_id",
    )?;
    let phase_rows = phase_stmt
        .query_map([run_id], |row| {
            let depends_json: String = row.get(3)?;
            let depends_on: Vec<String> = match serde_json::from_str::<Vec<String>>(&depends_json) {
                Ok(parsed) => parsed,
                Err(_) => Vec::new(),
            };
            Ok(PhaseStatusRow {
                phase_id: row.get(0)?,
                name: row.get(1)?,
                objective: row.get(2)?,
                depends_on,
                status: row.get(4)?,
                summary: row.get(5)?,
                started_at: row.get(6)?,
                completed_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let mut task_stmt = conn.prepare(
        "SELECT task_id, phase_id, title, status, owner, summary, updated_at \
         FROM zyal_super_tasks WHERE run_id = ?1 ORDER BY phase_id, task_id",
    )?;
    let task_rows = task_stmt
        .query_map([run_id], |row| {
            Ok(TaskStatusRow {
                task_id: row.get(0)?,
                phase_id: row.get(1)?,
                title: row.get(2)?,
                status: row.get(3)?,
                owner: row.get(4)?,
                summary: row.get(5)?,
                updated_at: row.get(6)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let report = StatusReport {
        run_id: run_id.to_string(),
        phases: phase_rows,
        tasks: task_rows,
    };
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

#[derive(Serialize)]
struct StatusReport {
    run_id: String,
    phases: Vec<PhaseStatusRow>,
    tasks: Vec<TaskStatusRow>,
}

#[derive(Serialize)]
struct PhaseStatusRow {
    phase_id: String,
    name: String,
    objective: String,
    depends_on: Vec<String>,
    status: String,
    summary: String,
    started_at: Option<String>,
    completed_at: Option<String>,
    updated_at: String,
}

#[derive(Serialize)]
struct TaskStatusRow {
    task_id: String,
    phase_id: String,
    title: String,
    status: String,
    owner: Option<String>,
    summary: String,
    updated_at: String,
}

fn run_resume(args: &PortRunArgs, run_id: &str) -> Result<()> {
    let store = open_store(args, false)?;
    let conn = store.connection();
    let manifest_json: String = conn
        .query_row(
            "SELECT manifest_json FROM zyal_super_runs WHERE run_id = ?1",
            [run_id],
            |row| row.get::<_, String>(0),
        )
        .with_context(|| format!("look up run `{run_id}`"))?;
    let manifest: SuperWorkflow = serde_json::from_str(&manifest_json)
        .with_context(|| format!("decode persisted manifest for run `{run_id}`"))?;

    // Demote in-flight phases so they re-enter the ready set on the next pass.
    let now = chrono_now_rfc3339();
    conn.execute(
        "UPDATE zyal_super_phases \
         SET status = ?1, updated_at = ?2 \
         WHERE run_id = ?3 AND status = ?4",
        rusqlite::params![
            PhaseStatus::Pending.as_str(),
            now,
            run_id,
            PhaseStatus::Running.as_str(),
        ],
    )
    .context("reset Running phases to Pending on resume")?;

    walk_waves(&store, &manifest, run_id, args)
}

/// Walk the manifest's execution layers serially. Within each layer, mark
/// every phase `Running` then either:
///
/// - **stub mode** (default): immediately `Complete` with a synthetic
///   summary. Useful for exercising the schema + dependency walk.
/// - **live mode** (`args.live == true`): drive a single
///   `jekko run --ephemeral --json --agent plan` subprocess for the phase
///   and store its stdout as the phase summary.
///
/// `args.max_stages` caps the total number of phases that may complete in
/// this invocation; anything past the cap is recorded `Blocked` with the
/// summary `"stopped at max_stages"`. `args.time_budget_hours` enforces a
/// wall-clock ceiling: when the elapsed time exceeds the budget the
/// remaining phases are recorded `Blocked` with the summary
/// `"stopped at time_budget"`. A `Failed` phase halts advancement; the
/// Local RFC3339 timestamp without pulling chrono into this crate's public
/// surface — uses the supervisor's chrono via a tiny local helper so we
/// keep dep churn out of `jekko-cli`. The supervisor already re-exports
/// nothing public; we just rely on the supervisor's chrono dep being in
/// the tree and use a freestanding format string.
fn chrono_now_rfc3339() -> String {
    // SystemTime -> seconds since epoch -> rough RFC3339-ish UTC string.
    // We intentionally avoid a chrono dep on jekko-cli; the value here is
    // only used as the `updated_at` for the demotion sweep on resume and
    // is replaced by every subsequent `record_phase_status` write.
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // 1970-01-01T00:00:00+00:00 baseline; this is a best-effort label, not
    // a parsed timestamp.
    format!("epoch:{secs}")
}
