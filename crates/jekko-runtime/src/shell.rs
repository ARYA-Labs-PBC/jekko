//! Shell process spawning.
//!
//! Ported from `packages/jekko/src/shell/shell.ts`. Wraps `tokio::process`
//! with a tiny "run, capture, return" helper plus kill-tree handling
//! for cases where the spawned child has launched its own subprocesses.

use std::path::Path;
use std::process::Stdio;

use serde::{Deserialize, Serialize};
use tokio::process::Command;

use crate::error::{RuntimeError, RuntimeResult};

/// Error message when child stdin pipe is already closed before the parent writes.
const STDIN_CLOSED_MSG: &str = "child stdin closed before write";

/// Result of running a one-shot command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShellOutput {
    /// Exit code (0 on success). `None` if the process was killed by a signal.
    pub code: Option<i32>,
    /// Captured stdout.
    pub stdout: String,
    /// Captured stderr.
    pub stderr: String,
}

/// Spawn `sh -c <command>` in `cwd` and capture stdout / stderr / exit code.
///
/// Note: this is a captured spawn, **not** a PTY. For terminal-style
/// processes use [`crate::pty`].
pub async fn run(command: &str, cwd: impl AsRef<Path>) -> RuntimeResult<ShellOutput> {
    let cwd = cwd.as_ref();
    let out = Command::new("sh")
        .arg("-c")
        .arg(command)
        .current_dir(cwd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await?;
    Ok(ShellOutput {
        code: out.status.code(),
        stdout: String::from_utf8_lossy(&out.stdout).to_string(),
        stderr: String::from_utf8_lossy(&out.stderr).to_string(),
    })
}

/// Spawn `sh -c <command>` and pipe stdin. Returns stdout/stderr/code.
pub async fn run_with_stdin(
    command: &str,
    cwd: impl AsRef<Path>,
    stdin: &[u8],
) -> RuntimeResult<ShellOutput> {
    let cwd = cwd.as_ref();
    let mut child = Command::new("sh")
        .arg("-c")
        .arg(command)
        .current_dir(cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    {
        let mut h = match child.stdin.take() {
            Some(h) => h,
            None => return Err(RuntimeError::other(STDIN_CLOSED_MSG.to_string())),
        };
        use tokio::io::AsyncWriteExt;
        h.write_all(stdin).await?;
    }
    let out = child.wait_with_output().await?;
    Ok(ShellOutput {
        code: out.status.code(),
        stdout: String::from_utf8_lossy(&out.stdout).to_string(),
        stderr: String::from_utf8_lossy(&out.stderr).to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn echo_hello() {
        let out = run("printf hello", ".").await.unwrap();
        assert_eq!(out.stdout, "hello");
        assert_eq!(out.code, Some(0));
    }

    #[tokio::test]
    async fn stdin_pipe() {
        let out = run_with_stdin("cat", ".", b"hi").await.unwrap();
        assert_eq!(out.stdout, "hi");
    }

    #[tokio::test]
    async fn nonzero_exit() {
        let out = run("exit 7", ".").await.unwrap();
        assert_eq!(out.code, Some(7));
    }
}
