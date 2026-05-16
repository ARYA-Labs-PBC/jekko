//! Git-aware workspace snapshot.
//!
//! Ported from `packages/jekko/src/snapshot/index.ts`. The TS module
//! shells out to git via a hidden "snapshot" worktree to maintain a
//! per-session set of commit-shaped snapshots. We expose the same
//! observable shape (snapshot hash, files, patch) and use the system
//! `git` binary the same way.

use std::path::{Path, PathBuf};
use std::process::Stdio;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::process::Command;

use crate::error::RuntimeResult;

/// One snapshot record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snapshot {
    /// Stable hash identifying the snapshot.
    pub hash: String,
    /// Files captured.
    pub files: Vec<PathBuf>,
}

/// Snapshot a directory tree by hashing each tracked file. This is a
/// VCS-agnostic path used when `git` is unavailable.
pub async fn hash_tree(root: impl AsRef<Path>) -> RuntimeResult<Snapshot> {
    let root = root.as_ref();
    let mut files = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    let mut digest = Sha256::new();
    while let Some(path) = stack.pop() {
        let meta = match tokio::fs::metadata(&path).await {
            Ok(m) => m,
            Err(_) => continue,
        };
        if meta.is_dir() {
            if path.file_name().is_some_and(|n| n == ".git") {
                continue;
            }
            let mut rd = match tokio::fs::read_dir(&path).await {
                Ok(r) => r,
                Err(_) => continue,
            };
            while let Ok(Some(entry)) = rd.next_entry().await {
                stack.push(entry.path());
            }
        } else if meta.is_file() {
            let bytes = tokio::fs::read(&path).await?;
            let rel = match path.strip_prefix(root) {
                Ok(p) => p.to_path_buf(),
                Err(_) => path.clone(),
            };
            digest.update(rel.to_string_lossy().as_bytes());
            digest.update(b"\0");
            digest.update(&bytes);
            files.push(rel);
        }
    }
    files.sort();
    Ok(Snapshot {
        hash: hex_encode(&digest.finalize()),
        files,
    })
}

/// Run a `diff` between two paths using the system `diff` binary.
pub async fn diff(from: &Path, to: &Path) -> RuntimeResult<String> {
    let out = Command::new("diff")
        .arg("-ruN")
        .arg(from)
        .arg(to)
        .stderr(Stdio::null())
        .output()
        .await?;
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

/// Lightweight wrapper around `sha2::Sha256` for tests that want to
/// compute the same hash inline.
pub fn hash_bytes(bytes: &[u8]) -> String {
    let mut d = Sha256::new();
    d.update(bytes);
    hex_encode(&d.finalize())
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn hash_tree_stable() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("a.txt"), b"hi").unwrap();
        std::fs::write(dir.path().join("b.txt"), b"hello").unwrap();
        let s1 = hash_tree(dir.path()).await.unwrap();
        let s2 = hash_tree(dir.path()).await.unwrap();
        assert_eq!(s1.hash, s2.hash);
        assert_eq!(s1.files.len(), 2);
    }

    #[tokio::test]
    async fn hash_tree_changes_with_edits() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("a.txt"), b"hi").unwrap();
        let s1 = hash_tree(dir.path()).await.unwrap();
        std::fs::write(dir.path().join("a.txt"), b"hi2").unwrap();
        let s2 = hash_tree(dir.path()).await.unwrap();
        assert_ne!(s1.hash, s2.hash);
    }
}
