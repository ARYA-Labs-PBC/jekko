//! In-memory prompt history with cursor-based navigation.
//!
//! Ports the essential navigation surface of
//! `packages/jekko/src/cli/cmd/tui/component/prompt/history.tsx`. Persistence
//! and the on-disk JSONL format are deferred — this layer only owns the
//! in-memory `VecDeque` and the index used by the widget.

use std::collections::VecDeque;

/// Default max retained entries — matches the TS `MAX_HISTORY_ENTRIES`.
const DEFAULT_CAPACITY: usize = 50;

/// In-memory history of submitted prompts.
#[derive(Clone, Debug)]
pub struct PromptHistory {
    entries: VecDeque<String>,
    capacity: usize,
    /// Navigation cursor, 0 == "no entry selected (live buffer)", 1 == newest.
    cursor: usize,
}

impl Default for PromptHistory {
    fn default() -> Self {
        Self::with_capacity(DEFAULT_CAPACITY)
    }
}

impl PromptHistory {
    /// Build a history bounded to `capacity` entries.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            entries: VecDeque::with_capacity(capacity.max(1)),
            capacity: capacity.max(1),
            cursor: 0,
        }
    }

    /// Append a new entry. Trims oldest entries past the capacity bound.
    pub fn push(&mut self, entry: impl Into<String>) {
        let entry = entry.into();
        if entry.is_empty() {
            return;
        }
        self.entries.push_back(entry);
        while self.entries.len() > self.capacity {
            self.entries.pop_front();
        }
        self.cursor = 0;
    }

    /// Total number of stored entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the history is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Drop every entry and reset the cursor.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.cursor = 0;
    }

    /// Move the cursor backward (toward older entries). Returns the new
    /// selection (or `None` if no entry is selected after the move).
    pub fn nav_up(&mut self) -> Option<&str> {
        if self.entries.is_empty() {
            return None;
        }
        if self.cursor < self.entries.len() {
            self.cursor += 1;
        }
        self.current()
    }

    /// Move the cursor forward (toward newer entries / the live buffer).
    /// Returns the new selection, or `None` once the cursor reaches the live
    /// buffer.
    pub fn nav_down(&mut self) -> Option<&str> {
        if self.cursor == 0 {
            return None;
        }
        self.cursor -= 1;
        self.current()
    }

    /// Reset the navigation cursor to the live buffer (no selection).
    pub fn reset_cursor(&mut self) {
        self.cursor = 0;
    }

    /// Inspect the entry the cursor is currently pointing at.
    pub fn current(&self) -> Option<&str> {
        if self.cursor == 0 {
            return None;
        }
        let idx = self
            .entries
            .len()
            .checked_sub(self.cursor)
            .expect("cursor must be ≤ history length");
        self.entries.get(idx).map(String::as_str)
    }

    /// Iterate over the entries from oldest to newest.
    pub fn iter(&self) -> impl Iterator<Item = &str> {
        self.entries.iter().map(String::as_str)
    }
}
