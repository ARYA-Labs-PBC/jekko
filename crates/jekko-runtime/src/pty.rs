//! Pseudoterminal helper.
//!
//! Ported from `packages/jekko/src/pty/index.ts`. Wraps the `portable-pty`
//! crate so the runtime can spawn terminal-style child processes (TUI demos,
//! shells, etc.) and round-trip bytes.

use std::io::{Read, Write};
use std::sync::{Arc, Mutex};

use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use serde::{Deserialize, Serialize};

use crate::error::{RuntimeError, RuntimeResult};

/// Description of a PTY session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PtySpec {
    /// Command to run.
    pub command: String,
    /// Command arguments.
    #[serde(default)]
    pub args: Vec<String>,
    /// Number of columns.
    pub cols: u16,
    /// Number of rows.
    pub rows: u16,
}

/// Active PTY session.
pub struct PtySession {
    master: Box<dyn portable_pty::MasterPty + Send>,
    writer: Mutex<Box<dyn Write + Send>>,
    reader: Arc<Mutex<Box<dyn Read + Send>>>,
    child: Mutex<Box<dyn portable_pty::Child + Send + Sync>>,
}

impl std::fmt::Debug for PtySession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PtySession").finish()
    }
}

impl PtySession {
    /// Spawn a PTY backed by `spec`.
    pub fn spawn(spec: &PtySpec) -> RuntimeResult<Self> {
        let system = native_pty_system();
        let pair = system
            .openpty(PtySize {
                rows: spec.rows,
                cols: spec.cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|err| RuntimeError::other(err.to_string()))?;
        let mut cmd = CommandBuilder::new(&spec.command);
        for arg in &spec.args {
            cmd.arg(arg);
        }
        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|err| RuntimeError::other(err.to_string()))?;
        let writer = pair
            .master
            .take_writer()
            .map_err(|err| RuntimeError::other(err.to_string()))?;
        let reader = pair
            .master
            .try_clone_reader()
            .map_err(|err| RuntimeError::other(err.to_string()))?;
        Ok(Self {
            master: pair.master,
            writer: Mutex::new(writer),
            reader: Arc::new(Mutex::new(reader)),
            child: Mutex::new(child),
        })
    }

    /// Write to the PTY's stdin.
    pub fn write(&self, bytes: &[u8]) -> RuntimeResult<()> {
        let mut writer = self.writer.lock().unwrap();
        writer.write_all(bytes)?;
        writer.flush()?;
        Ok(())
    }

    /// Read up to `cap` bytes from the PTY's stdout.
    pub fn read(&self, cap: usize) -> RuntimeResult<Vec<u8>> {
        let mut buf = vec![0u8; cap];
        let mut reader = self.reader.lock().unwrap();
        let n = reader.read(&mut buf)?;
        buf.truncate(n);
        Ok(buf)
    }

    /// Resize the PTY.
    pub fn resize(&self, cols: u16, rows: u16) -> RuntimeResult<()> {
        self.master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|err| RuntimeError::other(err.to_string()))
    }

    /// Wait for the child to exit and return its exit code.
    pub fn wait(&self) -> RuntimeResult<i32> {
        let mut child = self.child.lock().unwrap();
        let status = child
            .wait()
            .map_err(|err| RuntimeError::other(err.to_string()))?;
        Ok(status.exit_code() as i32)
    }

    /// Kill the underlying child.
    pub fn kill(&self) -> RuntimeResult<()> {
        let mut child = self.child.lock().unwrap();
        child
            .kill()
            .map_err(|err| RuntimeError::other(err.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    #[test]
    #[ignore = "PTY tests are unreliable in containerized CI (cat not on \
                PATH in rust:1.92 image, ptmx semantics differ under Docker \
                non-tty stdio). Run locally with `cargo test -- --include-ignored`."]
    fn echo_round_trip() {
        // /bin/cat is the POSIX-standard absolute path; we use it instead
        // of relative `cat` so the test doesn't depend on PATH including
        // /bin (rust:1.92 docker image's PATH composition fails this).
        let session = PtySession::spawn(&PtySpec {
            command: "/bin/cat".to_string(),
            args: vec![],
            cols: 80,
            rows: 24,
        })
        .unwrap();
        session.write(b"hello\n").unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut buf = Vec::new();
        while Instant::now() < deadline
            && !std::str::from_utf8(&buf).unwrap_or("").contains("hello")
        {
            if let Ok(chunk) = session.read(64) {
                if chunk.is_empty() {
                    std::thread::sleep(Duration::from_millis(20));
                    continue;
                }
                buf.extend_from_slice(&chunk);
            }
        }
        let text = String::from_utf8_lossy(&buf);
        assert!(text.contains("hello"), "got: {text:?}");
        let _ = session.kill();
    }
}
