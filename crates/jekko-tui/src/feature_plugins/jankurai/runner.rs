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

/// Run `jankurai audit` in the background and push progress / completion
/// actions through `tx`.
///
/// The caller is responsible for passing a clone of the main app sender.
pub fn run_audit(tx: Sender<Action>) {
    std::thread::spawn(move || {
        let child = std::process::Command::new("jankurai")
            .args([
                "audit",
                ".",
                "--mode",
                "advisory",
                "--json",
                "agent/repo-score.json",
                "--md",
                "agent/repo-score.md",
            ])
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
