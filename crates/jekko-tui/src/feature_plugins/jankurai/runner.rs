//! Section: Jankurai Runner
//!
//! Spawns `jankurai audit` as a subprocess and watches `agent/repo-score.json`
//! for changes.  The runner hands results back to the app via a channel that
//! produces `Action::JankuraiScoreUpdate`.
//!
//! Call `run_audit(tx)` to fire-and-forget the audit.  The function returns
//! immediately; the audit runs on a background thread.
//!
//! Standard invocation mirrors the AGENTS.md canonical form:
//!   jankurai audit . --mode advisory --json agent/repo-score.json --md agent/repo-score.md

use std::io::{BufRead, BufReader};
use std::process::Stdio;
use std::sync::mpsc::Sender;

use crate::action::Action;

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
                let _ = tx.send(Action::JankuraiScoreUpdate { success: false });
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

        // Collect exit status.
        match child.wait() {
            Ok(s) if s.success() => {
                let _ = tx.send(Action::JankuraiScoreUpdate { success: true });
            }
            _ => {
                let _ = tx.send(Action::JankuraiScoreUpdate { success: false });
            }
        }
    });
}
