//! Project state.
//!
//! Ported from `packages/jekko/src/project/` (instance + bootstrap helpers).
//! The TS layer wraps the SQLite `project` table with bootstrapping logic
//! (auto-naming from the worktree path, VCS detection). We keep that
//! surface area minimal: the rest of the runtime treats the project as a
//! handful of derived facts (id, worktree path, optional VCS tag).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{RuntimeError, RuntimeResult};

/// Default project name used when the worktree leaf cannot be derived.
const DEFAULT_PROJECT_NAME: &str = "project";

/// Materialised project record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectInfo {
    /// Project id.
    pub id: String,
    /// Worktree path.
    pub worktree: PathBuf,
    /// Detected VCS tag (`"git"`, `"hg"`, …) if known.
    pub vcs: Option<String>,
    /// Human-friendly name (default derived from worktree leaf).
    pub name: String,
}

/// Create a `ProjectInfo` from a worktree path, auto-detecting VCS and
/// deriving an id / name. Does not touch the database.
pub fn from_worktree(worktree: impl AsRef<Path>) -> RuntimeResult<ProjectInfo> {
    let worktree = worktree.as_ref().to_path_buf();
    if !worktree.exists() {
        return Err(RuntimeError::invalid(format!(
            "worktree {} does not exist",
            worktree.display()
        )));
    }
    let id = format!("proj_{}", Uuid::new_v4().simple());
    let vcs = detect_vcs(&worktree);
    let name = match worktree.file_name() {
        Some(n) => n.to_string_lossy().into_owned(),
        None => DEFAULT_PROJECT_NAME.to_string(),
    };
    Ok(ProjectInfo {
        id,
        worktree,
        vcs,
        name,
    })
}

/// Detect the VCS rooted at `path`. Currently recognises `git` and `hg`.
pub fn detect_vcs(path: &Path) -> Option<String> {
    if path.join(".git").exists() {
        return Some("git".into());
    }
    if path.join(".hg").exists() {
        return Some("hg".into());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn from_worktree_finds_git() {
        let dir = tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(".git")).unwrap();
        let info = from_worktree(dir.path()).unwrap();
        assert_eq!(info.vcs.as_deref(), Some("git"));
    }
}
