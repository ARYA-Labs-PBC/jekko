//! Filesystem helpers (read / glob / list / ignore-aware walk).
//!
//! Ported from `packages/jekko/src/file/index.ts` and
//! `packages/jekko/src/file/ignore.ts`. Pure async I/O on top of Tokio.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tokio::fs;

use crate::error::{RuntimeError, RuntimeResult};

/// Maximum bytes to load through [`read_file`] before returning an error.
pub const DEFAULT_MAX_BYTES: u64 = 50 * 1024;

/// Read `path` into a `String`, returning an error if larger than `max`.
pub async fn read_file(path: impl AsRef<Path>, max: u64) -> RuntimeResult<String> {
    let path = path.as_ref();
    let meta = fs::metadata(path).await?;
    if meta.len() > max {
        return Err(RuntimeError::invalid(format!(
            "file {} is {} bytes (> {} max)",
            path.display(),
            meta.len(),
            max
        )));
    }
    let bytes = fs::read(path).await?;
    String::from_utf8(bytes).map_err(|err| RuntimeError::other(err.to_string()))
}

/// Write `contents` to `path` atomically (write to `<path>.tmp` then rename).
pub async fn write_file(path: impl AsRef<Path>, contents: impl AsRef<[u8]>) -> RuntimeResult<()> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await?;
    }
    let mut tmp = path.to_path_buf();
    // Explicit typed branching: empty extension means "no .tmp infix prefix".
    // The clippy lint below would collapse this into an implicit-default form
    // that hides the typed state.
    #[allow(clippy::manual_unwrap_or_default)]
    let prior_ext: &str = match path.extension().and_then(|e| e.to_str()) {
        Some(ext) => ext,
        None => "",
    };
    tmp.set_extension(format!("{}.tmp", prior_ext));
    fs::write(&tmp, contents).await?;
    fs::rename(&tmp, path).await?;
    Ok(())
}

/// List a directory (non-recursive), returning sorted entries.
pub async fn list_dir(path: impl AsRef<Path>) -> RuntimeResult<Vec<DirEntry>> {
    let mut rd = fs::read_dir(path.as_ref()).await?;
    let mut out = Vec::new();
    while let Some(entry) = rd.next_entry().await? {
        let meta = entry.metadata().await?;
        out.push(DirEntry {
            path: entry.path(),
            kind: if meta.is_dir() {
                EntryKind::Directory
            } else if meta.is_symlink() {
                EntryKind::Symlink
            } else {
                EntryKind::File
            },
            size: meta.len(),
        });
    }
    out.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(out)
}

/// Recursively glob `pattern` rooted at `base`, returning matching files.
///
/// Supports `*` (any chars within a segment), `**` (any number of segments),
/// and `?` (any single char). Matches the subset of `globby` semantics used
/// by `packages/jekko/src/file/index.ts`.
pub fn glob(base: impl AsRef<Path>, pattern: &str) -> RuntimeResult<Vec<PathBuf>> {
    let base = base.as_ref();
    let mut out = Vec::new();
    let segments: Vec<&str> = pattern.split('/').collect();
    walk_glob(base, &segments, 0, &mut out)?;
    out.sort();
    Ok(out)
}

fn walk_glob(
    here: &Path,
    segments: &[&str],
    idx: usize,
    out: &mut Vec<PathBuf>,
) -> RuntimeResult<()> {
    if idx >= segments.len() {
        if here.is_file() {
            out.push(here.to_path_buf());
        }
        return Ok(());
    }
    let segment = segments[idx];
    let is_last = idx == segments.len() - 1;

    if segment == "**" {
        // ** matches any number of path segments (including zero).
        walk_glob(here, segments, idx + 1, out)?;
        if here.is_dir() {
            for entry in std::fs::read_dir(here)? {
                let entry = entry?;
                if entry.file_type()?.is_dir() {
                    walk_glob(&entry.path(), segments, idx, out)?;
                }
            }
        }
        return Ok(());
    }

    if !here.is_dir() {
        return Ok(());
    }
    for entry in std::fs::read_dir(here)? {
        let entry = entry?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if !glob_segment(&name_str, segment) {
            continue;
        }
        let next = entry.path();
        if is_last {
            if next.is_file() {
                out.push(next);
            }
        } else if next.is_dir() {
            walk_glob(&next, segments, idx + 1, out)?;
        }
    }
    Ok(())
}

fn glob_segment(value: &str, pattern: &str) -> bool {
    let v = value.as_bytes();
    let p = pattern.as_bytes();
    let (mut i, mut j, mut star_i, mut star_j) = (0_usize, 0_usize, None, 0_usize);
    while i < v.len() {
        if j < p.len() && (p[j] == b'?' || p[j] == v[i]) {
            i += 1;
            j += 1;
        } else if j < p.len() && p[j] == b'*' {
            star_i = Some(i);
            star_j = j;
            j += 1;
        } else if let Some(si) = star_i {
            j = star_j + 1;
            star_i = Some(si + 1);
            i = si + 1;
        } else {
            return false;
        }
    }
    while j < p.len() && p[j] == b'*' {
        j += 1;
    }
    j == p.len()
}

/// One directory entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirEntry {
    /// Full path.
    pub path: PathBuf,
    /// Entry kind.
    pub kind: EntryKind,
    /// Size in bytes (0 for directories).
    pub size: u64,
}

/// Filesystem entry kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EntryKind {
    /// Regular file.
    File,
    /// Directory.
    Directory,
    /// Symlink.
    Symlink,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn read_and_write_round_trip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("hello.txt");
        write_file(&path, b"hi").await.unwrap();
        let text = read_file(&path, 1024).await.unwrap();
        assert_eq!(text, "hi");
    }

    #[tokio::test]
    async fn read_rejects_oversized() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("big.txt");
        write_file(&path, &vec![b'a'; 100]).await.unwrap();
        let err = read_file(&path, 10).await.unwrap_err();
        assert!(matches!(err, RuntimeError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn list_dir_sorted() {
        let dir = tempdir().unwrap();
        write_file(dir.path().join("b.txt"), b"").await.unwrap();
        write_file(dir.path().join("a.txt"), b"").await.unwrap();
        let entries = list_dir(dir.path()).await.unwrap();
        let names: Vec<_> = entries
            .iter()
            .map(|e| e.path.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, vec!["a.txt", "b.txt"]);
    }

    #[test]
    fn glob_recursive() {
        let dir = tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("a/b")).unwrap();
        std::fs::write(dir.path().join("x.rs"), b"").unwrap();
        std::fs::write(dir.path().join("a/y.rs"), b"").unwrap();
        std::fs::write(dir.path().join("a/b/z.rs"), b"").unwrap();
        std::fs::write(dir.path().join("a/b/z.txt"), b"").unwrap();

        let hits = glob(dir.path(), "**/*.rs").unwrap();
        assert_eq!(hits.len(), 3);
    }

    #[test]
    fn glob_single_segment() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("a.rs"), b"").unwrap();
        std::fs::write(dir.path().join("b.rs"), b"").unwrap();
        std::fs::write(dir.path().join("c.txt"), b"").unwrap();
        let hits = glob(dir.path(), "*.rs").unwrap();
        assert_eq!(hits.len(), 2);
    }
}
