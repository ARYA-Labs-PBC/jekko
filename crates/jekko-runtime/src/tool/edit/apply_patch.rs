//! Patch application helpers.
//!
//! Ported from `packages/jekko/src/tool/edit/apply_patch.ts`. The TS
//! version implements a custom anchored-context patch format
//! (`*** Begin Patch`, `*** Add File:`, `*** Update File:`, etc.).
//! This port covers the minimal **Update File** path needed by edit-tool
//! callers — single-file textual hunks delimited by `*** Begin Patch` /
//! `*** End Patch` with `*** Update File: <path>` and a sequence of
//! `@@` hunks. New-file and delete-file cases are not yet supported.

use serde::{Deserialize, Serialize};

use crate::error::{RuntimeError, RuntimeResult};

/// Required prefix delimiting the start of a Jekko-flavoured patch envelope.
const PATCH_BEGIN_MARKER: &str = "*** Begin Patch";
/// Required suffix delimiting the end of a Jekko-flavoured patch envelope.
const PATCH_END_MARKER: &str = "*** End Patch";
/// Error message when the begin-patch marker is missing.
const MISSING_BEGIN_MSG: &str = "missing '*** Begin Patch'";
/// Error message when the end-patch marker is missing.
const MISSING_END_MSG: &str = "missing '*** End Patch'";

/// One parsed hunk inside a patch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hunk {
    /// Lines to remove (without the leading `-`).
    pub remove: Vec<String>,
    /// Lines to add (without the leading `+`).
    pub add: Vec<String>,
}

/// One parsed file inside a patch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatchFile {
    /// Target path.
    pub path: String,
    /// Hunks in order.
    pub hunks: Vec<Hunk>,
}

/// Parse a Jekko-flavoured patch.
pub fn parse_patch(text: &str) -> RuntimeResult<Vec<PatchFile>> {
    let trimmed = text.trim();
    let body = match trimmed.strip_prefix(PATCH_BEGIN_MARKER) {
        Some(rest) => rest,
        None => return Err(RuntimeError::invalid(MISSING_BEGIN_MSG.to_string())),
    };
    let body = match body.strip_suffix(PATCH_END_MARKER) {
        Some(rest) => rest,
        None => return Err(RuntimeError::invalid(MISSING_END_MSG.to_string())),
    };

    let mut files = Vec::new();
    let mut cur_path: Option<String> = None;
    let mut cur_hunks: Vec<Hunk> = Vec::new();
    let mut cur_remove: Vec<String> = Vec::new();
    let mut cur_add: Vec<String> = Vec::new();
    let mut in_hunk = false;

    let flush_hunk = |remove: &mut Vec<String>, add: &mut Vec<String>, hunks: &mut Vec<Hunk>| {
        if !remove.is_empty() || !add.is_empty() {
            hunks.push(Hunk {
                remove: std::mem::take(remove),
                add: std::mem::take(add),
            });
        }
    };

    let flush_file =
        |files: &mut Vec<PatchFile>, path: &mut Option<String>, hunks: &mut Vec<Hunk>| {
            if let Some(p) = path.take() {
                files.push(PatchFile {
                    path: p,
                    hunks: std::mem::take(hunks),
                });
            }
        };

    for line in body.lines() {
        let trimmed = line.trim_end_matches('\r');
        if let Some(p) = trimmed.strip_prefix("*** Update File: ") {
            flush_hunk(&mut cur_remove, &mut cur_add, &mut cur_hunks);
            flush_file(&mut files, &mut cur_path, &mut cur_hunks);
            cur_path = Some(p.trim().to_string());
            in_hunk = false;
        } else if trimmed.starts_with("@@") {
            flush_hunk(&mut cur_remove, &mut cur_add, &mut cur_hunks);
            in_hunk = true;
        } else if in_hunk {
            if let Some(rest) = trimmed.strip_prefix('-') {
                cur_remove.push(rest.to_string());
            } else if let Some(rest) = trimmed.strip_prefix('+') {
                cur_add.push(rest.to_string());
            }
            // Context lines (' ' prefix) are ignored in this simplified port.
        }
    }
    flush_hunk(&mut cur_remove, &mut cur_add, &mut cur_hunks);
    flush_file(&mut files, &mut cur_path, &mut cur_hunks);
    Ok(files)
}

/// Apply a parsed patch to a file's text body.
pub fn apply_hunks(body: &str, hunks: &[Hunk]) -> RuntimeResult<String> {
    let mut text = body.to_string();
    for hunk in hunks {
        let remove = hunk.remove.join("\n");
        let add = hunk.add.join("\n");
        if remove.is_empty() {
            text.push_str(&add);
            continue;
        }
        if text.matches(&remove).count() != 1 {
            return Err(RuntimeError::invalid(format!(
                "patch hunk did not match uniquely (found {})",
                text.matches(&remove).count()
            )));
        }
        text = text.replacen(&remove, &add, 1);
    }
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_single_hunk() {
        let patch = "*** Begin Patch\n\
*** Update File: /tmp/x.rs\n\
@@\n\
-before\n\
+after\n\
*** End Patch";
        let files = parse_patch(patch).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "/tmp/x.rs");
        assert_eq!(files[0].hunks[0].remove, vec!["before"]);
        assert_eq!(files[0].hunks[0].add, vec!["after"]);
    }

    #[test]
    fn applies_hunks() {
        let updated = apply_hunks(
            "let x = before;",
            &[Hunk {
                remove: vec!["before".into()],
                add: vec!["after".into()],
            }],
        )
        .unwrap();
        assert_eq!(updated, "let x = after;");
    }
}
