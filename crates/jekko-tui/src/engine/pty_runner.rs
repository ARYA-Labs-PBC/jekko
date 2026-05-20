//! PTY-backed process runner (COWBOY.md F2).
//!
//! For commands that need a real terminal: TTY detection (`isatty`), color
//! output gated by `clicolor`, progress bars, curses-
//! based tools. Uses `portable-pty` to allocate a pseudo-tty, runs the child
//! inside it, and streams the raw byte output. The `engine::ansi` parser
//! converts the bytes to styled spans before they hit the transcript.

use std::io::Read;
use std::time::Duration;

use anyhow::{Context, Result};
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use tokio::sync::mpsc;

use crate::action::ToolEvent;
use crate::engine::ansi;
use crate::engine::cancel::{CancelLevel, CancellationToken};
use crate::engine::sandbox_macos;
use crate::engine::sandbox_policy::SandboxPolicy;

#[derive(Clone, Debug)]
pub struct PtyCommand {
    pub id: String,
    pub label: String,
    pub program: String,
    pub args: Vec<String>,
    pub cols: u16,
    pub rows: u16,
    /// Optional shared cancellation handle. When the token reaches `Hard` or
    /// `Force`, the runner uses `ChildKiller::kill()` on the child.
    pub cancel: Option<CancellationToken>,
    /// Optional sandbox policy. `None` preserves legacy unrestricted PTY
    /// behavior. When `Some`, the policy is applied to the underlying
    /// `portable_pty::CommandBuilder` before `spawn_command`. See
    /// `engine::sandbox_policy` for the enforcement scope.
    pub policy: Option<SandboxPolicy>,
}

impl PtyCommand {
    pub fn new(
        id: impl Into<String>,
        label: impl Into<String>,
        program: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            program: program.into(),
            args: Vec::new(),
            cols: 120,
            rows: 40,
            cancel: None,
            policy: None,
        }
    }

    pub fn with_args(mut self, args: Vec<String>) -> Self {
        self.args = args;
        self
    }

    pub fn with_size(mut self, cols: u16, rows: u16) -> Self {
        self.cols = cols;
        self.rows = rows;
        self
    }

    pub fn with_cancel(mut self, cancel: CancellationToken) -> Self {
        self.cancel = Some(cancel);
        self
    }

    pub fn with_policy(mut self, policy: SandboxPolicy) -> Self {
        self.policy = Some(policy);
        self
    }
}

/// Run `cmd` inside a PTY, streaming output through `tx`. Returns once the
/// child exits. PTY reads happen on a `spawn_blocking` task (portable-pty's
/// reader is sync); the channel is async.
pub async fn run(cmd: PtyCommand, tx: mpsc::Sender<ToolEvent>) -> Result<()> {
    let _ = tx
        .send(ToolEvent::Start {
            id: cmd.id.clone(),
            name: cmd.label.clone(),
            input: Some(format!("{} {}", cmd.program, cmd.args.join(" "))),
        })
        .await;

    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize {
            rows: cmd.rows,
            cols: cmd.cols,
            pixel_width: 0,
            pixel_height: 0,
        })
        .with_context(|| "openpty")?;

    // T-SANDBOX-FS-ISOLATION-MACOS: same wrap as plain_runner — on macOS
    // with a fs-scoped policy this rewrites to
    // the macOS sandbox wrapper. Off macOS
    // or with no policy / no fs scope, returns the original tuple and the
    // PTY spawns the program directly as before.
    let (effective_program, effective_args) = match cmd.policy.as_ref() {
        Some(policy) => sandbox_macos::wrap_command(&cmd.program, &cmd.args, policy),
        None => (cmd.program.clone(), cmd.args.clone()),
    };

    let mut builder = CommandBuilder::new(&effective_program);
    for a in &effective_args {
        builder.arg(a);
    }
    if let Some(policy) = cmd.policy.as_ref() {
        policy.apply_to_pty_builder(&mut builder);
    }

    // T-SANDBOX-FS-ISOLATION-LINUX: PTY landlock isolation is deferred.
    // `portable-pty::CommandBuilder` does not expose a `pre_exec` hook
    // (the closure would have to be wired into the unix-specific
    // implementation of `spawn_command`, which lives behind the
    // builder's facade). Tracked as T-SANDBOX-PTY-LANDLOCK. The
    // non-PTY codepath (`plain_runner::run`) does apply landlock; PTY
    // runs on Linux fall back to advisory cwd/env scrubbing only.

    let pair = pair;
    let master = pair.master;
    let slave = pair.slave;

    let mut child = slave
        .spawn_command(builder)
        .with_context(|| format!("spawn {}", cmd.program))?;
    drop(slave);

    // Clone a separate killer so the cancellation poll task can signal the
    // child while another `spawn_blocking` task is parked inside `wait()`.
    let mut killer = child.clone_killer();

    let mut reader = master
        .try_clone_reader()
        .with_context(|| "clone pty reader")?;

    let id_read = cmd.id.clone();
    let tx_read = tx.clone();
    let reader_task = tokio::task::spawn_blocking(move || {
        let mut buf = [0u8; 8192];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    let spans = ansi::parse_bytes(&buf[..n]);
                    let chunk: String = spans
                        .iter()
                        .map(|s| s.content.as_ref())
                        .collect::<Vec<_>>()
                        .join("");
                    if tx_read
                        .blocking_send(ToolEvent::StdoutChunk {
                            id: id_read.clone(),
                            chunk,
                        })
                        .is_err()
                    {
                        return;
                    }
                }
                Err(_) => break,
            }
        }
    });

    // Cancellation poll: 50ms tick, signals via the cloned killer when the
    // token reaches Hard/Force. Exits when `wait()` completes (cancel_rx).
    let cancelled_flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let cancel_poll_task = if let Some(token) = cmd.cancel.clone() {
        let (stop_tx, mut stop_rx) = mpsc::channel::<()>(1);
        let cancelled_set = cancelled_flag.clone();
        let task = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = stop_rx.recv() => return,
                    _ = tokio::time::sleep(Duration::from_millis(50)) => {
                        if matches!(token.level(), CancelLevel::Hard | CancelLevel::Force) {
                            let _ = killer.kill();
                            cancelled_set.store(true, std::sync::atomic::Ordering::SeqCst);
                            return;
                        }
                    }
                }
            }
        });
        Some((task, stop_tx))
    } else {
        None
    };

    // portable-pty's `wait()` is sync, so we run it on spawn_blocking.
    let exit_task = tokio::task::spawn_blocking(move || child.wait().ok());
    let exit_status = exit_task.await.ok().flatten();
    drop(master); // close PTY → reader EOF → reader_task exits

    if let Some((task, stop_tx)) = cancel_poll_task {
        let _ = stop_tx.send(()).await;
        let _ = task.await;
    }
    let _ = reader_task.await;

    let cancelled = cancelled_flag.load(std::sync::atomic::Ordering::SeqCst);
    let success = exit_status.as_ref().map(|s| s.success()).unwrap_or(false);
    if success && !cancelled {
        let _ = tx.send(ToolEvent::Complete { id: cmd.id }).await;
    } else if cancelled {
        let _ = tx
            .send(ToolEvent::Fail {
                id: cmd.id,
                error: "cancelled".to_string(),
            })
            .await;
    } else {
        let code = exit_status
            .as_ref()
            .map(|s| s.exit_code() as i64)
            .unwrap_or(-1);
        let _ = tx
            .send(ToolEvent::Fail {
                id: cmd.id,
                error: format!("exit {code}"),
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
    async fn runs_echo_under_pty() {
        let (tx, mut rx) = mpsc::channel(32);
        let cmd = PtyCommand::new("p1", "echo", "echo").with_args(vec!["hello-pty".into()]);
        let runner = tokio::spawn(run(cmd, tx));
        let mut saw_start = false;
        let mut saw_chunk_with_hello = false;
        let mut saw_complete = false;
        while let Ok(Some(evt)) = tokio::time::timeout(Duration::from_secs(10), rx.recv()).await {
            match evt {
                ToolEvent::Start { .. } => saw_start = true,
                ToolEvent::StdoutChunk { chunk, .. } if chunk.contains("hello-pty") => {
                    saw_chunk_with_hello = true;
                }
                ToolEvent::Complete { .. } => {
                    saw_complete = true;
                    break;
                }
                _ => {}
            }
        }
        let _ = runner.await;
        assert!(saw_start, "missing Start");
        assert!(saw_chunk_with_hello, "missing 'hello-pty' chunk");
        assert!(saw_complete, "missing Complete");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn nonzero_exit_yields_fail() {
        let (tx, mut rx) = mpsc::channel(32);
        let cmd = PtyCommand::new("p2", "false", "false");
        let runner = tokio::spawn(run(cmd, tx));
        let mut saw_fail = false;
        while let Ok(Some(evt)) = tokio::time::timeout(Duration::from_secs(10), rx.recv()).await {
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
        let cmd = PtyCommand::new("p3", "sleep", "sleep")
            .with_args(vec!["30".into()])
            .with_cancel(token.clone());
        let runner = tokio::spawn(run(cmd, tx));

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
}
