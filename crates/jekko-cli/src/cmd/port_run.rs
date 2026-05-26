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
//! The per-phase body is intentionally a STUB for now (it marks the phase
//! `Running` then immediately `Complete`). The real per-phase invocation
//! lands when `jankurai-runner` is wired in as a follow-up; replacing the
//! inner `mark_complete` block with the real worker spawn is the next big
//! integration step.
//!
//! Modes:
//! - `--super <PATH>` -> compile + persist + walk waves.
//! - `--dry-run`      -> print the wave plan as JSON without persisting.
//! - `--resume <ID>`  -> reopen a run, reset in-flight `Running` phases to
//!                       `Pending`, then walk remaining waves.
//! - `--status <ID>`  -> print persisted phase + task rows as JSON; no state
//!                       changes.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command as ProcCommand;

use anyhow::{anyhow, bail, Context, Result};
use clap::Args;
use serde::Serialize;
use serde_json::Value as JsonValue;
use tempfile::NamedTempFile;
use zyal_supervisor::{
    execution_layers, validate_manifest, Phase, PhaseStatus, SuperWorkflow, SupervisorStore,
};

use crate::cli::GlobalOpts;

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

    /// Informational cap on stages; passed through to the plan JSON for
    /// downstream tools. The scaffold does not enforce it.
    #[arg(long = "max-stages", value_name = "N")]
    pub max_stages: Option<u32>,

    /// Informational time budget in hours; recorded so observers can
    /// reason about run scope. The scaffold does not enforce it.
    #[arg(long = "time-budget-hours", value_name = "H")]
    pub time_budget_hours: Option<f64>,
}

/// Entry point invoked from `main.rs`.
pub fn run(_global: &GlobalOpts, args: &PortRunArgs) -> Result<()> {
    validate_arg_combination(args)?;

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
    walk_waves(&store, &manifest, &run_id)
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

/// Resolve the supervisor DB path: `--db` if set, else
/// `~/.jekko/zyal-supervisor.sqlite`.
fn resolve_db_path(args: &PortRunArgs) -> Result<PathBuf> {
    if let Some(p) = args.db.as_ref() {
        return Ok(p.clone());
    }
    let home = std::env::var_os("HOME")
        .ok_or_else(|| anyhow!("HOME is not set; pass --db <PATH> explicitly"))?;
    let mut path = PathBuf::from(home);
    path.push(".jekko");
    path.push("zyal-supervisor.sqlite");
    Ok(path)
}

fn open_store(args: &PortRunArgs, in_memory: bool) -> Result<SupervisorStore> {
    if in_memory {
        return SupervisorStore::open_in_memory().context("open in-memory supervisor store");
    }
    let path = resolve_db_path(args)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("mkdir -p {}", parent.display()))?;
    }
    SupervisorStore::open(&path).with_context(|| format!("open supervisor store at {}", path.display()))
}

/// Load a SuperWorkflow manifest from `path`. `.zyal` files are compiled
/// via the `zyalc` binary into a tempfile; `.json` files are read directly.
fn load_manifest(path: &Path) -> Result<SuperWorkflow> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let json_text = match ext.as_str() {
        "json" => std::fs::read_to_string(path)
            .with_context(|| format!("read manifest {}", path.display()))?,
        "zyal" => compile_zyal_to_json(path)?,
        other => bail!(
            "unsupported manifest extension `.{other}`; expected `.zyal` or `.json` (at {})",
            path.display()
        ),
    };
    parse_supervisor_manifest(&json_text, path)
}

/// Shell out to `cargo run -p zyalc --offline -- compile <path> --out <tempfile>`
/// and return the JSON text. Tempfile is created with `tempfile::NamedTempFile`
/// so it cleans up after a successful read.
fn compile_zyal_to_json(path: &Path) -> Result<String> {
    let tmp = NamedTempFile::new().context("allocate tempfile for zyalc output")?;
    let tmp_path = tmp.path().to_path_buf();
    let status = ProcCommand::new("cargo")
        .args([
            "run",
            "-p",
            "zyalc",
            "--offline",
            "--quiet",
            "--",
            "compile",
        ])
        .arg(path)
        .arg("--out")
        .arg(&tmp_path)
        .status()
        .with_context(|| format!("invoke zyalc to compile {}", path.display()))?;
    if !status.success() {
        bail!(
            "zyalc failed to compile {} (exit code {:?})",
            path.display(),
            status.code()
        );
    }
    let text = std::fs::read_to_string(&tmp_path)
        .with_context(|| format!("read compiled manifest at {}", tmp_path.display()))?;
    Ok(text)
}

/// Parse the JSON text produced by zyalc into the supervisor's
/// `SuperWorkflow` shape. The two formats differ slightly — zyalc emits a
/// nested `{id, job:{name,objective,...}, superworkflow:{phases:[...]}}`
/// document, while the supervisor model is flat. We adapt here so callers
/// can hand any of the three accepted shapes:
///
/// 1. The flat supervisor shape (used directly by tests + simple manifests).
/// 2. The nested zyalc emission shape (`agent/superworkflows/*.json`).
/// 3. A bare `{phases: [...]}` document carrying just the DAG.
fn parse_supervisor_manifest(text: &str, source: &Path) -> Result<SuperWorkflow> {
    let value: JsonValue = serde_json::from_str(text)
        .with_context(|| format!("parse manifest JSON at {}", source.display()))?;
    // Prefer the flat shape when both `phases` and `objective` are present at
    // the top level (the supervisor's canonical encoding).
    if value.get("phases").is_some() && value.get("objective").is_some() {
        return serde_json::from_value::<SuperWorkflow>(value)
            .with_context(|| format!("decode flat SuperWorkflow at {}", source.display()));
    }
    // Otherwise treat the document as the zyalc-nested emission shape.
    adapt_zyalc_emission(&value, source)
}

/// Translate the `agent/superworkflows/*.json` shape into a flat
/// `SuperWorkflow`. Best-effort: anything outside the supervisor's core
/// fields is dropped silently for the scaffold; richer policies are a
/// follow-up.
fn adapt_zyalc_emission(value: &JsonValue, source: &Path) -> Result<SuperWorkflow> {
    let id = value
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("manifest at {} is missing `id`", source.display()))?
        .to_string();
    let job = value.get("job");
    let name = job
        .and_then(|j| j.get("name"))
        .and_then(|v| v.as_str())
        .unwrap_or(id.as_str())
        .to_string();
    let objective = job
        .and_then(|j| j.get("objective"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let phases_node = value
        .get("superworkflow")
        .and_then(|sw| sw.get("phases"))
        .or_else(|| value.get("phases"))
        .ok_or_else(|| {
            anyhow!(
                "manifest at {} is missing `superworkflow.phases` (or top-level `phases`)",
                source.display()
            )
        })?;
    let raw_phases: Vec<JsonValue> = phases_node
        .as_array()
        .ok_or_else(|| anyhow!("`phases` at {} must be a JSON array", source.display()))?
        .clone();
    let mut phases: Vec<Phase> = Vec::with_capacity(raw_phases.len());
    for (idx, raw) in raw_phases.into_iter().enumerate() {
        let phase_id = raw
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                anyhow!(
                    "phase #{idx} at {} is missing `id`",
                    source.display()
                )
            })?
            .to_string();
        let phase_name = raw
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or(phase_id.as_str())
            .to_string();
        let phase_objective = raw
            .get("objective")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let depends_on: Vec<String> = raw
            .get("depends_on")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|s| s.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        phases.push(Phase {
            id: phase_id,
            name: phase_name,
            objective: phase_objective,
            depends_on,
            write_scope: Default::default(),
            signoff: Default::default(),
            gates: Vec::new(),
        });
    }
    Ok(SuperWorkflow {
        id,
        name,
        objective,
        phases,
        controller: Default::default(),
        memory: Default::default(),
        sandbox: Default::default(),
        repo_graph: Default::default(),
        parity: Default::default(),
    })
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
            let depends_on: Vec<String> =
                serde_json::from_str(&depends_json).unwrap_or_default();
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

    walk_waves(&store, &manifest, run_id)
}

/// Walk the manifest's execution layers serially. Within each layer, mark
/// every phase `Running` then immediately `Complete`.
///
/// STUB: the inner `record_phase_status(..., Complete, ...)` call is where
/// a real `jankurai-runner` invocation belongs. The scaffold persists the
/// transition so dashboards + `--status` look real, but no model work
/// happens here. Replacing this body is the next follow-up.
fn walk_waves(store: &SupervisorStore, manifest: &SuperWorkflow, run_id: &str) -> Result<()> {
    let waves = execution_layers(manifest)
        .map_err(|err| anyhow!("plan execution layers failed: {err}"))?;
    let completed_already: BTreeSet<String> = store
        .completed_phase_ids(run_id)
        .context("load completed phase ids")?
        .into_iter()
        .collect();
    let total_waves = waves.len();
    for (i, wave) in waves.into_iter().enumerate() {
        let mut newly_completed = 0usize;
        for phase_id in &wave {
            if completed_already.contains(phase_id) {
                continue;
            }
            store
                .record_phase_status(run_id, phase_id, PhaseStatus::Running, "")
                .with_context(|| format!("mark phase `{phase_id}` running"))?;
            // STUB: real per-phase work (jankurai-runner role lanes,
            // worktree spawn, reducer fence, sign-off gate) lands when the
            // next follow-up wires `super_agent_runner` in. For Phase H
            // scaffold we immediately mark complete so the DB schema +
            // wave traversal are exercised end-to-end.
            store
                .record_phase_status(
                    run_id,
                    phase_id,
                    PhaseStatus::Complete,
                    "stub: real per-phase work lands when jankurai-runner is wired in",
                )
                .with_context(|| format!("mark phase `{phase_id}` complete"))?;
            newly_completed += 1;
        }
        println!(
            "wave {}/{} complete, {} phases marked complete",
            i + 1,
            total_waves,
            newly_completed
        );
    }
    println!("run `{run_id}` complete (stub bodies)");
    Ok(())
}

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
