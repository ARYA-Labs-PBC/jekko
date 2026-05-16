//! Shell out to `rg` for fast recursive grep.
//!
//! Ported from `packages/jekko/src/util/ripgrep.ts`. The TS module embeds
//! a regex walker; here we keep it simple and require the user to have
//! `rg` on `$PATH`, using a pure-Rust grep when it is missing.

use std::path::{Path, PathBuf};
use std::process::Stdio;

use serde::{Deserialize, Serialize};
use tokio::process::Command;

use crate::error::{RuntimeError, RuntimeResult};

/// One ripgrep match.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrepMatch {
    /// Matching file path.
    pub path: PathBuf,
    /// 1-indexed line number.
    pub line: u32,
    /// Full text of the matching line.
    pub text: String,
}

/// Run a grep against `pattern` rooted at `base`. Prefers `rg`; otherwise
/// uses the built-in walker when `rg` is missing.
pub async fn grep(base: impl AsRef<Path>, pattern: &str) -> RuntimeResult<Vec<GrepMatch>> {
    let base = base.as_ref().to_path_buf();
    if rg_available().await {
        rg(&base, pattern).await
    } else {
        walk_grep(&base, pattern).await
    }
}

async fn rg_available() -> bool {
    Command::new("rg")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
        .map(|s| s.success())
        .unwrap_or(false)
}

async fn rg(base: &Path, pattern: &str) -> RuntimeResult<Vec<GrepMatch>> {
    let out = Command::new("rg")
        .arg("--line-number")
        .arg("--no-heading")
        .arg("--color")
        .arg("never")
        .arg(pattern)
        .arg(base)
        .stderr(Stdio::null())
        .output()
        .await?;
    if !out.status.success() && out.status.code().is_none_or(|c| c != 1) {
        return Err(RuntimeError::Command(
            String::from_utf8_lossy(&out.stderr).to_string(),
        ));
    }
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    Ok(parse_rg_output(&text))
}

fn parse_rg_output(text: &str) -> Vec<GrepMatch> {
    let mut out = Vec::new();
    for line in text.lines() {
        if let Some((path, rest)) = line.split_once(':') {
            if let Some((lineno, body)) = rest.split_once(':') {
                if let Ok(n) = lineno.parse::<u32>() {
                    out.push(GrepMatch {
                        path: PathBuf::from(path),
                        line: n,
                        text: body.to_string(),
                    });
                }
            }
        }
    }
    out
}

async fn walk_grep(base: &Path, pattern: &str) -> RuntimeResult<Vec<GrepMatch>> {
    let re = regex::Regex::new(pattern).map_err(|err| RuntimeError::invalid(err.to_string()))?;
    let mut out = Vec::new();
    let mut stack: Vec<PathBuf> = vec![base.to_path_buf()];
    while let Some(p) = stack.pop() {
        let meta = match tokio::fs::metadata(&p).await {
            Ok(m) => m,
            Err(_) => continue,
        };
        if meta.is_dir() {
            let mut rd = match tokio::fs::read_dir(&p).await {
                Ok(r) => r,
                Err(_) => continue,
            };
            while let Ok(Some(entry)) = rd.next_entry().await {
                stack.push(entry.path());
            }
        } else if meta.is_file() {
            if let Ok(bytes) = tokio::fs::read(&p).await {
                if let Ok(text) = std::str::from_utf8(&bytes) {
                    for (idx, line) in text.lines().enumerate() {
                        if re.is_match(line) {
                            out.push(GrepMatch {
                                path: p.clone(),
                                line: (idx + 1) as u32,
                                text: line.to_string(),
                            });
                        }
                    }
                }
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn finds_match_via_walk_or_rg() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("a.txt"), b"hello world\nfoo bar\n").unwrap();
        std::fs::write(dir.path().join("b.txt"), b"baz qux\n").unwrap();
        let hits = grep(dir.path(), "foo").await.unwrap();
        assert_eq!(hits.len(), 1);
        assert!(hits[0].text.contains("foo"));
    }
}
