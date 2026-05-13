//! NDJSON event sink. Append-only line stream at
//! `agent/zyal/runner-events.jsonl`. Each line ≤ 512 bytes so the daemon-side
//! tailer (PR4) can budget its read window. The schema is deliberately flat:
//! every event carries `ts` + `kind` + `run_id`, plus a free-form `data`
//! object for kind-specific fields.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const EVENT_FILE_REL: &str = "agent/zyal/runner-events.jsonl";
pub const MAX_LINE_BYTES: usize = 512;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    RunStarted,
    WorkerStarted,
    WorkerPass,
    WorkerFail,
    CommitLanded,
    RebaseConflict,
    WorkerRollback,
    GcPruned,
    RunFinished,
    BootstrapRequired,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub ts: u64,
    pub kind: EventKind,
    pub run_id: String,
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub data: Value,
}

pub struct EventSink {
    path: PathBuf,
    run_id: String,
}

impl EventSink {
    pub fn open(repo_root: &Path, run_id: &str) -> Result<Self> {
        let path = repo_root.join(EVENT_FILE_REL);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("mkdir -p {}", parent.display()))?;
        }
        Ok(Self {
            path,
            run_id: run_id.to_string(),
        })
    }

    pub fn emit(&self, kind: EventKind, data: Value) -> Result<()> {
        let event = Event {
            ts: now_epoch_secs(),
            kind,
            run_id: self.run_id.clone(),
            data,
        };
        let line = serde_json::to_string(&event).context("serialize event")?;
        if line.len() > MAX_LINE_BYTES {
            return Err(anyhow::anyhow!(
                "runner event exceeds {} bytes ({} bytes): {}",
                MAX_LINE_BYTES,
                line.len(),
                truncate_for_error(&line),
            ));
        }
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .with_context(|| format!("open {}", self.path.display()))?;
        file.write_all(line.as_bytes())?;
        file.write_all(b"\n")?;
        Ok(())
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

fn now_epoch_secs() -> u64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(d) => d.as_secs(),
        Err(_) => 0,
    }
}

fn truncate_for_error(line: &str) -> &str {
    let cap = line.char_indices().nth(120).map(|(i, _)| i).unwrap_or(line.len());
    &line[..cap]
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::tempdir;

    #[test]
    fn emits_one_line_per_event() {
        let dir = tempdir().unwrap();
        let sink = EventSink::open(dir.path(), "run-1").unwrap();
        sink.emit(EventKind::RunStarted, json!({"pool_size": 4})).unwrap();
        sink.emit(EventKind::WorkerStarted, json!({"worker": "w-01"})).unwrap();
        let text = fs::read_to_string(sink.path()).unwrap();
        assert_eq!(text.lines().count(), 2);
        assert!(text.contains("run_started"));
        assert!(text.contains("worker_started"));
    }

    #[test]
    fn rejects_lines_over_512_bytes() {
        let dir = tempdir().unwrap();
        let sink = EventSink::open(dir.path(), "run-1").unwrap();
        let huge = "x".repeat(600);
        let err = sink.emit(EventKind::WorkerPass, json!({"blob": huge})).unwrap_err();
        assert!(err.to_string().contains("exceeds"));
        // and nothing was written
        assert!(!sink.path().exists() || fs::read_to_string(sink.path()).unwrap().is_empty());
    }

    #[test]
    fn lines_are_parseable_back_into_event() {
        let dir = tempdir().unwrap();
        let sink = EventSink::open(dir.path(), "run-1").unwrap();
        sink.emit(EventKind::CommitLanded, json!({"sha": "abc123"})).unwrap();
        let text = fs::read_to_string(sink.path()).unwrap();
        let event: Event = serde_json::from_str(text.trim()).unwrap();
        assert_eq!(event.kind, EventKind::CommitLanded);
        assert_eq!(event.run_id, "run-1");
        assert_eq!(event.data["sha"], "abc123");
    }
}
