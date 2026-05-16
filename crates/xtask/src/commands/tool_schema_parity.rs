//! `xtask tool-schema-parity` — inventory the `jekko-runtime` tool set.
//!
//! Walks `crates/jekko-runtime/src/tool/` for `<tool>.rs` files (plus the
//! `edit/` sub-module), extracts each tool's id by grepping for the
//! `fn id(&self) -> &'static str` body, and diffs the resulting list
//! against the snapshot at `crates/xtask/fixtures/tool-schemas/index.json`.
//!
//! We deliberately do *not* take a runtime dependency on `jekko-runtime`
//! itself — pulling tokio + portable-pty + notify into xtask would balloon
//! cold-build time. The id discovery is therefore lexical: a tool file is
//! recognised when it declares `fn id(&self) -> &'static str { "X" }`.
//!
//! NOTE: once the runtime trait surface stabilises and the schema is
//! cheap to obtain, replace this lexical walker with a direct
//! `default_registry().catalog()` call from a thin runtime-bin or a
//! cargo subprocess.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde_json::json;

use super::parity_diff::SetDiff;

/// Default snapshot location.
pub const SNAPSHOT_REL: &str = "crates/xtask/fixtures/tool-schemas/index.json";

/// Directory we walk to find tool files. Each direct child `*.rs` and
/// `edit/mod.rs` is candidate.
const TOOL_DIR: &str = "crates/jekko-runtime/src/tool";

/// Public entry point. `strict` controls whether a non-empty diff
/// produces a non-zero exit.
pub fn run(repo_root: &Path, strict: bool) -> Result<()> {
    let discovered = discover_tool_ids(&repo_root.join(TOOL_DIR))?;
    let snapshot_path = repo_root.join(SNAPSHOT_REL);

    if !snapshot_path.exists() {
        if let Some(parent) = snapshot_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("create snapshot dir {}", parent.display()))?;
        }
        let body = build_snapshot_body(&discovered);
        fs::write(&snapshot_path, body)
            .with_context(|| format!("write initial snapshot {}", snapshot_path.display()))?;
        println!(
            "tool-schema-parity: snapshot did not exist — wrote initial baseline to {}",
            snapshot_path.display()
        );
        return Ok(());
    }

    let snapshot_text = fs::read_to_string(&snapshot_path)
        .with_context(|| format!("read snapshot {}", snapshot_path.display()))?;
    let snapshot: serde_json::Value = serde_json::from_str(&snapshot_text)
        .with_context(|| format!("parse snapshot {}", snapshot_path.display()))?;
    let expected: Vec<String> = match snapshot.get("tools").and_then(|v| v.as_array()) {
        Some(arr) => arr
            .iter()
            .filter_map(|v| v.get("id").and_then(|s| s.as_str()).map(|s| s.to_string()))
            .collect(),
        None => Vec::new(),
    };

    let actual: Vec<String> = discovered.keys().cloned().collect();
    let diff = SetDiff::compute(actual.clone(), expected.clone());

    if diff.is_empty() {
        println!(
            "tool-schema-parity: ✓ {} tools match snapshot",
            actual.len()
        );
        return Ok(());
    }

    println!(
        "tool-schema-parity: ✗ snapshot mismatch ({} added, {} removed)",
        diff.added.len(),
        diff.removed.len()
    );
    for id in &diff.added {
        println!("  + {id}");
    }
    for id in &diff.removed {
        println!("  - {id}");
    }

    if strict {
        bail!(
            "tool-schema-parity: snapshot mismatch ({} added, {} removed)",
            diff.added.len(),
            diff.removed.len()
        );
    }
    Ok(())
}

/// Format a tool source path relative to the `jekko-runtime` crate
/// root, joined with `/` for cross-platform JSON stability. If the
/// path doesn't traverse `jekko-runtime`, returns the original path.
fn relative_to_runtime(path: &Path) -> String {
    let mut parts: Vec<String> = Vec::new();
    let mut seen_runtime = false;
    for component in path.components() {
        if let std::path::Component::Normal(s) = component {
            let lossy = s.to_string_lossy();
            if lossy == "jekko-runtime" {
                seen_runtime = true;
            }
            if seen_runtime {
                parts.push(lossy.into_owned());
            }
        }
    }
    if parts.is_empty() {
        path.display().to_string()
    } else {
        parts.join("/")
    }
}

/// Build the JSON body of the initial snapshot.
fn build_snapshot_body(discovered: &BTreeMap<String, PathBuf>) -> String {
    let tools: Vec<serde_json::Value> = discovered
        .iter()
        .map(|(id, path)| {
            json!({
                "id": id,
                "source": relative_to_runtime(path),
            })
        })
        .collect();
    let doc = json!({
        "_note": "Replace this snapshot with `default_registry().catalog()` once the runtime trait is callable from xtask without pulling tokio.",
        "tools": tools,
    });
    let mut body = serde_json::to_string_pretty(&doc).unwrap();
    body.push('\n');
    body
}

/// Walk `tool_dir` looking for tool implementations. Returns
/// `{ tool_id: source_file }` sorted by id.
pub fn discover_tool_ids(tool_dir: &Path) -> Result<BTreeMap<String, PathBuf>> {
    let mut out: BTreeMap<String, PathBuf> = BTreeMap::new();
    if !tool_dir.exists() {
        return Ok(out);
    }
    visit_dir(tool_dir, &mut out)?;
    Ok(out)
}

fn visit_dir(dir: &Path, out: &mut BTreeMap<String, PathBuf>) -> Result<()> {
    for entry in fs::read_dir(dir).with_context(|| format!("read {}", dir.display()))? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            visit_dir(&path, out)?;
        } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            if let Some(id) = extract_tool_id(&path)? {
                out.insert(id, path);
            }
        }
    }
    Ok(())
}

/// Look for the canonical `fn id(&self) -> &'static str { "X" }`
/// signature. Returns the literal string `X` or `None` if no id is
/// declared in this file.
fn extract_tool_id(path: &Path) -> Result<Option<String>> {
    let text =
        fs::read_to_string(path).with_context(|| format!("read tool source {}", path.display()))?;
    let needle = "fn id(&self) -> &'static str";
    let bytes = text.as_bytes();
    // Use a simple substring search; not perf-critical.
    if let Some(idx) = find_substring(bytes, needle.as_bytes()) {
        // Skip past the signature; find the first quoted literal on
        // the next few lines.
        let after = &text[idx + needle.len()..];
        if let Some(start_quote) = after.find('"') {
            let body = &after[start_quote + 1..];
            if let Some(end_quote) = body.find('"') {
                return Ok(Some(body[..end_quote].to_string()));
            }
        }
    }
    Ok(None)
}

fn find_substring(hay: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    hay.windows(needle.len()).position(|w| w == needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_substring_returns_first_match() {
        assert_eq!(find_substring(b"abcdef", b"cd"), Some(2));
        assert_eq!(find_substring(b"aaaa", b"aa"), Some(0));
        assert_eq!(find_substring(b"abc", b"xyz"), None);
    }

    #[test]
    fn extract_tool_id_parses_canonical_signature() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bash.rs");
        std::fs::write(
            &path,
            "#[async_trait]\nimpl Tool for BashTool {\n    fn id(&self) -> &'static str { \"bash\" }\n}\n",
        )
        .unwrap();
        assert_eq!(extract_tool_id(&path).unwrap(), Some("bash".to_string()));
    }

    #[test]
    fn extract_tool_id_returns_none_when_signature_absent() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("not-a-tool.rs");
        std::fs::write(&path, "pub fn helper() {}\n").unwrap();
        assert_eq!(extract_tool_id(&path).unwrap(), None);
    }

    #[test]
    fn discover_tool_ids_walks_subdirectories() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("bash.rs"),
            "fn id(&self) -> &'static str { \"bash\" }\n",
        )
        .unwrap();
        let edit_dir = dir.path().join("edit");
        std::fs::create_dir(&edit_dir).unwrap();
        std::fs::write(
            edit_dir.join("mod.rs"),
            "fn id(&self) -> &'static str { \"edit\" }\n",
        )
        .unwrap();
        let found = discover_tool_ids(dir.path()).unwrap();
        let ids: Vec<String> = found.keys().cloned().collect();
        assert_eq!(ids, vec!["bash".to_string(), "edit".to_string()]);
    }

    #[test]
    fn run_first_pass_seeds_snapshot() {
        let repo = tempfile::tempdir().unwrap();
        // Seed a tool tree under the canonical path.
        let tool_root = repo.path().join(TOOL_DIR);
        std::fs::create_dir_all(&tool_root).unwrap();
        std::fs::write(
            tool_root.join("bash.rs"),
            "fn id(&self) -> &'static str { \"bash\" }\n",
        )
        .unwrap();
        std::fs::write(
            tool_root.join("read.rs"),
            "fn id(&self) -> &'static str { \"read\" }\n",
        )
        .unwrap();

        run(repo.path(), false).unwrap();
        let snapshot = repo.path().join(SNAPSHOT_REL);
        assert!(snapshot.exists());
        let body = std::fs::read_to_string(&snapshot).unwrap();
        assert!(body.contains("\"bash\""));
        assert!(body.contains("\"read\""));
    }

    #[test]
    fn run_second_pass_detects_addition() {
        let repo = tempfile::tempdir().unwrap();
        let tool_root = repo.path().join(TOOL_DIR);
        std::fs::create_dir_all(&tool_root).unwrap();
        std::fs::write(
            tool_root.join("bash.rs"),
            "fn id(&self) -> &'static str { \"bash\" }\n",
        )
        .unwrap();
        // First run seeds.
        run(repo.path(), false).unwrap();

        // Add a tool.
        std::fs::write(
            tool_root.join("grep.rs"),
            "fn id(&self) -> &'static str { \"grep\" }\n",
        )
        .unwrap();
        let err = run(repo.path(), true).unwrap_err();
        assert!(format!("{err:#}").contains("snapshot mismatch"));
    }
}
