//! Filesystem watchers.
//!
//! Ported from `packages/jekko/src/util/watcher.ts`. Wraps [`notify`] in
//! an async-friendly shell that fans events into a Tokio mpsc channel.

use std::path::{Path, PathBuf};

use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::error::{RuntimeError, RuntimeResult};

/// One filesystem change event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangeEvent {
    /// Event kind tag.
    pub kind: ChangeKind,
    /// Affected paths.
    pub paths: Vec<PathBuf>,
}

/// Coarse classification of a filesystem change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChangeKind {
    /// File / dir created.
    Create,
    /// File / dir modified.
    Modify,
    /// File / dir removed.
    Remove,
    /// Other (rename, metadata, …).
    Other,
}

/// Live watcher handle. Drop to stop watching.
pub struct WatcherHandle {
    _watcher: RecommendedWatcher,
}

impl std::fmt::Debug for WatcherHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WatcherHandle").finish()
    }
}

/// Start watching `path` recursively, returning a handle and an mpsc
/// receiver that yields [`ChangeEvent`]s.
pub fn watch(
    path: impl AsRef<Path>,
) -> RuntimeResult<(WatcherHandle, mpsc::Receiver<ChangeEvent>)> {
    let (tx, rx) = mpsc::channel(256);
    let mut watcher: RecommendedWatcher = notify::recommended_watcher(move |res| match res {
        Ok(event) => {
            let _ = tx.try_send(translate(&event));
        }
        Err(_) => {
            // Drop the error silently; the receiver will keep waiting.
        }
    })
    .map_err(|err| RuntimeError::other(err.to_string()))?;
    watcher
        .watch(path.as_ref(), RecursiveMode::Recursive)
        .map_err(|err| RuntimeError::other(err.to_string()))?;
    Ok((WatcherHandle { _watcher: watcher }, rx))
}

fn translate(event: &Event) -> ChangeEvent {
    use notify::EventKind;
    let kind = match event.kind {
        EventKind::Create(_) => ChangeKind::Create,
        EventKind::Modify(_) => ChangeKind::Modify,
        EventKind::Remove(_) => ChangeKind::Remove,
        _ => ChangeKind::Other,
    };
    ChangeEvent {
        kind,
        paths: event.paths.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tempfile::tempdir;

    #[tokio::test(flavor = "multi_thread")]
    async fn detects_writes() {
        let dir = tempdir().unwrap();
        let (handle, mut rx) = watch(dir.path()).unwrap();
        let path = dir.path().join("x.txt");
        tokio::time::sleep(Duration::from_millis(50)).await;
        std::fs::write(&path, b"hi").unwrap();
        let evt = tokio::time::timeout(Duration::from_secs(5), rx.recv())
            .await
            .expect("timeout")
            .expect("channel closed");
        assert!(matches!(
            evt.kind,
            ChangeKind::Create | ChangeKind::Modify | ChangeKind::Other
        ));
        drop(handle);
    }
}
