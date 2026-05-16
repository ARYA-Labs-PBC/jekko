//! `@`-mention popup with fuzzy file matching.
//!
//! The host crate supplies the candidate `Vec<PathBuf>` — this module does no
//! filesystem I/O. Matching is the same simple subsequence fuzzy match used by
//! `dialog::command::CommandPalette`, scored by:
//!
//! * exact prefix > substring match > subsequence match,
//! * tie-broken by candidate length (shorter wins).

use std::path::PathBuf;

/// One row in the mention popup.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MentionCandidate {
    /// Full path. Displayed verbatim (or relativized by the caller).
    pub path: PathBuf,
    /// Optional human label override.
    pub label: Option<String>,
}

impl MentionCandidate {
    /// Construct a candidate from a path.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            label: None,
        }
    }

    /// Display string the popup renders.
    pub fn display(&self) -> String {
        if let Some(label) = self.label.as_deref() {
            return label.to_string();
        }
        self.path.to_string_lossy().to_string()
    }
}

/// Mention popup state.
#[derive(Clone, Debug)]
pub struct MentionPopup {
    candidates: Vec<MentionCandidate>,
    query: String,
    cursor: usize,
    open: bool,
    /// Byte offset of the triggering `@` inside the prompt buffer. Tracked so
    /// the host can splice the chosen mention back in.
    trigger_offset: Option<usize>,
}

impl Default for MentionPopup {
    fn default() -> Self {
        Self::with_candidates(Vec::new())
    }
}

impl MentionPopup {
    /// Build a popup over a list of file candidates.
    pub fn with_candidates(candidates: Vec<MentionCandidate>) -> Self {
        Self {
            candidates,
            query: String::new(),
            cursor: 0,
            open: false,
            trigger_offset: None,
        }
    }

    /// Replace the candidate list (callers may refresh it as the workspace
    /// changes).
    pub fn set_candidates(&mut self, candidates: Vec<MentionCandidate>) {
        self.candidates = candidates;
        self.cursor = 0;
    }

    /// Whether the popup is currently visible.
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Open the popup. `trigger_offset` is the byte position of the `@`.
    pub fn open(&mut self, trigger_offset: usize) {
        self.open = true;
        self.query.clear();
        self.cursor = 0;
        self.trigger_offset = Some(trigger_offset);
    }

    /// Close and forget the trigger anchor.
    pub fn close(&mut self) {
        self.open = false;
        self.query.clear();
        self.cursor = 0;
        self.trigger_offset = None;
    }

    /// Update the query.
    pub fn set_query(&mut self, query: impl Into<String>) {
        self.query = query.into();
        self.cursor = 0;
    }

    /// Current query string.
    pub fn query(&self) -> &str {
        &self.query
    }

    /// Trigger offset within the prompt buffer (if any).
    pub fn trigger_offset(&self) -> Option<usize> {
        self.trigger_offset
    }

    /// Cursor inside the filtered list.
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// Move the cursor by `delta`, wrapping around the filtered list.
    pub fn move_cursor(&mut self, delta: isize) {
        let len = self.filtered().len();
        if len == 0 {
            self.cursor = 0;
            return;
        }
        let len_i = len as isize;
        let mut idx = self.cursor as isize + delta;
        while idx < 0 {
            idx += len_i;
        }
        self.cursor = (idx % len_i) as usize;
    }

    /// Filtered candidate list sorted by score (best first).
    pub fn filtered(&self) -> Vec<MentionCandidate> {
        if self.query.is_empty() {
            return self.candidates.clone();
        }
        let lowered = self.query.to_lowercase();
        let mut scored: Vec<(i64, MentionCandidate)> = self
            .candidates
            .iter()
            .filter_map(|c| score(&c.display().to_lowercase(), &lowered).map(|s| (s, c.clone())))
            .collect();
        scored.sort_by_key(|(score, _)| std::cmp::Reverse(*score));
        scored.into_iter().map(|(_, c)| c).collect()
    }

    /// Currently selected candidate.
    pub fn selected(&self) -> Option<MentionCandidate> {
        self.filtered().get(self.cursor).cloned()
    }
}

fn score(haystack: &str, needle: &str) -> Option<i64> {
    if needle.is_empty() {
        return Some(0);
    }
    if haystack.starts_with(needle) {
        return Some(1_000 - (haystack.len() as i64));
    }
    if haystack.contains(needle) {
        return Some(500 - (haystack.len() as i64));
    }
    let mut hay_chars = haystack.chars();
    let mut matched = 0i64;
    for nc in needle.chars() {
        if let Some(pos) = hay_chars.by_ref().position(|hc| hc == nc) {
            matched += 1;
            // Reward consecutive matches by subtracting the gap.
            matched -= pos as i64;
        } else {
            return None;
        }
    }
    Some(matched)
}
