//! Non-interactive process runner (COWBOY.md F1).
//!
//! Spawns `tokio::process::Command`, streams stdout/stderr line-by-line as
//! `ToolEvent` chunks, emits `Start`/`Complete`/`Fail` bookends. Caller owns
//! the channel — runner just produces events.

use std::process::Stdio;
use std::time::Duration;

use anyhow::{Context, Result};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;

use crate::action::ToolEvent;
use crate::engine::cancel::{CancelLevel, CancellationToken};
#[cfg(target_os = "linux")]
use crate::engine::sandbox_linux;
use crate::engine::sandbox_macos;
use crate::engine::sandbox_policy::SandboxPolicy;

#[derive(Clone, Debug, Default)]
pub struct PlainCommand {
    pub id: String,
    pub label: String,
    pub program: String,
    pub args: Vec<String>,
    /// Optional shared cancellation handle. When the token reaches `Hard` or
    /// `Force`, the runner sends `start_kill` to the child.
    pub cancel: Option<CancellationToken>,
    /// Optional sandbox policy. `None` preserves the legacy unrestricted
    /// behavior (cwd inherited, env inherited). When `Some`, the policy is
    /// applied to the underlying `tokio::process::Command` before spawn —
    /// see `engine::sandbox_policy` for what's actually enforced (cwd + env
    /// only in v1; path/network isolation is documented out of scope).
    pub policy: Option<SandboxPolicy>,
}

pub async fn run(cmd: PlainCommand, tx: mpsc::Sender<ToolEvent>) -> Result<()> {
    let _ = tx
        .send(ToolEvent::Start {
            id: cmd.id.clone(),
            name: cmd.label.clone(),
            input: Some(format!("{} {}", cmd.program, cmd.args.join(" "))),
        })
        .await;

    // T-SANDBOX-FS-ISOLATION-MACOS: on macOS, if a policy with fs scope
    // is set, `wrap_command` rewrites (program, args) to invoke
    // the macOS sandbox wrapper. On any
    // other host, or when policy is None / has no fs scope, the tuple
    // comes back unchanged and we spawn exactly as before.
    let (effective_program, effective_args) = match cmd.policy.as_ref() {
        Some(policy) => sandbox_macos::wrap_command(&cmd.program, &cmd.args, policy),
        None => (cmd.program.clone(), cmd.args.clone()),
    };

    let mut command = Command::new(&effective_program);
    command
        .args(&effective_args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(policy) = cmd.policy.as_ref() {
        policy.apply_to_command(&mut command);
    }

    // T-SANDBOX-FS-ISOLATION-LINUX: on Linux, install the landlock
    // ruleset in the forked child via `pre_exec` so the kernel enforces
    // fs scope from the moment exec*() runs. The jekko TUI parent is
    // unaffected; only the spawned child inherits the restrictions.
    // (`tokio::process::Command::pre_exec` is an inherent unix-cfg
    // method; no extra trait import is needed.)
    //
    #[cfg(target_os = "linux")]
    if let Some(policy) = cmd.policy.as_ref() {
        let policy_for_child = policy.clone();
        // SAFETY: the closure runs between fork(2) and execve(2). The body
        // only applies the prebuilt Landlock policy and performs no logging.
        unsafe {
            command.pre_exec(move || sandbox_linux::apply_landlock(&policy_for_child));
        }
    }

    let mut child = command
        .spawn()
        .with_context(|| format!("spawn {}", cmd.program))?;

    let stdout = child.stdout.take().expect("stdout piped");
    let stderr = child.stderr.take().expect("stderr piped");

    let id_stdout = cmd.id.clone();
    let tx_stdout = tx.clone();
    let stdout_task = tokio::spawn(async move {
        let mut lines = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            if tx_stdout
                .send(ToolEvent::StdoutChunk {
                    id: id_stdout.clone(),
                    chunk: line,
                })
                .await
                .is_err()
            {
                break;
            }
        }
    });

    let id_stderr = cmd.id.clone();
    let tx_stderr = tx.clone();
    let stderr_task = tokio::spawn(async move {
        let mut lines = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            if tx_stderr
                .send(ToolEvent::StderrChunk {
                    id: id_stderr.clone(),
                    chunk: line,
                })
                .await
                .is_err()
            {
                break;
            }
        }
    });

    // Wait loop: race child exit against periodic cancellation checks. v1
    // policy — Hard/Force trigger `start_kill` (SIGKILL on Unix via tokio).
    // Soft is advisory only here (no SIGINT delivery without libc/nix); the
    // UI shows the spinner, and a second Esc within 2s promotes to Hard via
    // the `Escalator`.
    let mut cancelled = false;
    let status = loop {
        tokio::select! {
            wait = child.wait() => {
                break wait.with_context(|| format!("wait {}", cmd.program))?;
            }
            _ = tokio::time::sleep(Duration::from_millis(50)) => {
                if let Some(tok) = &cmd.cancel {
                    if matches!(tok.level(), CancelLevel::Hard | CancelLevel::Force) && !cancelled {
                        let _ = child.start_kill();
                        cancelled = true;
                    }
                }
            }
        }
    };

    let _ = stdout_task.await;
    let _ = stderr_task.await;

    if status.success() && !cancelled {
        let _ = tx.send(ToolEvent::Complete { id: cmd.id }).await;
    } else {
        let _ = tx
            .send(ToolEvent::Fail {
                id: cmd.id,
                error: if cancelled {
                    "cancelled".to_string()
                } else {
                    format!("exit {}", status.code().unwrap_or(-1))
                },
            })
            .await;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test(flavor = "current_thread")]
    async fn runs_echo_and_completes() {
        let (tx, mut rx) = mpsc::channel(32);
        let cmd = PlainCommand {
            id: "t1".into(),
            label: "echo".into(),
            program: "echo".into(),
            args: vec!["hello".into()],
            cancel: None,
            policy: None,
        };
        let runner = tokio::spawn(run(cmd, tx));
        let mut saw_start = false;
        let mut saw_stdout = false;
        let mut saw_complete = false;
        while let Ok(Some(evt)) = tokio::time::timeout(Duration::from_secs(5), rx.recv()).await {
            match evt {
                ToolEvent::Start { .. } => saw_start = true,
                ToolEvent::StdoutChunk { chunk, .. } if chunk.contains("hello") => {
                    saw_stdout = true;
                }
                ToolEvent::Complete { .. } => {
                    saw_complete = true;
                    break;
                }
                _ => {}
            }
        }
        let _ = runner.await;
        assert!(saw_start);
        assert!(saw_stdout);
        assert!(saw_complete);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn nonzero_exit_emits_fail() {
        let (tx, mut rx) = mpsc::channel(32);
        let cmd = PlainCommand {
            id: "t2".into(),
            label: "false".into(),
            program: "false".into(),
            args: vec![],
            cancel: None,
            policy: None,
        };
        let runner = tokio::spawn(run(cmd, tx));
        let mut saw_fail = false;
        while let Ok(Some(evt)) = tokio::time::timeout(Duration::from_secs(5), rx.recv()).await {
            if matches!(evt, ToolEvent::Fail { .. }) {
                saw_fail = true;
                break;
            }
        }
        let _ = runner.await;
        assert!(saw_fail);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn cancellation_kills_long_running() {
        let token = CancellationToken::new();
        let (tx, mut rx) = mpsc::channel(32);
        let cmd = PlainCommand {
            id: "t3".into(),
            label: "sleep".into(),
            program: "sleep".into(),
            args: vec!["30".into()],
            cancel: Some(token.clone()),
            policy: None,
        };
        let runner = tokio::spawn(run(cmd, tx));

        // Give the child a moment to start, then raise to Hard.
        tokio::time::sleep(Duration::from_millis(100)).await;
        token.cancel_hard();

        let mut saw_fail = false;
        while let Ok(Some(evt)) = tokio::time::timeout(Duration::from_secs(5), rx.recv()).await {
            if matches!(evt, ToolEvent::Fail { .. }) {
                saw_fail = true;
                break;
            }
        }
        let _ = tokio::time::timeout(Duration::from_secs(5), runner).await;
        assert!(saw_fail, "expected Fail after cancellation");
    }

    /// Integration: with an allowlist policy, the child's `env` only shows
    /// the named vars (plus whatever the OS adds for fork/exec on the
    /// platform, which is none on Unix for `/usr/bin/env`).
    #[tokio::test(flavor = "current_thread")]
    async fn policy_allowlist_filters_env_in_child() {
        use crate::engine::sandbox_policy::{SandboxEnv, SandboxPolicy};

        // Tempdir so we don't depend on whatever cwd `cargo test` ran in.
        let tmp = tempfile::tempdir().expect("tempdir");
        std::env::set_var("JEKKO_PLAIN_ALLOWED_VAR", "yes");
        std::env::set_var("JEKKO_PLAIN_BLOCKED_VAR", "nope");

        let policy = SandboxPolicy {
            cwd: Some(tmp.path().to_path_buf()),
            allowed_paths: vec![],
            env: SandboxEnv::Allowlist(vec!["JEKKO_PLAIN_ALLOWED_VAR".to_string()]),
            allow_net: false,
        };

        let (tx, mut rx) = mpsc::channel(32);
        let cmd = PlainCommand {
            id: "policy-env".into(),
            label: "env".into(),
            program: "/usr/bin/env".into(),
            args: vec![],
            cancel: None,
            policy: Some(policy),
        };
        let runner = tokio::spawn(run(cmd, tx));

        let mut collected = String::new();
        let mut saw_terminal = false;
        while let Ok(Some(evt)) = tokio::time::timeout(Duration::from_secs(5), rx.recv()).await {
            match evt {
                ToolEvent::StdoutChunk { chunk, .. } => {
                    collected.push_str(&chunk);
                    collected.push('\n');
                }
                ToolEvent::Complete { .. } | ToolEvent::Fail { .. } => {
                    saw_terminal = true;
                    break;
                }
                _ => {}
            }
        }
        let _ = runner.await;
        std::env::remove_var("JEKKO_PLAIN_ALLOWED_VAR");
        std::env::remove_var("JEKKO_PLAIN_BLOCKED_VAR");

        assert!(saw_terminal, "runner did not terminate");
        assert!(
            collected.contains("JEKKO_PLAIN_ALLOWED_VAR=yes"),
            "expected allowlisted var in child env; got:\n{collected}"
        );
        assert!(
            !collected.contains("JEKKO_PLAIN_BLOCKED_VAR"),
            "blocked env leaked to child:\n{collected}"
        );
    }
}
