use anyhow::{bail, Context, Result};
use serde::Serialize;
use std::fs;
use std::path::Path;
use std::process::Command as ProcessCommand;

use crate::cleanup_cutover;
use crate::shared::repo_root;

#[derive(Copy, Clone, Debug, Eq, PartialEq, clap::ValueEnum)]
pub(crate) enum GuardMode {
    Advisory,
    Final,
}

#[derive(Debug, Serialize)]
struct GuardHit {
    path: String,
    pattern: String,
}

pub(crate) fn run_preflight() -> Result<()> {
    let root = repo_root()?;
    println!("preflight: pre-cutover readiness report\n");

    let mut failures: Vec<String> = Vec::new();

    let plan = cleanup_cutover::compute_plan(&root);
    let total = plan.delete_files.len() + plan.delete_dirs.len() + plan.edit_files.len();
    println!(
        "  [{}] cleanup-cutover plan: {} delete_files, {} delete_dirs, {} edit_files (total {})",
        if total >= 50 { "OK" } else { "WARN" },
        plan.delete_files.len(),
        plan.delete_dirs.len(),
        plan.edit_files.len(),
        total
    );
    if total < 50 {
        failures.push("cleanup-cutover plan looks too small (<50 paths)".into());
    }

    let ops_dir = root.join("ops/ci");
    let bun_ops = if ops_dir.exists() {
        let mut hits: Vec<String> = Vec::new();
        for entry in fs::read_dir(&ops_dir).context("read ops/ci")? {
            let entry = entry?;
            let p = entry.path();
            if p.extension().and_then(|e| e.to_str()) != Some("sh") {
                continue;
            }
            if let Ok(text) = fs::read_to_string(&p) {
                for line in text.lines() {
                    let t = line.trim();
                    if t.starts_with("bun ")
                        || t == "bun"
                        || t.starts_with("bun install")
                        || t.starts_with("bun run")
                    {
                        hits.push(p.strip_prefix(&root).unwrap_or(&p).display().to_string());
                        break;
                    }
                }
            }
        }
        hits
    } else {
        Vec::new()
    };
    println!(
        "  [{}] ops/ci/*.sh free of non-Rust runtime calls: {} script(s) still live",
        if bun_ops.is_empty() { "OK" } else { "FAIL" },
        bun_ops.len()
    );
    if !bun_ops.is_empty() {
        for s in &bun_ops {
            println!("       - {s}");
        }
        failures.push(format!(
            "{} ops/ci script(s) still call the non-Rust runtime",
            bun_ops.len()
        ));
    }

    let workflows_dir = root.join(".github/workflows");
    let setup_bun_hits = if workflows_dir.exists() {
        let mut hits: Vec<String> = Vec::new();
        for entry in fs::read_dir(&workflows_dir).context("read workflows")? {
            let entry = entry?;
            let p = entry.path();
            if p.extension().and_then(|e| e.to_str()) != Some("yml") {
                continue;
            }
            if let Ok(text) = fs::read_to_string(&p) {
                if text.contains("setup-bun") {
                    hits.push(p.strip_prefix(&root).unwrap_or(&p).display().to_string());
                }
            }
        }
        hits
    } else {
        Vec::new()
    };
    println!(
        "  [{}] .github/workflows/*.yml free of Bun setup action: {} workflow(s) still live",
        if setup_bun_hits.is_empty() {
            "OK"
        } else {
            "FAIL"
        },
        setup_bun_hits.len()
    );
    if !setup_bun_hits.is_empty() {
        for s in &setup_bun_hits {
            println!("       - {s}");
        }
        failures.push(format!(
            "{} workflow(s) still reference the Bun setup action",
            setup_bun_hits.len()
        ));
    }

    let critical = [
        "Cargo.toml",
        "Cargo.lock",
        "crates",
        "agent",
        "db",
        ".github/workflows/parity.yml",
    ];
    let mut critical_violations: Vec<String> = Vec::new();
    for rel in &critical {
        let abs = root.join(rel);
        if plan.delete_files.iter().any(|p| p == &abs) || plan.delete_dirs.iter().any(|p| p == &abs)
        {
            critical_violations.push((*rel).to_string());
        }
    }
    println!(
        "  [{}] workspace-critical paths NOT in delete lists: {} violations",
        if critical_violations.is_empty() {
            "OK"
        } else {
            "FAIL"
        },
        critical_violations.len()
    );
    if !critical_violations.is_empty() {
        for s in &critical_violations {
            println!("       - {s} would be deleted by cutover!");
        }
        failures.push(format!(
            "{} critical paths in delete list",
            critical_violations.len()
        ));
    }

    let required_in_plan = ["bun.lock", "package.json", "tsconfig.json"];
    let mut missing: Vec<String> = Vec::new();
    for rel in &required_in_plan {
        let abs = root.join(rel);
        if !plan.delete_files.iter().any(|p| p == &abs) {
            missing.push((*rel).to_string());
        }
    }
    println!(
        "  [{}] root manifests in delete plan: {} missing",
        if missing.is_empty() { "OK" } else { "FAIL" },
        missing.len()
    );
    if !missing.is_empty() {
        failures.push(format!(
            "{} root manifests missing from cleanup plan",
            missing.len()
        ));
    }

    println!();
    if failures.is_empty() {
        println!("preflight: ✓ PASS — workspace ready for cleanup-cutover --execute");
        Ok(())
    } else {
        println!("preflight: ✗ FAIL — {} blocker(s) remain:", failures.len());
        for f in &failures {
            println!("  • {f}");
        }
        bail!("preflight: {} unresolved blocker(s)", failures.len());
    }
}

pub(crate) fn run_cleanup_cutover(execute: bool) -> Result<()> {
    let root = repo_root()?;
    let plan = cleanup_cutover::compute_plan(&root);

    let mode = if execute { "EXECUTE" } else { "DRY-RUN" };
    println!("cleanup-cutover ({mode})");
    println!(
        "  delete_files: {} / delete_dirs: {} / edit_files: {}",
        plan.delete_files.len(),
        plan.delete_dirs.len(),
        plan.edit_files.len()
    );

    for path in &plan.delete_files {
        let abs = root.join(path);
        let exists = abs.exists();
        if !execute {
            println!(
                "  file  : {} {}",
                path.display(),
                if exists { "(exists)" } else { "(missing)" }
            );
            continue;
        }
        if !exists {
            continue;
        }
        fs::remove_file(&abs).with_context(|| format!("rm file {}", abs.display()))?;
        println!("  rm    : {}", path.display());
    }

    for path in &plan.delete_dirs {
        let abs = root.join(path);
        let exists = abs.exists();
        if !execute {
            println!(
                "  dir   : {} {}",
                path.display(),
                if exists { "(exists)" } else { "(missing)" }
            );
            continue;
        }
        if !exists {
            continue;
        }
        fs::remove_dir_all(&abs).with_context(|| format!("rm dir {}", abs.display()))?;
        println!("  rm -r : {}", path.display());
    }

    for path in &plan.edit_files {
        println!(
            "  edit  : {} {}",
            path.display(),
            if execute {
                "(skipped: scrub-by-hand or follow-up packet)"
            } else {
                "(needs scrub)"
            }
        );
    }

    if !execute {
        println!();
        println!("preview complete. re-run with --execute to actually remove these paths.");
    } else {
        println!();
        println!("cleanup-cutover: done. Now run `guard-forbidden-runtime --mode final`.");
    }

    Ok(())
}

pub(crate) fn guard_forbidden_runtime(mode: GuardMode) -> Result<()> {
    let root = repo_root()?;
    let patterns = [
        "@opentui",
        "opentui-spinner",
        "solid-js",
        "bun:test",
        "Bun.",
        "bun:",
        "bunfig",
        "bun.lock",
        "@types/bun",
        "@tsconfig/bun",
        "vite",
        "@vite",
        "vite.config",
    ];
    let allow_prefixes = [
        root.join("target"),
        root.join(".git"),
        root.join("tips/goodbye_OpenTUIBun"),
        root.join("docs/archive/historical/open-tui-bun-inventory.md"),
        root.join("docs/archive/historical/open-tui-bun-rust-port.md"),
        root.join("docs/archive/historical/open-tui-bun-deletion-plan.md"),
        root.join("docs/ZYAL"),
        root.join("docs/ZYAL_MISSION.md"),
        root.join("achat.md"),
        root.join("CHANGELOG.md"),
        root.join("README.md"),
        root.join("CONTRIBUTING.md"),
        root.join("UNLOCK_WORKPLAN.md"),
        root.join("ZYAL_WORKFLOW.md"),
        root.join("JANKURAI_TASKLIST.md"),
        root.join("agent/TUI_UPGRADE.md"),
        root.join("agent/proofs"),
        root.join("agent/baselines"),
        root.join("agent/repo-score.json"),
        root.join("agent/repo-score.md"),
        root.join("agent/owner-map.json"),
        root.join("agent/standard-version.toml"),
        root.join("agent/audit-policy.toml"),
        root.join("agent/jankurai-install.toml"),
        root.join("agent/generated-zones.toml"),
        root.join(".jekko/agent/generated-zones.toml"),
        root.join("crates/memory-benchmark/data"),
        root.join("crates/xtask"),
    ];

    let mut hits = Vec::new();
    let output = ProcessCommand::new("git")
        .args(["ls-files", "-co", "--exclude-standard", "--full-name"])
        .current_dir(&root)
        .output()
        .context("running git ls-files")?;

    if !output.status.success() {
        bail!("git ls-files failed with status {}", output.status);
    }

    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let path = root.join(line);
        if allow_prefixes.iter().any(|prefix| path.starts_with(prefix)) {
            continue;
        }
        if !is_text_candidate(&path) {
            continue;
        }
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(_) => continue,
        };
        for pattern in patterns {
            if text.contains(pattern) {
                hits.push(GuardHit {
                    path: path
                        .strip_prefix(&root)
                        .unwrap_or(&path)
                        .display()
                        .to_string(),
                    pattern: pattern.to_string(),
                });
            }
        }
    }

    if hits.is_empty() {
        println!("no forbidden runtime references found");
        return Ok(());
    }

    for hit in &hits {
        println!("{}: {}", hit.path, hit.pattern);
    }

    if mode == GuardMode::Final {
        bail!("forbidden runtime references found: {}", hits.len());
    }

    Ok(())
}

fn is_text_candidate(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|v| v.to_str()),
        Some(
            "rs" | "ts"
                | "tsx"
                | "js"
                | "jsx"
                | "json"
                | "jsonc"
                | "toml"
                | "md"
                | "yml"
                | "yaml"
                | "sh"
                | "nix"
        )
    )
}
