//! `xtask httpapi-parity` — compare TS HTTP handlers against the Rust port.
//!
//! Walks the TS handler directory (`packages/jekko/src/server/routes/instance/httpapi/handlers/`)
//! and the Rust router source (`crates/jekko-server/src/routes/`). Matches by
//! file stem. Reports TS handlers that have no Rust counterpart and vice
//! versa.
//!
//! Per the packet plan, when `crates/jekko-server/src/routes/` does not
//! yet exist (E subagent still in flight) this command exits 0 with a
//! "skipped" message — the parent CI loop uses this as the synchronization
//! point.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

use super::parity_diff::SetDiff;

pub const TS_HANDLER_ROOT: &str = "packages/jekko/src/server/routes/instance/httpapi/handlers";
pub const RUST_ROUTE_ROOT: &str = "crates/jekko-server/src/routes";

/// Routes that exist on the Rust side but have no TS counterpart, by design.
/// These are extensions added during the port (event streaming, OpenAPI doc,
/// v2 endpoints, raw WebSocket bridge).
const EXPECTED_RUST_EXTRAS: &[&str] = &["events", "openapi", "v2", "ws"];

/// TS-side handlers that have not been ported yet but are explicitly deferred.
/// These will land in a follow-up packet once the runtime/control surfaces
/// they depend on stabilise.
const TS_DEFERRED: &[&str] = &[
    "control",        // control-plane routes — needs workspace orchestration port
    "global",         // global config — minor; runtime API not yet exposed
    "index",          // index endpoint — wired via Axum routing instead
    "project",        // project routes — needs project service trait
    "session-errors", // session-error reporting — needs runtime error bus
];

/// Walk a directory tree returning every regular file path relative to
/// `root`.
fn walk_files(root: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    if !root.exists() {
        return Ok(out);
    }
    walk_inner(root, root, &mut out)?;
    out.sort();
    Ok(out)
}

fn walk_inner(base: &Path, dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(dir).with_context(|| format!("read {}", dir.display()))? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            walk_inner(base, &path, out)?;
        } else if let Ok(rel) = path.strip_prefix(base) {
            out.push(rel.to_path_buf());
        }
    }
    Ok(())
}

/// Compute the unique handler "keys" used for diffing. We strip the
/// file extension and treat `mod.rs` as a directory entry (using the
/// parent directory name as the key).
fn handler_keys(files: &[PathBuf], extension: &str) -> Vec<String> {
    let mut keys: Vec<String> = files
        .iter()
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some(extension))
        .filter_map(|p| handler_key_for(p))
        .collect();
    keys.sort();
    keys.dedup();
    keys
}

fn handler_key_for(path: &Path) -> Option<String> {
    let stem = path.file_stem()?.to_str()?;
    if stem == "mod" {
        // Use the parent directory name.
        path.parent()
            .and_then(|p| p.file_name())
            .and_then(|s| s.to_str())
            .map(|s| s.to_string())
    } else {
        Some(stem.to_string())
    }
}

/// Returns `(ts_keys, rust_keys, diff, skipped)`. `skipped` is true when
/// either side of the diff is absent — meaning the migration has either not
/// started (no Rust routes yet) or has completed (Codex removed the TS
/// handler tree post-port).
pub fn inventory(repo_root: &Path) -> Result<(Vec<String>, Vec<String>, SetDiff, bool)> {
    let ts_root = repo_root.join(TS_HANDLER_ROOT);
    let rust_root = repo_root.join(RUST_ROUTE_ROOT);

    let ts_files = walk_files(&ts_root)?;
    let ts_keys = handler_keys(&ts_files, "ts");

    if !rust_root.exists() {
        return Ok((ts_keys, Vec::new(), SetDiff::default(), true));
    }

    let rust_files = walk_files(&rust_root)?;
    let rust_keys = handler_keys(&rust_files, "rs");

    // Post-cutover: TS handler tree has been deleted. There's nothing to
    // diff against — gate is informational from here on.
    if ts_keys.is_empty() {
        return Ok((ts_keys, rust_keys, SetDiff::default(), true));
    }

    let diff = SetDiff::compute(rust_keys.clone(), ts_keys.clone());
    Ok((ts_keys, rust_keys, diff, false))
}

pub fn run(repo_root: &Path, strict: bool) -> Result<()> {
    let (ts_keys, rust_keys, diff, skipped) = inventory(repo_root)?;
    if skipped {
        if ts_keys.is_empty() {
            println!(
                "httpapi-parity: SKIPPED — TS handler tree absent (post-cutover), {} Rust handler(s) present",
                rust_keys.len()
            );
        } else {
            println!(
                "httpapi-parity: SKIPPED — {} does not exist yet",
                RUST_ROUTE_ROOT
            );
            println!("httpapi-parity: TS side has {} handler(s)", ts_keys.len());
        }
        return Ok(());
    }
    println!(
        "httpapi-parity: {} TS handler(s), {} Rust handler(s)",
        ts_keys.len(),
        rust_keys.len()
    );

    // Filter the diff against the known-good extras + deferred lists.
    let unexpected_ts_only: Vec<&String> = diff
        .removed
        .iter()
        .filter(|name| !TS_DEFERRED.contains(&name.as_str()))
        .collect();
    let unexpected_rust_only: Vec<&String> = diff
        .added
        .iter()
        .filter(|name| !EXPECTED_RUST_EXTRAS.contains(&name.as_str()))
        .collect();

    if !diff.removed.is_empty() {
        let expected = diff.removed.len() - unexpected_ts_only.len();
        println!(
            "httpapi-parity: {} TS handler(s) missing on Rust side ({} expected-deferred, {} unexpected):",
            diff.removed.len(),
            expected,
            unexpected_ts_only.len()
        );
        for name in &diff.removed {
            let mark = if TS_DEFERRED.contains(&name.as_str()) {
                "deferred"
            } else {
                "UNEXPECTED"
            };
            println!("  - {name} ({mark})");
        }
    }
    if !diff.added.is_empty() {
        let expected = diff.added.len() - unexpected_rust_only.len();
        println!(
            "httpapi-parity: {} Rust handler(s) with no TS counterpart ({} expected-extra, {} unexpected):",
            diff.added.len(),
            expected,
            unexpected_rust_only.len()
        );
        for name in &diff.added {
            let mark = if EXPECTED_RUST_EXTRAS.contains(&name.as_str()) {
                "expected"
            } else {
                "UNEXPECTED"
            };
            println!("  + {name} ({mark})");
        }
    }

    if unexpected_ts_only.is_empty() && unexpected_rust_only.is_empty() {
        println!("httpapi-parity: ✓ inventory matches (modulo expected extras + deferred)");
        return Ok(());
    }
    if strict {
        bail!(
            "httpapi-parity: {} unexpected TS-only, {} unexpected Rust-only",
            unexpected_ts_only.len(),
            unexpected_rust_only.len()
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handler_key_for_strips_extension_and_handles_mod_rs() {
        assert_eq!(
            handler_key_for(Path::new("session.rs")),
            Some("session".to_string())
        );
        assert_eq!(
            handler_key_for(Path::new("v2/mod.rs")),
            Some("v2".to_string())
        );
    }

    #[test]
    fn handler_keys_filters_by_extension() {
        let files = vec![
            PathBuf::from("session.ts"),
            PathBuf::from("session.rs"),
            PathBuf::from("v2/mod.rs"),
            PathBuf::from("README.md"),
        ];
        assert_eq!(handler_keys(&files, "ts"), vec!["session"]);
        let rs = handler_keys(&files, "rs");
        assert_eq!(rs, vec!["session", "v2"]);
    }

    #[test]
    fn inventory_marks_skipped_when_rust_root_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let ts_root = tmp.path().join(TS_HANDLER_ROOT);
        fs::create_dir_all(&ts_root).unwrap();
        fs::write(ts_root.join("session.ts"), "// fixture").unwrap();
        let (ts, rust, diff, skipped) = inventory(tmp.path()).unwrap();
        assert!(skipped);
        assert_eq!(ts, vec!["session".to_string()]);
        assert!(rust.is_empty());
        assert!(diff.is_empty());
    }

    #[test]
    fn inventory_reports_missing_rust_handler() {
        let tmp = tempfile::tempdir().unwrap();
        let ts_root = tmp.path().join(TS_HANDLER_ROOT);
        let rust_root = tmp.path().join(RUST_ROUTE_ROOT);
        fs::create_dir_all(&ts_root).unwrap();
        fs::create_dir_all(&rust_root).unwrap();
        fs::write(ts_root.join("session.ts"), "// fixture").unwrap();
        fs::write(ts_root.join("config.ts"), "// fixture").unwrap();
        fs::write(rust_root.join("session.rs"), "// fixture").unwrap();
        let (ts, rust, diff, skipped) = inventory(tmp.path()).unwrap();
        assert!(!skipped);
        assert_eq!(ts.len(), 2);
        assert_eq!(rust.len(), 1);
        assert_eq!(diff.removed, vec!["config".to_string()]);
    }

    #[test]
    fn run_in_strict_mode_errors_on_diff() {
        let tmp = tempfile::tempdir().unwrap();
        let ts_root = tmp.path().join(TS_HANDLER_ROOT);
        let rust_root = tmp.path().join(RUST_ROUTE_ROOT);
        fs::create_dir_all(&ts_root).unwrap();
        fs::create_dir_all(&rust_root).unwrap();
        fs::write(ts_root.join("session.ts"), "// fixture").unwrap();
        let err = run(tmp.path(), true).unwrap_err();
        assert!(format!("{err:#}").contains("TS-only"));
    }

    #[test]
    fn run_skipped_branch_exits_zero() {
        let tmp = tempfile::tempdir().unwrap();
        // Neither root exists -- treated as skipped.
        run(tmp.path(), true).unwrap();
    }
}
