//! Section: Jankurai Runner
//!
//! Spawns `jankurai audit` as a subprocess and watches `agent/repo-score.json`
//! for changes.  The runner hands results back to the app via a channel that
//! produces `Action::JankuraiScoreUpdate` with a parsed `AuditSummary`.
//!
//! Call `run_audit(tx)` to fire-and-forget the audit.  The function returns
//! immediately; the audit runs on a background thread.
//!
//! Standard invocation mirrors the AGENTS.md canonical form:
//!   jankurai audit . --mode advisory --json agent/repo-score.json --md agent/repo-score.md

use std::io::{BufRead, BufReader};
use std::process::Stdio;
use std::sync::mpsc::Sender;

use crate::action::{Action, AuditFinding, AuditSummary};

const AUDIT_ARGS: &[&str] = &[
    "audit",
    ".",
    "--mode",
    "advisory",
    "--json",
    "agent/repo-score.json",
    "--md",
    "agent/repo-score.md",
];

/// Run `jankurai audit` in the background and push progress / completion
/// actions through `tx`.
///
/// The caller is responsible for passing a clone of the main app sender.
pub fn run_audit(tx: Sender<Action>) {
    std::thread::spawn(move || {
        let child = std::process::Command::new("jankurai")
            .args(AUDIT_ARGS)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn();

        let mut child = match child {
            Ok(c) => c,
            Err(_) => {
                let _ = tx.send(Action::JankuraiScoreUpdate {
                    success: false,
                    summary: None,
                });
                return;
            }
        };

        // Drain stderr in a background thread so it doesn't block.
        let stderr = child.stderr.take();
        let tx_err = tx.clone();
        let stderr_thread = stderr.map(|se| {
            std::thread::spawn(move || {
                let reader = BufReader::new(se);
                for line in reader.lines().map_while(Result::ok) {
                    let trimmed = line.trim().to_string();
                    if !trimmed.is_empty() {
                        let _ = tx_err.send(Action::JankuraiAuditLine(trimmed));
                    }
                }
            })
        });

        // Drain stdout on the current thread.
        if let Some(stdout) = child.stdout.take() {
            let reader = BufReader::new(stdout);
            for line in reader.lines().map_while(Result::ok) {
                let trimmed = line.trim().to_string();
                if !trimmed.is_empty() {
                    let _ = tx.send(Action::JankuraiAuditLine(trimmed));
                }
            }
        }

        // Wait for stderr thread to finish.
        if let Some(handle) = stderr_thread {
            let _ = handle.join();
        }

        // Collect exit status and parse the results JSON.
        match child.wait() {
            Ok(s) if s.success() => {
                let summary = parse_audit_json("agent/repo-score.json");
                let _ = tx.send(Action::JankuraiScoreUpdate {
                    success: true,
                    summary,
                });
            }
            _ => {
                let _ = tx.send(Action::JankuraiScoreUpdate {
                    success: false,
                    summary: None,
                });
            }
        }
    });
}

/// Run a full jankurai cycle: audit → jankurai-runner --once → re-audit.
///
/// Progress lines are forwarded as `Action::JankuraiRunnerLine`. The cycle
/// ends with `Action::JankuraiCycleComplete { improved }` where `improved`
/// reflects whether the re-audit score is higher than the initial score.
pub fn run_cycle(tx: Sender<Action>) {
    std::thread::spawn(move || {
        // Step 1: initial audit.
        let initial_score = run_audit_inner(&tx);

        // Step 2: if no findings (or audit failed), skip the runner.
        let has_issues = match &initial_score {
            Some(s) => s.hard_findings > 0 || s.soft_findings > 0 || s.caps_count > 0,
            None => false,
        };

        if has_issues {
            // Step 3: run jankurai-runner --once to apply the top fix.
            let _ = tx.send(Action::JankuraiRunnerLine(
                "launching jankurai-runner --once …".to_string(),
            ));
            let runner_child = std::process::Command::new("jankurai-runner")
                .args(["--once"])
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn();

            if let Ok(mut runner) = runner_child {
                let stderr = runner.stderr.take();
                let tx_err = tx.clone();
                let stderr_thread = stderr.map(|se| {
                    std::thread::spawn(move || {
                        let reader = std::io::BufReader::new(se);
                        for line in std::io::BufRead::lines(reader).map_while(Result::ok) {
                            let trimmed = line.trim().to_string();
                            if !trimmed.is_empty() {
                                let _ = tx_err.send(Action::JankuraiRunnerLine(trimmed));
                            }
                        }
                    })
                });
                if let Some(stdout) = runner.stdout.take() {
                    let reader = std::io::BufReader::new(stdout);
                    for line in std::io::BufRead::lines(reader).map_while(Result::ok) {
                        let trimmed = line.trim().to_string();
                        if !trimmed.is_empty() {
                            let _ = tx.send(Action::JankuraiRunnerLine(trimmed));
                        }
                    }
                }
                if let Some(h) = stderr_thread {
                    let _ = h.join();
                }
                let _ = runner.wait();
            } else {
                let _ = tx.send(Action::JankuraiRunnerLine(
                    "jankurai-runner not found — skipping fix step".to_string(),
                ));
            }

            // Step 4: re-audit to verify.
            let _ = tx.send(Action::JankuraiRunnerLine("re-auditing …".to_string()));
            let final_score = run_audit_inner(&tx);

            let improved = match (initial_score, final_score) {
                (Some(before), Some(after)) => after.score > before.score,
                _ => false,
            };
            let _ = tx.send(Action::JankuraiCycleComplete { improved });
        } else {
            // No issues — cycle is trivially complete.
            let _ = tx.send(Action::JankuraiCycleComplete { improved: false });
        }
    });
}

/// Internal helper: runs `jankurai audit`, streams lines, sends `JankuraiScoreUpdate`,
/// and returns the parsed summary.
fn run_audit_inner(tx: &Sender<Action>) -> Option<AuditSummary> {
    let child = std::process::Command::new("jankurai")
        .args(AUDIT_ARGS)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn();

    let mut child = child.ok()?;

    let stderr = child.stderr.take();
    let tx_err = tx.clone();
    let stderr_thread = stderr.map(|se| {
        std::thread::spawn(move || {
            let reader = std::io::BufReader::new(se);
            for line in std::io::BufRead::lines(reader).map_while(Result::ok) {
                let trimmed = line.trim().to_string();
                if !trimmed.is_empty() {
                    let _ = tx_err.send(Action::JankuraiAuditLine(trimmed));
                }
            }
        })
    });

    if let Some(stdout) = child.stdout.take() {
        let reader = std::io::BufReader::new(stdout);
        for line in std::io::BufRead::lines(reader).map_while(Result::ok) {
            let trimmed = line.trim().to_string();
            if !trimmed.is_empty() {
                let _ = tx.send(Action::JankuraiAuditLine(trimmed));
            }
        }
    }

    if let Some(h) = stderr_thread {
        let _ = h.join();
    }

    match child.wait() {
        Ok(s) if s.success() => {
            let summary = parse_audit_json("agent/repo-score.json");
            let _ = tx.send(Action::JankuraiScoreUpdate {
                success: true,
                summary: summary.clone(),
            });
            summary
        }
        _ => {
            let _ = tx.send(Action::JankuraiScoreUpdate {
                success: false,
                summary: None,
            });
            None
        }
    }
}

/// Parse `agent/repo-score.json` into an [`AuditSummary`].
///
/// Returns `None` if the file is missing or malformed — the caller should
/// fall back to the simple "Audit complete" card.
fn parse_audit_json(path: &str) -> Option<AuditSummary> {
    let content = std::fs::read_to_string(path).ok()?;
    let doc: serde_json::Value = serde_json::from_str(&content).ok()?;

    let score = doc.get("score")?.as_u64()?;
    let raw_score = doc.get("raw_score")?.as_u64().unwrap_or(score);

    let caps: Vec<String> = doc
        .get("caps_applied")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();

    let decision = doc.get("decision");
    let hard_findings = decision
        .and_then(|d| d.get("hard_findings"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let soft_findings = decision
        .and_then(|d| d.get("soft_findings"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    let blockers: Vec<String> = doc
        .get("conformance_blockers")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();

    // Extract findings with agent_fix hints — these are the actionable ones.
    let actionable_findings: Vec<AuditFinding> = doc
        .get("findings")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|f| {
                    let agent_fix = f.get("agent_fix")?.as_str()?.to_string();
                    if agent_fix.is_empty() {
                        return None;
                    }
                    Some(AuditFinding {
                        severity: f
                            .get("severity")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown")
                            .to_string(),
                        problem: f
                            .get("problem")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        agent_fix,
                        path: f
                            .get("path")
                            .and_then(|v| v.as_str())
                            .unwrap_or(".")
                            .to_string(),
                        rule_id: f
                            .get("rule_id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        line: f.get("line").and_then(|v| v.as_u64()),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    Some(AuditSummary {
        score,
        raw_score,
        caps_count: caps.len(),
        caps,
        hard_findings,
        soft_findings,
        blockers,
        actionable_findings,
    })
}
