//! Plan computation and reporting struct for the Packet O cutover.
//!
//! `compute_plan(repo_root)` returns a [`CutoverPlan`] enumerating every path
//! the cutover will touch (delete_files / delete_dirs / edit_files). It does
//! NOT touch the filesystem itself — `main.rs::run_cleanup_cutover` is the
//! IO layer that consumes the plan; this keeps computation separate from IO
//! and makes the cutover reviewable.

use std::path::{Path, PathBuf};

use super::inventory::{
    BUILD_CACHES, EDIT_DOCS, EDIT_GATED, EDIT_TOP_LEVEL, GITHUB_WORKFLOWS_BUN, HUSKY_FILES,
    MINI_FLEET, NIX_FILES, PACKAGES_DIRS, PACKAGE_LOCAL_NOTES, PATCHES, ROOT_MANIFESTS,
    ROOT_SCRIPTS_MJS, ROOT_SCRIPT_OTHER, ROOT_SCRIPT_TS, ROOT_TEST_DIRS, SETUP_BUN_ACTION,
    SPECS_DIRS, TOOLS_JS,
};

/// A bundled plan describing the inventory the cutover will act on.
///
/// `delete_files` and `delete_dirs` are paths the executor will remove from
/// the working tree. `edit_files` are paths that will be updated in place
/// because they still serve a runtime purpose until the Rust replacement
/// lands.
#[derive(Debug, Clone)]
pub struct CutoverPlan {
    pub delete_files: Vec<PathBuf>,
    pub delete_dirs: Vec<PathBuf>,
    pub edit_files: Vec<PathBuf>,
}

impl CutoverPlan {
    /// Total number of paths in the plan. Exposed for downstream tooling.
    #[allow(dead_code)]
    pub fn total(&self) -> usize {
        self.delete_files.len() + self.delete_dirs.len() + self.edit_files.len()
    }
}

/// Compute the planned cutover relative to `repo_root`. Does NOT touch the
/// filesystem.
///
/// `repo_root` is joined onto every entry so the caller can hand the plan
/// directly to a future executor; this is intentional even though the
/// current callers only inspect the names.
pub fn compute_plan(repo_root: &Path) -> CutoverPlan {
    let join = |relative: &str| repo_root.join(relative);

    let delete_files: Vec<PathBuf> = ROOT_MANIFESTS
        .iter()
        .chain(HUSKY_FILES.iter())
        .chain(ROOT_SCRIPT_TS.iter())
        .chain(ROOT_SCRIPT_OTHER.iter())
        .chain(ROOT_SCRIPTS_MJS.iter())
        .chain(TOOLS_JS.iter())
        .chain(PATCHES.iter())
        .chain(NIX_FILES.iter())
        .chain(GITHUB_WORKFLOWS_BUN.iter())
        .chain(PACKAGE_LOCAL_NOTES.iter())
        .map(|relative| join(relative))
        .collect();

    let delete_dirs: Vec<PathBuf> = PACKAGES_DIRS
        .iter()
        .chain(BUILD_CACHES.iter())
        .chain(MINI_FLEET.iter())
        .chain(ROOT_TEST_DIRS.iter())
        .chain(SETUP_BUN_ACTION.iter())
        .chain(SPECS_DIRS.iter())
        .map(|relative| join(relative))
        .collect();

    let edit_files: Vec<PathBuf> = EDIT_GATED
        .iter()
        .chain(EDIT_DOCS.iter())
        .chain(EDIT_TOP_LEVEL.iter())
        .map(|relative| join(relative))
        .collect();

    CutoverPlan {
        delete_files,
        delete_dirs,
        edit_files,
    }
}
