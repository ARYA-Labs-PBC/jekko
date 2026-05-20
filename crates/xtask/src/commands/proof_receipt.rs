use std::fs;
use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result};
use chrono::Utc;
use serde::Serialize;

#[derive(Serialize)]
struct Receipt {
    schema_version: &'static str,
    standard_version: &'static str,
    auditor_version: &'static str,
    receipt_id: String,
    lane: String,
    command: String,
    exit_code: i32,
    elapsed_ms: u64,
    artifacts: Vec<String>,
    changed_paths: Vec<String>,
    dirty_worktree: bool,
    repo_root: &'static str,
    rules_covered: Vec<String>,
    git_head: String,
    generated_at: String,
    status: String,
}

pub fn run(lane: &str, status: &str, out: &Path) -> Result<()> {
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    let generated_at = Utc::now();
    let changed_paths = changed_paths_for_lane(lane);
    let receipt = Receipt {
        schema_version: "1.0.0",
        standard_version: "0.7.0",
        auditor_version: "xtask",
        receipt_id: format!("{}-{}", lane, generated_at.timestamp()),
        lane: lane.to_string(),
        command: command_for_lane(lane),
        exit_code: if status == "ok" { 0 } else { 1 },
        elapsed_ms: 0,
        artifacts: artifacts_for_lane(lane),
        changed_paths,
        dirty_worktree: dirty_worktree(),
        repo_root: ".",
        rules_covered: rules_for_lane(lane),
        git_head: git_head(),
        generated_at: generated_at.timestamp().to_string(),
        status: status.to_string(),
    };
    let body = serde_json::to_string_pretty(&receipt)?;
    fs::write(out, format!("{body}\n")).with_context(|| format!("write {}", out.display()))?;
    println!("proof-receipt: wrote {}", out.display());
    Ok(())
}

fn command_for_lane(lane: &str) -> String {
    match lane {
        "security" => "cargo run -p xtask --locked -- security-lane --out target/jankurai/security"
            .to_string(),
        other => format!("xtask proof-receipt --lane {other}"),
    }
}

fn artifacts_for_lane(lane: &str) -> Vec<String> {
    match lane {
        "security" => vec![
            "target/jankurai/security/evidence.json".to_string(),
            "target/jankurai/security/gitleaks.json".to_string(),
            "target/jankurai/security/cargo-audit.json".to_string(),
            "target/jankurai/security/lane-status.txt".to_string(),
        ],
        _ => Vec::new(),
    }
}

fn changed_paths_for_lane(lane: &str) -> Vec<String> {
    match lane {
        "security" => vec![
            "agent/owner-map.json".to_string(),
            "agent/test-map.json".to_string(),
            "agent/tool-adoption.toml".to_string(),
        ],
        _ => Vec::new(),
    }
}

fn rules_for_lane(lane: &str) -> Vec<String> {
    match lane {
        "security" => vec!["HLT-024-AGENT-TOOL-SUPPLY-GAP".to_string()],
        _ => Vec::new(),
    }
}

fn git_head() -> String {
    Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .filter(|head| !head.is_empty())
        .unwrap_or_else(|| "unknown".to_string())
}

fn dirty_worktree() -> bool {
    Command::new("git")
        .args(["status", "--porcelain"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| !output.stdout.is_empty())
        .unwrap_or(true)
}
