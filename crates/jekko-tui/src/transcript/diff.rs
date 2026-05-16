//! Unified-diff parser used by [`super::cards::ToolCard`].
//!
//! Ports `packages/jekko/src/cli/cmd/tui/util/revert-diff.ts`. The TS layer
//! uses the `diff` npm package's `parsePatch`; here we implement a small
//! hand-rolled parser that handles the unified-diff syntax we actually see
//! from `jekko-runtime`: `--- a/file` / `+++ b/file` file headers followed by
//! `@@ -l1,c1 +l2,c2 @@` hunk headers and `+` / `-` / ` ` body lines.
//!
//! The orchestrator may later replace this with the `similar` crate once
//! `Cargo.toml` is updated; the public surface (`DiffFile`, `DiffHunk`,
//! `DiffLine`, `parse_unified_diff`) is stable.

/// One file's worth of patch output.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiffFile {
    /// File name with the `a/`/`b/` prefix stripped.
    pub filename: String,
    /// Previous path (raw, including `a/` if present).
    pub previous_path: Option<String>,
    /// New path (raw, including `b/` if present).
    pub new_path: Option<String>,
    /// Number of `+` lines across all hunks.
    pub additions: usize,
    /// Number of `-` lines across all hunks.
    pub deletions: usize,
    /// Hunks for this file.
    pub hunks: Vec<DiffHunk>,
}

/// A single `@@` hunk.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiffHunk {
    /// Starting line number in the previous file.
    pub old_start: u32,
    /// Number of context+removed lines in the previous file.
    pub old_lines: u32,
    /// Starting line number in the new file.
    pub new_start: u32,
    /// Number of context+added lines in the new file.
    pub new_lines: u32,
    /// Body lines.
    pub lines: Vec<DiffLine>,
}

/// Kind of body line within a hunk.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum DiffLineKind {
    /// Added line.
    Add,
    /// Removed line.
    Del,
    /// Context line.
    Ctx,
}

/// One body line.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiffLine {
    /// Add / Del / Ctx.
    pub kind: DiffLineKind,
    /// Text content (without the leading `+`/`-`/` ` marker).
    pub text: String,
}

/// Parse a unified-diff string into one [`DiffFile`] per `--- previous / +++ new`
/// header pair. Malformed input returns an empty vector.
pub fn parse_unified_diff(text: &str) -> Vec<DiffFile> {
    if text.is_empty() {
        return Vec::new();
    }
    let mut files: Vec<DiffFile> = Vec::new();
    let mut current_file: Option<DiffFile> = None;
    let mut current_hunk: Option<DiffHunk> = None;

    for raw in text.lines() {
        if let Some(rest) = raw.strip_prefix("--- ") {
            // New file. Flush prior hunk and file.
            if let Some(hunk) = current_hunk.take() {
                if let Some(file) = current_file.as_mut() {
                    file.hunks.push(hunk);
                }
            }
            if let Some(file) = current_file.take() {
                files.push(file);
            }
            current_file = Some(new_file(rest, None));
            continue;
        }
        if let Some(rest) = raw.strip_prefix("+++ ") {
            if let Some(file) = current_file.as_mut() {
                file.new_path = Some(rest.trim().to_string());
                file.filename =
                    pick_filename(file.previous_path.as_deref(), file.new_path.as_deref());
            } else {
                current_file = Some(new_file("/dev/null", Some(rest)));
            }
            continue;
        }
        if let Some(rest) = raw.strip_prefix("@@") {
            // Flush the previous hunk first.
            if let Some(hunk) = current_hunk.take() {
                if let Some(file) = current_file.as_mut() {
                    file.hunks.push(hunk);
                }
            }
            current_hunk = parse_hunk_header(rest);
            continue;
        }
        let Some(hunk) = current_hunk.as_mut() else {
            continue;
        };
        if raw.is_empty() {
            // Treat as a context blank line.
            hunk.lines.push(DiffLine {
                kind: DiffLineKind::Ctx,
                text: String::new(),
            });
            continue;
        }
        let first = raw.as_bytes()[0];
        match first {
            b'+' => {
                hunk.lines.push(DiffLine {
                    kind: DiffLineKind::Add,
                    text: raw[1..].to_string(),
                });
                if let Some(file) = current_file.as_mut() {
                    file.additions += 1;
                }
            }
            b'-' => {
                hunk.lines.push(DiffLine {
                    kind: DiffLineKind::Del,
                    text: raw[1..].to_string(),
                });
                if let Some(file) = current_file.as_mut() {
                    file.deletions += 1;
                }
            }
            b' ' => hunk.lines.push(DiffLine {
                kind: DiffLineKind::Ctx,
                text: raw[1..].to_string(),
            }),
            b'\\' => {
                // `\ No newline at end of file` — skip silently.
            }
            _ => {}
        }
    }

    if let Some(hunk) = current_hunk.take() {
        if let Some(file) = current_file.as_mut() {
            file.hunks.push(hunk);
        }
    }
    if let Some(file) = current_file.take() {
        files.push(file);
    }
    files
}

fn new_file(previous: &str, new: Option<&str>) -> DiffFile {
    let previous_path = previous.trim().to_string();
    let new_path = new.map(|s| s.trim().to_string());
    let filename = pick_filename(Some(&previous_path), new_path.as_deref());
    DiffFile {
        filename,
        previous_path: Some(previous_path),
        new_path,
        additions: 0,
        deletions: 0,
        hunks: Vec::new(),
    }
}

fn pick_filename(previous: Option<&str>, new: Option<&str>) -> String {
    let candidate = [new, previous]
        .into_iter()
        .flatten()
        .find(|p| !p.is_empty() && *p != "/dev/null")
        .unwrap_or("unknown");
    strip_ab_prefix(candidate).to_string()
}

fn strip_ab_prefix(raw: &str) -> &str {
    if let Some(rest) = raw.strip_prefix("a/") {
        return rest;
    }
    if let Some(rest) = raw.strip_prefix("b/") {
        return rest;
    }
    raw
}

fn parse_hunk_header(rest: &str) -> Option<DiffHunk> {
    // rest looks like ` -3,7 +3,8 @@ optional context`
    let trimmed = rest.trim_start();
    let bare = trimmed.strip_suffix(" @@").unwrap_or(trimmed);
    let core = bare.split(" @@").next().unwrap_or(bare).trim();
    let mut parts = core.split_whitespace();
    let previous_part = parts.next()?;
    let new_part = parts.next()?;
    let (old_start, old_lines) =
        parse_range(previous_part.strip_prefix('-').unwrap_or(previous_part))?;
    let (new_start, new_lines) = parse_range(new_part.strip_prefix('+').unwrap_or(new_part))?;
    Some(DiffHunk {
        old_start,
        old_lines,
        new_start,
        new_lines,
        lines: Vec::new(),
    })
}

fn parse_range(text: &str) -> Option<(u32, u32)> {
    let mut parts = text.split(',');
    let start: u32 = parts.next()?.parse().ok()?;
    let count: u32 = parts.next().map(|p| p.parse().ok()).unwrap_or(Some(1))?;
    Some((start, count))
}

/// Convenience snapshot helper: returns a stable single-line summary per
/// file. Useful for `insta` assertions in callers.
pub fn summarize(diff: &[DiffFile]) -> String {
    let mut out = String::new();
    for file in diff {
        out.push_str(&format!(
            "{} +{} -{}\n",
            file.filename, file.additions, file.deletions
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "--- a/foo.txt\n+++ b/foo.txt\n@@ -1,3 +1,3 @@\n line one\n-before two\n+after two\n line three\n";

    #[test]
    fn empty_input_returns_empty() {
        let files = parse_unified_diff("");
        assert!(files.is_empty());
    }

    #[test]
    fn parses_single_file_one_hunk() {
        let files = parse_unified_diff(SAMPLE);
        assert_eq!(files.len(), 1);
        let file = &files[0];
        assert_eq!(file.filename, "foo.txt");
        assert_eq!(file.additions, 1);
        assert_eq!(file.deletions, 1);
        assert_eq!(file.hunks.len(), 1);
        assert_eq!(file.hunks[0].lines.len(), 4);
    }

    #[test]
    fn strips_ab_prefix() {
        let files = parse_unified_diff(SAMPLE);
        assert_eq!(files[0].filename, "foo.txt");
        assert!(!files[0].filename.starts_with("a/"));
    }

    #[test]
    fn handles_multiple_files() {
        let body = "--- a/x\n+++ b/x\n@@ -1,1 +1,1 @@\n-a\n+b\n--- a/y\n+++ b/y\n@@ -1,1 +1,2 @@\n c\n+d\n";
        let files = parse_unified_diff(body);
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].filename, "x");
        assert_eq!(files[1].filename, "y");
        assert_eq!(files[1].additions, 1);
    }

    #[test]
    fn handles_dev_null_for_added_file() {
        let body = "--- /dev/null\n+++ b/new.txt\n@@ -0,0 +1,2 @@\n+hello\n+world\n";
        let files = parse_unified_diff(body);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].filename, "new.txt");
        assert_eq!(files[0].additions, 2);
        assert_eq!(files[0].deletions, 0);
    }

    #[test]
    fn classifies_line_kinds() {
        let files = parse_unified_diff(SAMPLE);
        let lines = &files[0].hunks[0].lines;
        assert_eq!(lines[0].kind, DiffLineKind::Ctx);
        assert_eq!(lines[1].kind, DiffLineKind::Del);
        assert_eq!(lines[2].kind, DiffLineKind::Add);
        assert_eq!(lines[3].kind, DiffLineKind::Ctx);
        assert_eq!(lines[2].text, "after two");
    }

    #[test]
    fn parses_hunk_header_with_default_count() {
        let body = "--- a/z\n+++ b/z\n@@ -1 +1 @@\n-before\n+after\n";
        let files = parse_unified_diff(body);
        assert_eq!(files[0].hunks[0].old_start, 1);
        assert_eq!(files[0].hunks[0].old_lines, 1);
        assert_eq!(files[0].hunks[0].new_start, 1);
        assert_eq!(files[0].hunks[0].new_lines, 1);
    }

    #[test]
    fn ignores_no_newline_marker() {
        let body =
            "--- a/n\n+++ b/n\n@@ -1,1 +1,1 @@\n-before\n\\ No newline at end of file\n+after\n";
        let files = parse_unified_diff(body);
        assert_eq!(files[0].additions, 1);
        assert_eq!(files[0].deletions, 1);
    }

    #[test]
    fn summarize_produces_one_line_per_file() {
        let files = parse_unified_diff(SAMPLE);
        let s = summarize(&files);
        assert_eq!(s.lines().count(), 1);
        assert!(s.contains("foo.txt"));
        assert!(s.contains("+1"));
        assert!(s.contains("-1"));
    }

    #[test]
    fn malformed_input_does_not_panic() {
        let body = "random text\nnot a diff\n@@ broken\n";
        let _ = parse_unified_diff(body);
    }
}
