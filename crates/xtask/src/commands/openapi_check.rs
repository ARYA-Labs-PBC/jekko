//! `xtask openapi-check` — diff the live OpenAPI doc against a snapshot.
//!
//! The live doc is obtained by spawning a `cargo run -p jekko-server --bin
//! openapi-dump` subprocess if (and only if) that binary target is declared
//! on `jekko-server`. Otherwise we log a SKIP and exit 0 — the binary will
//! be added by the E subagent once the server routes stabilise.
//!
//! Snapshot lives at `docs/openapi-snapshot.json`. First run seeds it.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context, Result};

pub const SNAPSHOT_REL: &str = "docs/openapi-snapshot.json";

/// Probe `crates/jekko-server/Cargo.toml` for an `openapi-dump` binary
/// target. Returns true when the helper is wired up so we can call into
/// it; false otherwise.
pub fn server_has_openapi_dump(repo_root: &Path) -> bool {
    let manifest = repo_root.join("crates/jekko-server/Cargo.toml");
    let Ok(text) = fs::read_to_string(&manifest) else {
        return false;
    };
    text.contains("openapi-dump") || text.contains("openapi_dump")
}

/// Run the check. Returns Ok on success, or Err when `strict` is set and
/// the snapshot diverges.
pub fn run(repo_root: &Path, strict: bool) -> Result<()> {
    if !server_has_openapi_dump(repo_root) {
        // The dump binary is not wired yet, so openapi-check is a no-op.
        println!("openapi-check: SKIPPED — jekko-server has no `openapi-dump` bin target yet");
        return Ok(());
    }

    let live = capture_openapi(repo_root)?;
    let snapshot_path = snapshot_path(repo_root);

    if !snapshot_path.exists() {
        if let Some(parent) = snapshot_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("create snapshot dir {}", parent.display()))?;
        }
        fs::write(&snapshot_path, &live)
            .with_context(|| format!("write initial snapshot {}", snapshot_path.display()))?;
        println!(
            "openapi-check: snapshot did not exist — wrote initial baseline to {}",
            snapshot_path.display()
        );
        return Ok(());
    }

    let expected = fs::read_to_string(&snapshot_path)
        .with_context(|| format!("read snapshot {}", snapshot_path.display()))?;
    if normalize(&live) == normalize(&expected) {
        println!("openapi-check: ✓ live doc matches snapshot");
        return Ok(());
    }

    let live_lines = live.lines().count();
    let expected_lines = expected.lines().count();
    println!(
        "openapi-check: ✗ snapshot differs (live {} lines, snapshot {} lines)",
        live_lines, expected_lines
    );
    if strict {
        bail!("openapi-check: snapshot mismatch");
    }
    Ok(())
}

fn snapshot_path(repo_root: &Path) -> PathBuf {
    repo_root.join(SNAPSHOT_REL)
}

/// Spawn the dump helper and return its stdout.
fn capture_openapi(repo_root: &Path) -> Result<String> {
    let mut cmd = Command::new("cargo");
    cmd.args([
        "run",
        "--quiet",
        "-p",
        "jekko-server",
        "--bin",
        "openapi-dump",
    ]);
    cmd.current_dir(repo_root);

    let output = cmd
        .output()
        .with_context(|| "spawn `cargo run -p jekko-server --bin openapi-dump`")?;
    if !output.status.success() {
        bail!(
            "openapi-dump failed (exit {}): {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Canonicalize a JSON-ish blob so cosmetic re-serialisation doesn't
/// cause false positives. Best-effort: if the input isn't valid JSON
/// we just compare it raw.
fn normalize(text: &str) -> String {
    let value = match serde_json::from_str::<serde_json::Value>(text) {
        Ok(value) => value,
        Err(_) => return text.to_string(),
    };
    match serde_json::to_string(&value) {
        Ok(canonical) => canonical,
        Err(_) => text.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_has_openapi_dump_returns_false_when_missing() {
        let tmp = tempfile::tempdir().unwrap();
        // No manifest at all.
        assert!(!server_has_openapi_dump(tmp.path()));
    }

    #[test]
    fn server_has_openapi_dump_detects_bin_declaration() {
        let tmp = tempfile::tempdir().unwrap();
        let manifest_dir = tmp.path().join("crates/jekko-server");
        fs::create_dir_all(&manifest_dir).unwrap();
        fs::write(
            manifest_dir.join("Cargo.toml"),
            "[[bin]]\nname = \"openapi-dump\"\npath = \"src/bin/openapi_dump.rs\"\n",
        )
        .unwrap();
        assert!(server_has_openapi_dump(tmp.path()));
    }

    #[test]
    fn run_skips_when_server_not_wired() {
        let tmp = tempfile::tempdir().unwrap();
        // No manifest declares the bin -> skip path -> Ok.
        run(tmp.path(), true).unwrap();
    }

    #[test]
    fn normalize_collapses_whitespace_in_json() {
        let a = "{\n  \"a\": 1\n}";
        let b = "{\"a\":1}";
        assert_eq!(normalize(a), normalize(b));
    }

    #[test]
    fn normalize_falls_back_to_raw_for_non_json() {
        let raw = "not a json doc";
        assert_eq!(normalize(raw), raw.to_string());
    }
}
