//! Tool-output collapse + raw-log persistence (COWBOY.md F4).
//!
//! Tool stdout/stderr can be massive. Two storage tiers:
//!
//! 1. **In-memory `OutputBuffer`** — every line received, lossless. Provides
//!    a `visible_view()` that returns at most ~80 lines (first 3 + last 3 +
//!    one `… +N lines (ctrl+o to expand)` marker) for the transcript card.
//!    When `expanded` is set, returns everything.
//!
//! 2. **On-disk raw log** — `~/.local/state/jekko/runs/<id>.log` (or
//!    `$XDG_STATE_HOME/jekko/runs/...`). Written incrementally on every chunk
//!    so even a crashed jekko leaves the full output recoverable. `Ctrl+T`
//!    wiring lives in the chat_runtime; this module just exposes `raw_path()`.

use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::PathBuf;

use anyhow::{Context, Result};

const DEFAULT_HEAD: usize = 3;
const DEFAULT_TAIL: usize = 3;
const COLLAPSE_THRESHOLD: usize = 80;

#[derive(Debug)]
pub struct OutputBuffer {
    id: String,
    head: usize,
    tail: usize,
    threshold: usize,
    lines: Vec<String>,
    expanded: bool,
    raw_path: Option<PathBuf>,
    raw_writer: Option<BufWriter<File>>,
}

impl OutputBuffer {
    /// Build a collapsing buffer. Raw log path is set lazily on the first
    /// chunk if we can determine a writable directory.
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            head: DEFAULT_HEAD,
            tail: DEFAULT_TAIL,
            threshold: COLLAPSE_THRESHOLD,
            lines: Vec::new(),
            expanded: false,
            raw_path: None,
            raw_writer: None,
        }
    }

    /// Override the head/tail counts and the line threshold above which
    /// collapse kicks in.
    pub fn with_thresholds(mut self, head: usize, tail: usize, threshold: usize) -> Self {
        self.head = head;
        self.tail = tail;
        self.threshold = threshold;
        self
    }

    /// Append a line. Persists to raw log on first call.
    pub fn push_line(&mut self, line: impl Into<String>) -> Result<()> {
        let line = line.into();
        self.ensure_raw_log_open()?;
        if let Some(w) = self.raw_writer.as_mut() {
            writeln!(w, "{line}").context("write raw log")?;
        }
        self.lines.push(line);
        Ok(())
    }

    /// Full raw line slice. Used by the pager when reading a completed tool's
    /// persistent buffer from the transcript sidecar.
    pub fn all_lines(&self) -> &[String] {
        &self.lines
    }

    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    pub fn raw_path(&self) -> Option<&PathBuf> {
        self.raw_path.as_ref()
    }

    pub fn set_expanded(&mut self, expanded: bool) {
        self.expanded = expanded;
    }

    pub fn expanded(&self) -> bool {
        self.expanded
    }

    /// Returns the rows the transcript card should render. `Collapsed::Full`
    /// means render every line; `Collapsed::Folded { head, hidden, tail }`
    /// means render `head` rows, one `… +<hidden> lines (ctrl+o to expand)`
    /// marker, then `tail` rows.
    pub fn visible_view(&self) -> Collapsed<'_> {
        let total = self.lines.len();
        if self.expanded || total <= self.threshold {
            return Collapsed::Full(&self.lines);
        }
        let head_slice = &self.lines[..self.head.min(total)];
        let tail_start = total.saturating_sub(self.tail);
        let tail_slice = &self.lines[tail_start..];
        let hidden = total.saturating_sub(self.head + self.tail);
        Collapsed::Folded {
            head: head_slice,
            hidden,
            tail: tail_slice,
        }
    }

    /// Flush the raw log writer. Call on shutdown or when the buffer is
    /// dropped from the transcript.
    pub fn flush(&mut self) -> Result<()> {
        if let Some(w) = self.raw_writer.as_mut() {
            w.flush().context("flush raw log")?;
        }
        Ok(())
    }

    fn ensure_raw_log_open(&mut self) -> Result<()> {
        if self.raw_writer.is_some() {
            return Ok(());
        }
        let dir = raw_log_dir().context("no raw log dir")?;
        std::fs::create_dir_all(&dir).with_context(|| format!("mkdir {}", dir.display()))?;
        let path = dir.join(format!("{}.log", self.id));
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .with_context(|| format!("open {}", path.display()))?;
        self.raw_path = Some(path);
        self.raw_writer = Some(BufWriter::new(file));
        Ok(())
    }
}

#[derive(Debug)]
pub enum Collapsed<'a> {
    Full(&'a [String]),
    Folded {
        head: &'a [String],
        hidden: usize,
        tail: &'a [String],
    },
}

impl<'a> Collapsed<'a> {
    /// Total number of rows the renderer should emit (head + marker + tail,
    /// or every line when full).
    pub fn row_count(&self) -> usize {
        match self {
            Collapsed::Full(lines) => lines.len(),
            Collapsed::Folded { head, tail, .. } => head.len() + 1 + tail.len(),
        }
    }
}

fn raw_log_dir() -> Option<PathBuf> {
    if let Some(xdg) = std::env::var_os("XDG_STATE_HOME") {
        return Some(PathBuf::from(xdg).join("jekko").join("runs"));
    }
    let home = std::env::var_os("HOME")?;
    Some(
        PathBuf::from(home)
            .join(".local")
            .join("state")
            .join("jekko")
            .join("runs"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use tempfile::TempDir;

    static ENV_GUARD: Mutex<()> = Mutex::new(());

    fn with_xdg_state_home<T>(dir: &TempDir, f: impl FnOnce() -> T) -> T {
        let _guard = ENV_GUARD.lock().unwrap();
        let previous = std::env::var_os("XDG_STATE_HOME");
        std::env::set_var("XDG_STATE_HOME", dir.path());
        let result = f();
        match previous {
            Some(value) => std::env::set_var("XDG_STATE_HOME", value),
            None => std::env::remove_var("XDG_STATE_HOME"),
        }
        result
    }

    fn buf_with_temp(id: &str) -> OutputBuffer {
        OutputBuffer::new(id)
    }

    #[test]
    fn short_output_renders_full() {
        let dir = TempDir::new().unwrap();
        with_xdg_state_home(&dir, || {
            let mut b = buf_with_temp("t1");
            for i in 0..5 {
                b.push_line(format!("line {i}")).unwrap();
            }
            match b.visible_view() {
                Collapsed::Full(lines) => assert_eq!(lines.len(), 5),
                _ => panic!("expected Full"),
            }
        });
    }

    #[test]
    fn long_output_collapses_to_head_marker_tail() {
        let dir = TempDir::new().unwrap();
        with_xdg_state_home(&dir, || {
            let mut b = buf_with_temp("t2");
            for i in 0..100 {
                b.push_line(format!("line {i}")).unwrap();
            }
            match b.visible_view() {
                Collapsed::Folded { head, hidden, tail } => {
                    assert_eq!(head.len(), 3);
                    assert_eq!(tail.len(), 3);
                    assert_eq!(hidden, 94);
                }
                _ => panic!("expected Folded"),
            }
        });
    }

    #[test]
    fn expanded_overrides_collapse() {
        let dir = TempDir::new().unwrap();
        with_xdg_state_home(&dir, || {
            let mut b = buf_with_temp("t3");
            for i in 0..200 {
                b.push_line(format!("line {i}")).unwrap();
            }
            b.set_expanded(true);
            match b.visible_view() {
                Collapsed::Full(lines) => assert_eq!(lines.len(), 200),
                _ => panic!("expected Full when expanded"),
            }
        });
    }

    #[test]
    fn raw_log_persists_full_output() {
        let dir = TempDir::new().unwrap();
        with_xdg_state_home(&dir, || {
            let mut b = buf_with_temp("persist");
            for i in 0..150 {
                b.push_line(format!("L{i}")).unwrap();
            }
            b.flush().unwrap();
            let path = b.raw_path().unwrap().clone();
            let written = std::fs::read_to_string(&path).unwrap();
            let n = written.lines().count();
            assert_eq!(n, 150);
            assert!(written.contains("L0"));
            assert!(written.contains("L149"));
        });
    }

    #[test]
    fn custom_thresholds_respected() {
        let dir = TempDir::new().unwrap();
        with_xdg_state_home(&dir, || {
            let mut b = buf_with_temp("custom").with_thresholds(1, 1, 10);
            for i in 0..30 {
                b.push_line(format!("line {i}")).unwrap();
            }
            match b.visible_view() {
                Collapsed::Folded { head, hidden, tail } => {
                    assert_eq!(head.len(), 1);
                    assert_eq!(tail.len(), 1);
                    assert_eq!(hidden, 28);
                }
                _ => panic!("expected Folded with custom threshold"),
            }
        });
    }

    #[test]
    fn row_count_matches_render_shape() {
        let dir = TempDir::new().unwrap();
        with_xdg_state_home(&dir, || {
            let mut b = buf_with_temp("rows");
            for i in 0..200 {
                b.push_line(format!("line {i}")).unwrap();
            }
            let view = b.visible_view();
            // head + marker + tail = 3 + 1 + 3 = 7
            assert_eq!(view.row_count(), 7);
        });
    }
}
