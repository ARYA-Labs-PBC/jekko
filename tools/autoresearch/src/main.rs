//! AutoResearch orchestrator.
//!
//! Single-binary skeleton that drives the chase loop:
//!   * `autoresearch seed`     — initialize / verify `autoresearch/chase-best`
//!   * `autoresearch tick`     — run one cycle (N workers, each a hyperparameter
//!                                permutation of cogcore config)
//!   * `autoresearch daemon`   — loop ticks until paused / aborted
//!   * `autoresearch forensics`— bundle last 3 cycles for review
//!
//! Phase 4 ships the T1 (deterministic GA over numeric configs) proposer;
//! T2/T3/T4 templates land later. The reducer is intentionally a thin
//! shell over `cargo run --bin chase_reduce` so the trusted scoring path
//! stays inside `crates/memory-benchmark`.

use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{self, Command};

use memory_benchmark::json::{self, Json};

const STATE_DIR: &str = ".jekko/daemon/memory-benchmark-chase";
const FIXTURE_QBANK_DEV_ONLY: bool = true;
const DEFAULT_WORKER_COUNT: usize = 4;

mod llm;
mod proposer;
mod template;

fn main() {
    let args: Vec<String> = env::args().collect();
    let cmd = args.get(1).map(String::as_str).unwrap_or("help");
    let rest = &args[2.min(args.len())..];
    let result = match cmd {
        "seed" => cmd_seed(rest),
        "tick" => cmd_tick(rest),
        "daemon" => cmd_daemon(rest),
        "forensics" => cmd_forensics(rest),
        _ => {
            print_help();
            Ok(())
        }
    };
    if let Err(err) = result {
        eprintln!("autoresearch: {err}");
        process::exit(2);
    }
}

fn print_help() {
    eprintln!(
        "autoresearch <command>\n\
           seed         — initialize/verify autoresearch/chase-best\n\
           tick         — run one cycle: propose, score, reduce, write receipt\n\
           daemon       — loop tick until paused.flag or aborted.flag exists\n\
           forensics    — bundle last 3 cycles into forensics-bundle.tar\n\
        flags:\n\
           --workers N            (default 4)\n\
           --candidate NAME       (default cogcore)\n\
           --cycle-id ID          (default derived from receipt count)\n\
           --unsafe-allow-skeleton  (required for daemon while reducer/worktrees are incomplete)\n\
           --use-dirty-source-dev-only  (copy dirty benchmark/cogcore sources into worktrees; receipts are dev_only)\n\
           --state-dir PATH       (default .jekko/daemon/memory-benchmark-chase)\n"
    );
}

#[derive(Default)]
struct Flags {
    workers: Option<usize>,
    candidate: Option<String>,
    cycle_id: Option<String>,
    state_dir: Option<String>,
    seed: Option<String>,
    unsafe_allow_skeleton: bool,
    use_dirty_source_dev_only: bool,
}

fn parse_flags(args: &[String]) -> Flags {
    let mut f = Flags::default();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--workers" => {
                f.workers = args.get(i + 1).and_then(|v| v.parse::<usize>().ok());
                i += 2;
            }
            "--candidate" => {
                f.candidate = args.get(i + 1).cloned();
                i += 2;
            }
            "--cycle-id" => {
                f.cycle_id = args.get(i + 1).cloned();
                i += 2;
            }
            "--state-dir" => {
                f.state_dir = args.get(i + 1).cloned();
                i += 2;
            }
            "--seed" => {
                f.seed = args.get(i + 1).cloned();
                i += 2;
            }
            "--unsafe-allow-skeleton" => {
                f.unsafe_allow_skeleton = true;
                i += 1;
            }
            "--use-dirty-source-dev-only" => {
                f.use_dirty_source_dev_only = true;
                i += 1;
            }
            _ => {
                i += 1;
            }
        }
    }
    f
}

fn cmd_seed(args: &[String]) -> Result<(), String> {
    let f = parse_flags(args);
    let state = state_dir(&f);
    fs::create_dir_all(&state).map_err(|e| format!("mkdir {state:?}: {e}"))?;
    fs::create_dir_all(state.join("receipts")).map_err(|e| format!("mkdir receipts: {e}"))?;
    fs::create_dir_all(state.join("reports/lanes"))
        .map_err(|e| format!("mkdir reports/lanes: {e}"))?;
    fs::create_dir_all(state.join("worktrees")).map_err(|e| format!("mkdir worktrees: {e}"))?;
    let best_state = state.join("best-state.json");
    if !best_state.exists() {
        let initial = "{\"name\":\"baseline\",\"total\":0.0,\"cycle_id\":\"0000000\"}\n";
        fs::write(&best_state, initial).map_err(|e| format!("write {best_state:?}: {e}"))?;
        eprintln!("autoresearch: seeded best-state.json at {best_state:?}");
    } else {
        eprintln!("autoresearch: best-state.json already exists at {best_state:?}");
    }
    Ok(())
}

fn cmd_tick(args: &[String]) -> Result<(), String> {
    let f = parse_flags(args);
    let state = state_dir(&f);
    let workers = f.workers.unwrap_or(DEFAULT_WORKER_COUNT);
    let candidate = f.candidate.as_deref().unwrap_or("cogcore");
    fs::create_dir_all(&state).map_err(|e| format!("mkdir {state:?}: {e}"))?;
    fs::create_dir_all(state.join("receipts")).map_err(|e| format!("mkdir receipts: {e}"))?;
    fs::create_dir_all(state.join("reports/lanes"))
        .map_err(|e| format!("mkdir reports/lanes: {e}"))?;
    let pause = state.join("paused.flag");
    if pause.exists() {
        return Err(format!(
            "paused.flag present at {pause:?}; remove it to resume"
        ));
    }
    let abort = state.join("aborted.flag");
    if abort.exists() {
        return Err(format!(
            "aborted.flag present at {abort:?}; investigate before resuming"
        ));
    }
    let cycle_id = f.cycle_id.unwrap_or_else(|| {
        let n = count_receipts(&state.join("receipts"));
        format!("{n:07}")
    });
    let seed = f.seed.unwrap_or_else(|| "public-dev-0001".to_string());

    let repo_root = env::current_dir().map_err(|e| format!("cwd: {e}"))?;
    let proposals = proposer::genetic::propose(workers, &cycle_id, &seed);
    let mut lane_results: Vec<LaneResult> = Vec::new();
    let worktree_root = state.join("worktrees").join(&cycle_id);

    for (idx, prop) in proposals.iter().enumerate() {
        let worker_id = format!("lane_{idx:02}");
        let out_dir = state.join(format!("reports/lanes/{worker_id}"));
        fs::create_dir_all(&out_dir).map_err(|e| format!("mkdir {out_dir:?}: {e}"))?;
        let lane_root = worktree_root.join(&worker_id);
        prepare_worktree(&repo_root, &lane_root, f.use_dirty_source_dev_only)?;
        write_worker_patch(&lane_root, prop)?;

        let out = out_dir.join("northstar.json");
        if let Err(err) = run_northstar(
            &lane_root,
            candidate,
            &prop.seed_label,
            &out,
            &prop.patch_content,
            prop.patch_path,
        ) {
            eprintln!("autoresearch: worker {worker_id} failed ({err})");
            continue;
        }
        let report = fs::read_to_string(&out).map_err(|e| format!("read {out:?}: {e}"))?;
        let total = extract_total(&report)?;
        fs::write(out_dir.join("config.rs"), &prop.patch_content)
            .map_err(|e| format!("write config.rs: {e}"))?;
        lane_results.push(LaneResult {
            worker_id,
            total,
            lane_root,
            patch_content: prop.patch_content.clone(),
            patch_path: prop.patch_path.to_string(),
        });
    }

    if lane_results.is_empty() {
        return Err("no successful worker lanes".to_string());
    }

    lane_results.sort_by(|a, b| {
        b.total
            .partial_cmp(&a.total)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let best_in_cycle = lane_results.first().cloned();
    let shadow_path = state.join("reports/shadow.json");
    if let Some(best_lane) = &best_in_cycle {
        let private_seed = env::var("MEMORY_BENCHMARK_PRIVATE_SEED")
            .unwrap_or_else(|_| "private-default-0001".to_string());
        run_northstar(
            &best_lane.lane_root,
            candidate,
            &private_seed,
            &shadow_path,
            &best_lane.patch_content,
            &best_lane.patch_path,
        )?;
    }

    let run_is_dev_only = f.use_dirty_source_dev_only || FIXTURE_QBANK_DEV_ONLY;
    let reference_reports = run_reference_reports(&repo_root, &state, &cycle_id)?;
    run_reducer(&repo_root, &state, &shadow_path, &reference_reports)?;

    let receipt_path = state.join(format!("receipts/{cycle_id}.json"));
    let receipt = build_receipt(
        &cycle_id,
        workers,
        candidate,
        &lane_results,
        run_is_dev_only,
        &reference_reports,
    );
    fs::write(&receipt_path, receipt).map_err(|e| format!("write receipt: {e}"))?;

    if let Some(best_lane) = best_in_cycle {
        eprintln!(
            "autoresearch: best lane {} scored {:.4} on cycle {}",
            best_lane.worker_id, best_lane.total, cycle_id
        );
    }
    eprintln!("autoresearch: cycle {cycle_id} complete; receipt at {receipt_path:?}");
    Ok(())
}

fn cmd_daemon(args: &[String]) -> Result<(), String> {
    let f = parse_flags(args);
    if !f.unsafe_allow_skeleton {
        return Err(
            "daemon refuses to run without --unsafe-allow-skeleton while the chase loop is still in skeleton mode"
                .to_string(),
        );
    }
    let state = state_dir(&f);
    let max_cycles = 4; // safety bound for skeleton — host scheduler may raise
    for i in 0..max_cycles {
        let pause = state.join("paused.flag");
        let abort = state.join("aborted.flag");
        if pause.exists() {
            eprintln!("autoresearch daemon: paused.flag detected on cycle {i}; halting");
            return Ok(());
        }
        if abort.exists() {
            eprintln!("autoresearch daemon: aborted.flag detected on cycle {i}; halting");
            return Ok(());
        }
        cmd_tick(args)?;
    }
    Ok(())
}

fn cmd_forensics(args: &[String]) -> Result<(), String> {
    let f = parse_flags(args);
    let state = state_dir(&f);
    let bundle = state.join("forensics-bundle.tar");
    let receipts = state.join("receipts");
    if !receipts.exists() {
        return Err(format!("no receipts dir at {receipts:?}"));
    }
    let status = Command::new("tar")
        .arg("-cf")
        .arg(&bundle)
        .arg("-C")
        .arg(&state)
        .arg("receipts")
        .arg("reports")
        .status()
        .map_err(|e| format!("tar: {e}"))?;
    if !status.success() {
        return Err(format!("tar exit {status:?}"));
    }
    eprintln!("autoresearch: forensics bundle at {bundle:?}");
    Ok(())
}

fn prepare_worktree(
    repo_root: &Path,
    lane_root: &Path,
    use_dirty_source_dev_only: bool,
) -> Result<(), String> {
    if !use_dirty_source_dev_only {
        ensure_trusted_paths_clean(repo_root)?;
    }
    if lane_root.exists() {
        let _ = Command::new("git")
            .current_dir(repo_root)
            .arg("worktree")
            .arg("remove")
            .arg("--force")
            .arg(lane_root)
            .status();
        let _ = fs::remove_dir_all(lane_root);
    }
    if let Some(parent) = lane_root.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("mkdir {parent:?}: {e}"))?;
    }
    let status = Command::new("git")
        .current_dir(repo_root)
        .arg("worktree")
        .arg("add")
        .arg("--detach")
        .arg(lane_root)
        .arg("HEAD")
        .status()
        .map_err(|e| format!("git worktree add: {e}"))?;
    if !status.success() {
        return Err(format!("git worktree add failed for {lane_root:?}"));
    }
    if use_dirty_source_dev_only {
        sync_worktree_path(repo_root, lane_root, "crates/memory-benchmark")?;
        sync_worktree_path(repo_root, lane_root, "crates/cogcore")?;
    }
    Ok(())
}

fn ensure_trusted_paths_clean(repo_root: &Path) -> Result<(), String> {
    let output = Command::new("git")
        .current_dir(repo_root)
        .arg("status")
        .arg("--porcelain")
        .arg("--")
        .arg("crates/memory-benchmark")
        .arg("crates/cogcore")
        .arg("tools/autoresearch")
        .output()
        .map_err(|e| format!("git status trusted paths: {e}"))?;
    if !output.status.success() {
        return Err("git status trusted paths failed".to_string());
    }
    if !output.stdout.is_empty() {
        return Err(format!(
            "trusted paths are dirty; commit first or pass --use-dirty-source-dev-only (non-promotable):\n{}",
            String::from_utf8_lossy(&output.stdout)
        ));
    }
    Ok(())
}

fn sync_worktree_path(repo_root: &Path, lane_root: &Path, rel_path: &str) -> Result<(), String> {
    let source = repo_root.join(rel_path);
    let dest = lane_root.join(rel_path);
    if !source.exists() {
        return Ok(());
    }
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("mkdir {parent:?}: {e}"))?;
    }
    let status = Command::new("rsync")
        .current_dir(repo_root)
        .arg("-a")
        .arg("--delete")
        .arg("--exclude")
        .arg(".git")
        .arg("--exclude")
        .arg("target")
        .arg(format!("{}/", source.display()))
        .arg(format!("{}/", dest.display()))
        .status()
        .map_err(|e| format!("rsync {rel_path}: {e}"))?;
    if !status.success() {
        return Err(format!("rsync failed for {rel_path} into {lane_root:?}"));
    }
    Ok(())
}

fn write_worker_patch(
    lane_root: &Path,
    proposal: &proposer::genetic::Proposal,
) -> Result<(), String> {
    llm::scan_patch(&proposal.patch_content)?;
    let root =
        fs::canonicalize(lane_root).map_err(|e| format!("canonicalize {lane_root:?}: {e}"))?;
    let patch_path = root.join(proposal.patch_path);
    let parent = patch_path
        .parent()
        .ok_or_else(|| format!("missing patch parent for {patch_path:?}"))?;
    let parent = fs::canonicalize(parent).map_err(|e| format!("canonicalize {parent:?}: {e}"))?;
    if !parent.starts_with(&root) {
        return Err(format!(
            "patch path escaped worktree: {:?} -> {:?}",
            proposal.patch_path, parent
        ));
    }
    fs::write(&patch_path, &proposal.patch_content)
        .map_err(|e| format!("write patch {patch_path:?}: {e}"))
}

fn run_reducer(
    repo_root: &Path,
    state: &Path,
    shadow_path: &Path,
    reference_reports: &[PathBuf],
) -> Result<(), String> {
    let mut command = Command::new("cargo");
    command
        .current_dir(repo_root)
        .arg("run")
        .arg("--manifest-path")
        .arg("crates/memory-benchmark/Cargo.toml")
        .arg("--locked")
        .arg("--bin")
        .arg("chase_reduce")
        .arg("--")
        .arg("--lanes")
        .arg(state.join("reports/lanes"))
        .arg("--current-best-state")
        .arg(state.join("best-state.json"))
        .arg("--current-candidates")
        .arg(state.join("reports/lanes"))
        .arg("--scoreboard")
        .arg(state.join("scoreboard.tsv"))
        .arg("--best-state")
        .arg(state.join("best-state.json"))
        .arg("--promotion-decision")
        .arg(state.join("promotion-decision.json"))
        .arg("--negative-memory")
        .arg(state.join("negative-memory.jsonl"))
        .arg("--best-patch")
        .arg(state.join("best.patch"))
        .arg("--curriculum")
        .arg(state.join("curriculum-proposals.json"))
        .arg("--shadow-report")
        .arg(shadow_path);
    for reference in reference_reports {
        command.arg("--reference-report").arg(reference);
    }
    command
        .arg("--out")
        .arg(state.join("reports/final-score.json"))
        .arg("--markdown")
        .arg(state.join("reports/final-score.md"));
    let status = command
        .status()
        .map_err(|e| format!("spawn chase_reduce: {e}"))?;
    if !status.success() {
        return Err(format!("chase_reduce failed with {status:?}"));
    }
    Ok(())
}

fn run_reference_reports(
    repo_root: &Path,
    state: &Path,
    cycle_id: &str,
) -> Result<Vec<PathBuf>, String> {
    let root = state.join("reports/references").join(cycle_id);
    fs::create_dir_all(&root).map_err(|e| format!("mkdir references: {e}"))?;
    let mut reports = Vec::new();
    for candidate in [
        "reference_context_pack",
        "reference_evidence_ledger",
        "reference_claim_skeptic",
    ] {
        let out = root.join(format!("{candidate}.json"));
        run_northstar(repo_root, candidate, "reference-public-0001", &out, "", "")?;
        reports.push(out);
    }
    Ok(reports)
}

fn run_northstar(
    worktree_root: &Path,
    candidate: &str,
    seed: &str,
    out: &Path,
    patch_content: &str,
    patch_path: &str,
) -> Result<(), String> {
    let worktree_root = fs::canonicalize(worktree_root)
        .map_err(|e| format!("canonicalize {worktree_root:?}: {e}"))?;
    let out = if out.is_absolute() {
        out.to_path_buf()
    } else {
        env::current_dir()
            .map_err(|e| format!("cwd: {e}"))?
            .join(out)
    };
    let northstar_dir = worktree_root.join("target/memory-benchmark/northstar");
    fs::create_dir_all(&northstar_dir).map_err(|e| format!("mkdir {northstar_dir:?}: {e}"))?;
    let t0 = northstar_dir.join("t0.json");
    let t1 = northstar_dir.join("t1.json");
    let compounding = northstar_dir.join("compounding.json");
    let hardening = northstar_dir.join("hardening.json");
    let qbank = northstar_dir.join("qbank.json");
    let report = worktree_root.join("target/memory-benchmark/northstar.json");

    run_bench(&worktree_root, candidate, "public", &[], &t0)?;
    run_bench(
        &worktree_root,
        candidate,
        "generated",
        &[("--seed", seed), ("--fixtures", "120")],
        &t1,
    )?;
    run_bench(
        &worktree_root,
        candidate,
        "compounding",
        &[("--seed", "compound-public-0001"), ("--fixtures", "24")],
        &compounding,
    )?;
    run_bench(
        &worktree_root,
        candidate,
        "hardening",
        &[("--seed", "harden-public-0001"), ("--fixtures", "20")],
        &hardening,
    )?;
    run_bench(
        &worktree_root,
        candidate,
        "real-papers",
        &[
            (
                "--paper-bank",
                "crates/memory-benchmark/data/real-paper-bank",
            ),
            ("--qbank-top-n", "50"),
        ],
        &qbank,
    )?;
    run_score_mix(
        &worktree_root,
        &[
            ("t0", "0.10", &t0),
            ("t1", "0.30", &t1),
            ("compounding", "0.20", &compounding),
            ("hardening", "0.15", &hardening),
            ("qbank", "0.20", &qbank),
        ],
        &report,
    )?;
    if !report.exists() {
        return Err(format!("missing northstar report at {report:?}"));
    }
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("mkdir {parent:?}: {e}"))?;
    }
    let wrapped = wrap_report(&report, patch_content, patch_path, FIXTURE_QBANK_DEV_ONLY)?;
    fs::write(&out, wrapped).map_err(|e| format!("write {out:?}: {e}"))?;
    Ok(())
}

fn run_bench(
    worktree_root: &Path,
    candidate: &str,
    suite: &str,
    extra: &[(&str, &str)],
    out: &Path,
) -> Result<(), String> {
    let mut command = Command::new("cargo");
    command
        .current_dir(worktree_root)
        .arg("run")
        .arg("--manifest-path")
        .arg("crates/memory-benchmark/Cargo.toml")
        .arg("--locked")
        .arg("--bin")
        .arg("bench")
        .arg("--")
        .arg("--candidate")
        .arg(candidate)
        .arg("--suite")
        .arg(suite);
    for (flag, value) in extra {
        command.arg(flag).arg(value);
    }
    if suite == "real-papers" {
        command.env("memory_benchmark_dev_qbank", "1");
    }
    command.arg("--out").arg(out);
    let status = command.status().map_err(|e| format!("spawn cargo: {e}"))?;
    if !status.success() {
        return Err(format!("bench {suite} failed with {status:?}"));
    }
    Ok(())
}

fn run_score_mix(
    worktree_root: &Path,
    inputs: &[(&str, &str, &Path)],
    out: &Path,
) -> Result<(), String> {
    let mut command = Command::new("cargo");
    command
        .current_dir(worktree_root)
        .arg("run")
        .arg("--manifest-path")
        .arg("crates/memory-benchmark/Cargo.toml")
        .arg("--locked")
        .arg("--bin")
        .arg("score_mix")
        .arg("--")
        .arg("--name")
        .arg("northstar");
    for (name, weight, path) in inputs {
        command
            .arg("--input")
            .arg(format!("{name}:{weight}:{}", path.display()));
    }
    command.arg("--out").arg(out);
    let status = command
        .status()
        .map_err(|e| format!("spawn score_mix: {e}"))?;
    if !status.success() {
        return Err(format!("score_mix failed with {status:?}"));
    }
    Ok(())
}

fn wrap_report(
    report: &Path,
    patch_content: &str,
    patch_path: &str,
    dev_only: bool,
) -> Result<String, String> {
    let text = fs::read_to_string(report).map_err(|e| format!("read {report:?}: {e}"))?;
    let mut parsed = json::parse(&text).map_err(|e| format!("parse {report:?}: {e}"))?;
    let Json::Object(ref mut obj) = parsed else {
        return Err(format!("northstar report is not a JSON object: {report:?}"));
    };
    if !patch_content.is_empty() {
        obj.insert("patch".to_string(), Json::Str(patch_content.to_string()));
    }
    if !patch_path.is_empty() {
        obj.insert("patch_path".to_string(), Json::Str(patch_path.to_string()));
    }
    obj.insert("dev_only".to_string(), Json::Bool(dev_only));
    if dev_only {
        obj.insert(
            "dev_only_reason".to_string(),
            Json::Str("checked-in qbank uses fixture papers".to_string()),
        );
    }
    Ok(format!("{}\n", parsed.to_string()))
}

#[derive(Clone)]
struct LaneResult {
    worker_id: String,
    total: f64,
    lane_root: PathBuf,
    patch_content: String,
    patch_path: String,
}

// ───────── helpers ─────────

fn state_dir(f: &Flags) -> PathBuf {
    PathBuf::from(f.state_dir.clone().unwrap_or_else(|| STATE_DIR.to_string()))
}

fn count_receipts(dir: &Path) -> usize {
    fs::read_dir(dir)
        .map(|entries| entries.flatten().count())
        .unwrap_or(0)
}

fn extract_total(text: &str) -> Result<f64, String> {
    let parsed = json::parse(text).map_err(|e| format!("parse lane report: {e}"))?;
    let Json::Object(obj) = parsed else {
        return Err("lane report is not a JSON object".to_string());
    };
    match obj.get("total") {
        Some(Json::Float(value)) => Ok(*value),
        Some(Json::Int(value)) => Ok(*value as f64),
        other => Err(format!(
            "lane report missing numeric top-level total: {other:?}"
        )),
    }
}

fn build_receipt(
    cycle_id: &str,
    workers: usize,
    candidate: &str,
    scores: &[LaneResult],
    dev_only: bool,
    reference_reports: &[PathBuf],
) -> String {
    let mut top = BTreeMap::new();
    top.insert("cycle_id".to_string(), format!("\"{cycle_id}\""));
    top.insert("candidate".to_string(), format!("\"{candidate}\""));
    top.insert("workers".to_string(), workers.to_string());
    top.insert("dev_only".to_string(), dev_only.to_string());
    top.insert("attempted".to_string(), scores.len().to_string());
    top.insert(
        "reference_report_count".to_string(),
        reference_reports.len().to_string(),
    );
    let best_total: f64 = scores.first().map(|lane| lane.total).unwrap_or(0.0);
    top.insert("best_total".to_string(), format!("{:.4}", best_total));
    let median = if scores.is_empty() {
        0.0
    } else {
        let mid = scores.len() / 2;
        scores[mid].total
    };
    top.insert("median_total".to_string(), format!("{:.4}", median));
    let mut body = String::from("{");
    for (i, (k, v)) in top.iter().enumerate() {
        if i > 0 {
            body.push(',');
        }
        body.push('"');
        body.push_str(k);
        body.push_str("\":");
        body.push_str(v);
    }
    body.push('}');
    body.push('\n');
    body
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_total_uses_only_top_level_total() {
        let report = r#"{"total":80.0,"nested":{"total":1000.0}}"#;
        assert_eq!(extract_total(report).unwrap(), 80.0);
    }

    #[test]
    fn wrap_report_marks_dev_only_and_preserves_patch_metadata() {
        let dir = env::temp_dir().join(format!("autoresearch-wrap-report-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let report = dir.join("northstar.json");
        fs::write(&report, r#"{"name":"northstar","total":80.0}"#).unwrap();

        let wrapped = wrap_report(
            &report,
            "diff --git a/crates/cogcore/src/config.rs b/crates/cogcore/src/config.rs\n",
            "config.patch",
            true,
        )
        .unwrap();
        let parsed = json::parse(&wrapped).unwrap();
        let Json::Object(obj) = parsed else {
            panic!("wrapped report must be an object");
        };
        assert!(matches!(obj.get("dev_only"), Some(Json::Bool(true))));
        assert!(matches!(obj.get("patch"), Some(Json::Str(_))));
        assert_eq!(
            obj.get("patch_path"),
            Some(&Json::Str("config.patch".to_string()))
        );

        let _ = fs::remove_file(report);
        let _ = fs::remove_dir(dir);
    }
}
