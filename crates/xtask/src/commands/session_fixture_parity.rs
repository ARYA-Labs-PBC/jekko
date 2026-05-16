//! `xtask session-fixture-parity` — inventory TS vs. Rust session fixtures.
//!
//! Walks both fixture trees and reports any fixture present in the TS
//! tree but missing on the Rust side. The intent is to flag work for
//! whoever ports session-runtime fixtures over to the Rust runtime tests.
//!
//! When `--strict` is set we exit 1 on any missing fixture; otherwise
//! this is informational and always exits 0.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

use super::parity_diff::SetDiff;

/// Default TS fixture root (relative to repo root). The directory may
/// not exist; in that case we treat the inventory as empty.
pub const TS_FIXTURE_ROOT: &str = "packages/jekko/test/session";

/// Default Rust fixture root (relative to repo root).
pub const RUST_FIXTURE_ROOT: &str = "crates/jekko-runtime/tests/fixtures/sessions";

/// Rust-side fixtures that have no TS counterpart by design — typically
/// fixtures ported during the migration that re-shape the original TS test
/// data into a Rust-native form. Strict mode does not fail on these.
const RUST_EXTRAS: &[&str] = &[
    // Runtime-polish subagent ported the TS Effect-runtime compaction test
    // harness into a static JSON fixture asserting the policy decision shape.
    // No TS .fixture.ts counterpart exists.
    "compaction",
];

/// `true` if the file name should count as a "fixture". TS-side we
/// recognise either `*.fixture.ts` or files inside an explicit
/// `fixtures/` sub-directory. Rust-side we recognise anything under
/// the `fixtures/sessions/` directory (so the caller can shape that
/// directory however suits them).
fn is_ts_fixture(path: &Path) -> bool {
    if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
        name.ends_with(".fixture.ts") || name.ends_with(".fixture.json")
    } else {
        false
    }
}

/// Strip the `.fixture.ts` / `.fixture.json` suffix to get a stable
/// "fixture name" key for set diffing. For other TS files, returns
/// the filename minus extension.
fn ts_fixture_key(path: &Path) -> Option<String> {
    let name = path.file_name()?.to_str()?;
    for suffix in [".fixture.ts", ".fixture.json"] {
        if let Some(stem) = name.strip_suffix(suffix) {
            return Some(stem.to_string());
        }
    }
    None
}

/// Rust-side key. Drop the file extension to compare with TS stems.
fn rust_fixture_key(path: &Path) -> Option<String> {
    let name = path.file_name()?.to_str()?;
    let stem = Path::new(name).file_stem()?.to_str()?;
    Some(stem.to_string())
}

/// Recursively walk `root` collecting any path that matches the
/// supplied filter. Returns paths relative to `root`.
fn walk_with<F>(root: &Path, accept: F) -> Result<Vec<PathBuf>>
where
    F: Fn(&Path) -> bool,
{
    let mut out = Vec::new();
    if !root.exists() {
        return Ok(out);
    }
    walk_inner(root, root, &accept, &mut out)?;
    out.sort();
    Ok(out)
}

fn walk_inner<F>(base: &Path, dir: &Path, accept: &F, out: &mut Vec<PathBuf>) -> Result<()>
where
    F: Fn(&Path) -> bool,
{
    for entry in fs::read_dir(dir).with_context(|| format!("read {}", dir.display()))? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            walk_inner(base, &path, accept, out)?;
        } else if accept(&path) {
            if let Ok(rel) = path.strip_prefix(base) {
                out.push(rel.to_path_buf());
            }
        }
    }
    Ok(())
}

/// Compute the inventory delta. Returns `(ts_keys, rust_keys, diff)`.
pub fn inventory(repo_root: &Path) -> Result<(Vec<String>, Vec<String>, SetDiff)> {
    let ts_root = repo_root.join(TS_FIXTURE_ROOT);
    let rust_root = repo_root.join(RUST_FIXTURE_ROOT);

    let ts_files = walk_with(&ts_root, is_ts_fixture)?;
    let rust_files = walk_with(&rust_root, |p| {
        matches!(
            p.extension().and_then(|s| s.to_str()),
            Some("json") | Some("yaml") | Some("toml")
        )
    })?;

    let mut ts_keys: Vec<String> = ts_files.iter().filter_map(|p| ts_fixture_key(p)).collect();
    let mut rust_keys: Vec<String> = rust_files
        .iter()
        .filter_map(|p| rust_fixture_key(p))
        .collect();
    ts_keys.sort();
    ts_keys.dedup();
    rust_keys.sort();
    rust_keys.dedup();

    let diff = SetDiff::compute(rust_keys.clone(), ts_keys.clone());
    Ok((ts_keys, rust_keys, diff))
}

/// Run the parity command. With `strict = true`, exit non-zero if any
/// TS fixture is missing on the Rust side (true regression). Rust-side
/// fixtures listed in [`RUST_EXTRAS`] are tolerated.
pub fn run(repo_root: &Path, strict: bool) -> Result<()> {
    let (ts_keys, rust_keys, diff) = inventory(repo_root)?;

    // Post-cutover: TS fixture tree deleted. Nothing to diff against.
    if ts_keys.is_empty() && !diff.added.is_empty() {
        println!(
            "session-fixture-parity: SKIPPED — TS fixture tree absent (post-cutover), {} Rust fixture(s) present",
            rust_keys.len()
        );
        return Ok(());
    }

    println!(
        "session-fixture-parity: {} TS fixture(s), {} Rust fixture(s)",
        ts_keys.len(),
        rust_keys.len()
    );

    let unexpected_rust_only: Vec<&String> = diff
        .added
        .iter()
        .filter(|name| !RUST_EXTRAS.contains(&name.as_str()))
        .collect();

    // `diff.removed` = present in TS, missing in Rust = true regression.
    if !diff.removed.is_empty() {
        println!(
            "session-fixture-parity: {} TS fixture(s) not yet ported to Rust:",
            diff.removed.len()
        );
        for name in &diff.removed {
            println!("  - {name}");
        }
    }
    if !diff.added.is_empty() {
        let expected = diff.added.len() - unexpected_rust_only.len();
        println!(
            "session-fixture-parity: {} Rust fixture(s) with no TS counterpart ({} expected-extra, {} unexpected):",
            diff.added.len(),
            expected,
            unexpected_rust_only.len()
        );
        for name in &diff.added {
            let mark = if RUST_EXTRAS.contains(&name.as_str()) {
                "expected"
            } else {
                "UNEXPECTED"
            };
            println!("  + {name} ({mark})");
        }
    }

    if diff.removed.is_empty() && unexpected_rust_only.is_empty() {
        println!("session-fixture-parity: ✓ inventory matches (modulo expected extras)");
        return Ok(());
    }

    if strict {
        bail!(
            "session-fixture-parity: {} missing on Rust side, {} unexpected on Rust side",
            diff.removed.len(),
            unexpected_rust_only.len()
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ts_fixture_key_strips_known_suffixes() {
        assert_eq!(
            ts_fixture_key(Path::new("compaction.fixture.ts")),
            Some("compaction".to_string())
        );
        assert_eq!(
            ts_fixture_key(Path::new("retry.fixture.json")),
            Some("retry".to_string())
        );
        assert_eq!(ts_fixture_key(Path::new("not-a-fixture.ts")), None);
    }

    #[test]
    fn rust_fixture_key_drops_extension() {
        assert_eq!(
            rust_fixture_key(Path::new("compaction.json")),
            Some("compaction".to_string())
        );
    }

    #[test]
    fn inventory_returns_empty_when_roots_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let (ts, rust, diff) = inventory(tmp.path()).unwrap();
        assert!(ts.is_empty());
        assert!(rust.is_empty());
        assert!(diff.is_empty());
    }

    #[test]
    fn inventory_detects_unported_fixture() {
        let tmp = tempfile::tempdir().unwrap();
        let ts_root = tmp.path().join(TS_FIXTURE_ROOT);
        fs::create_dir_all(&ts_root).unwrap();
        fs::write(ts_root.join("compaction.fixture.ts"), "// fixture").unwrap();
        // Rust side is missing.
        let (ts, rust, diff) = inventory(tmp.path()).unwrap();
        assert_eq!(ts, vec!["compaction".to_string()]);
        assert!(rust.is_empty());
        assert_eq!(diff.removed, vec!["compaction".to_string()]);
    }

    #[test]
    fn run_strict_errors_when_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let ts_root = tmp.path().join(TS_FIXTURE_ROOT);
        fs::create_dir_all(&ts_root).unwrap();
        fs::write(ts_root.join("retry.fixture.ts"), "// fixture").unwrap();
        let err = run(tmp.path(), true).unwrap_err();
        assert!(format!("{err:#}").contains("missing on Rust side"));
    }

    #[test]
    fn run_non_strict_completes_cleanly() {
        let tmp = tempfile::tempdir().unwrap();
        run(tmp.path(), false).unwrap();
    }
}
