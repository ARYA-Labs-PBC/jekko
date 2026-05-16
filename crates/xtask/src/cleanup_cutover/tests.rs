//! Tests for the Packet O cutover plan.

#![cfg(test)]

use std::path::{Path, PathBuf};

use super::compute_plan;

#[test]
fn plan_is_buildable() {
    let plan = compute_plan(Path::new("."));
    // Smoke test only; real assertions land when M-prep is done and
    // main.rs can dispatch cleanup-cutover.
    assert!(plan.delete_files.len() <= 10_000);
    assert!(plan.delete_dirs.len() <= 10_000);
    assert!(plan.edit_files.len() <= 10_000);
}

#[test]
fn plan_includes_bun_lock_and_packages_jekko() {
    let plan = compute_plan(Path::new("/repo"));
    let want_file = PathBuf::from("/repo/bun.lock");
    let want_dir = PathBuf::from("/repo/packages/jekko");
    assert!(
        plan.delete_files.iter().any(|p| p == &want_file),
        "bun.lock must be in delete_files",
    );
    assert!(
        plan.delete_dirs.iter().any(|p| p == &want_dir),
        "packages/jekko must be in delete_dirs",
    );
}

#[test]
fn plan_keeps_jekko_sh_in_edit_files() {
    let plan = compute_plan(Path::new("/repo"));
    let gated = PathBuf::from("/repo/ops/ci/jekko.sh");
    assert!(
        plan.edit_files.iter().any(|p| p == &gated),
        "ops/ci/jekko.sh must be edit-gated, not deleted",
    );
    assert!(
        !plan.delete_files.iter().any(|p| p == &gated),
        "ops/ci/jekko.sh must not be in delete_files",
    );
}

#[test]
fn plan_total_is_nontrivial() {
    let plan = compute_plan(Path::new("."));
    assert!(
        plan.total() > 50,
        "plan should cover the bulk of the JS surface"
    );
}

#[test]
fn plan_covers_root_manifests() {
    let plan = compute_plan(Path::new("/r"));
    for required in [
        "/r/package.json",
        "/r/tsconfig.json",
        "/r/bunfig.toml",
        "/r/bun.lock",
        "/r/turbo.json",
    ] {
        let p = PathBuf::from(required);
        assert!(
            plan.delete_files.iter().any(|q| q == &p),
            "{required} must be in delete_files"
        );
    }
}

#[test]
fn plan_covers_jekko_user_plugins() {
    // `.jekko` contains the tui-smoke plugin tree; deleting the dir
    // removes the `@opentui` guard hits in one sweep.
    let plan = compute_plan(Path::new("/r"));
    assert!(
        plan.delete_dirs
            .iter()
            .any(|p| p == &PathBuf::from("/r/.jekko")),
        ".jekko/ must be in delete_dirs"
    );
}

#[test]
fn plan_covers_setup_bun_action() {
    let plan = compute_plan(Path::new("/r"));
    assert!(
        plan.delete_dirs
            .iter()
            .any(|p| p == &PathBuf::from("/r/.github/actions/setup-bun")),
        ".github/actions/setup-bun must be in delete_dirs"
    );
}

#[test]
fn plan_covers_root_test_dir() {
    // root `test/` has `local/jnoccio-unlock.local.test.ts` which uses
    // `bun:test`; without the dir in the plan the guard final would never
    // reach zero hits.
    let plan = compute_plan(Path::new("/r"));
    assert!(
        plan.delete_dirs
            .iter()
            .any(|p| p == &PathBuf::from("/r/test")),
        "root test/ must be in delete_dirs"
    );
}

#[test]
fn plan_does_not_delete_mini_fleet_smoke() {
    // mini-fleet-smoke was a Bun crate; it's been migrated to a Rust
    // workspace member. The plan must NOT delete it, or `cargo build
    // --workspace` post-cutover would fail.
    let plan = compute_plan(Path::new("/r"));
    assert!(
        !plan
            .delete_dirs
            .iter()
            .any(|p| p == &PathBuf::from("/r/mini-fleet-smoke")),
        "mini-fleet-smoke is now a Rust crate; must NOT be deleted"
    );
}

#[test]
fn plan_does_not_delete_workspace_critical_paths() {
    // Sanity: paths that the Rust workspace depends on must NOT be in any
    // of the delete lists.
    let plan = compute_plan(Path::new("/r"));
    let must_keep = [
        "/r/Cargo.toml",
        "/r/Cargo.lock",
        "/r/crates",
        "/r/agent",
        "/r/db",
        "/r/docs/architecture.md",
        "/r/.github/workflows/parity.yml",
    ];
    for p in must_keep {
        let pb = PathBuf::from(p);
        assert!(
            !plan.delete_files.iter().any(|q| q == &pb),
            "{p} must NOT be in delete_files"
        );
        assert!(
            !plan.delete_dirs.iter().any(|q| q == &pb),
            "{p} must NOT be in delete_dirs"
        );
    }
}

#[test]
fn plan_is_deterministic_across_invocations() {
    let p1 = compute_plan(Path::new("/r"));
    let p2 = compute_plan(Path::new("/r"));
    assert_eq!(p1.delete_files, p2.delete_files);
    assert_eq!(p1.delete_dirs, p2.delete_dirs);
    assert_eq!(p1.edit_files, p2.edit_files);
}

#[test]
fn plan_paths_are_unique_within_each_list() {
    let plan = compute_plan(Path::new("/r"));
    let mut dfs: Vec<_> = plan.delete_files.clone();
    let pre = dfs.len();
    dfs.sort();
    dfs.dedup();
    assert_eq!(dfs.len(), pre, "delete_files contains duplicates");

    let mut dds: Vec<_> = plan.delete_dirs.clone();
    let pre = dds.len();
    dds.sort();
    dds.dedup();
    assert_eq!(dds.len(), pre, "delete_dirs contains duplicates");

    let mut efs: Vec<_> = plan.edit_files.clone();
    let pre = efs.len();
    efs.sort();
    efs.dedup();
    assert_eq!(efs.len(), pre, "edit_files contains duplicates");
}
