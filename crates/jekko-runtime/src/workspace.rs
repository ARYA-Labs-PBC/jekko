//! Workspace state.
//!
//! Ported from `packages/jekko/src/control-plane/workspace.*.ts`. The TS
//! "workspace" is a named subdirectory inside a project's worktree —
//! often a separate worktree, branch, or dev environment slot. Here we
//! port the in-memory shape; full SQLite-backed CRUD is provided by
//! [`jekko_store::workspace`] when the runtime is wired up against a real DB.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Workspace record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceInfo {
    /// Workspace id.
    pub id: String,
    /// Workspace kind/type tag (free-form string in TS).
    #[serde(rename = "type")]
    pub kind: String,
    /// Human-friendly name.
    pub name: String,
    /// FK to project id.
    pub project_id: String,
    /// Optional branch tag.
    #[serde(default)]
    pub branch: Option<String>,
    /// Optional working directory.
    #[serde(default)]
    pub directory: Option<PathBuf>,
}

impl WorkspaceInfo {
    /// Construct a minimal workspace.
    pub fn new(
        id: impl Into<String>,
        kind: impl Into<String>,
        project_id: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            kind: kind.into(),
            name: String::new(),
            project_id: project_id.into(),
            branch: None,
            directory: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_json() {
        let ws = WorkspaceInfo::new("ws_1", "worktree", "proj_1");
        let json = serde_json::to_string(&ws).unwrap();
        let de: WorkspaceInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(ws, de);
    }
}
