//! Bracketed-paste accumulator with inline summary chips.
//!
//! When the user pastes a block of text larger than a threshold (lines or
//! characters), the visible prompt collapses to a `[paste: N lines, Sz]` chip
//! and the actual content is stashed in a side buffer. On submit, the host
//! re-expands every chip back into the outgoing payload.

use std::fmt;

/// Minimum line count for a paste to be replaced with a summary chip.
pub const PASTE_LINE_THRESHOLD: usize = 8;
/// Minimum byte length for a paste to be replaced with a summary chip.
pub const PASTE_BYTE_THRESHOLD: usize = 280;

/// One stashed paste. `id` is the chip token inserted into the visible
/// buffer; `content` holds the original text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PasteRecord {
    /// Monotonic stash identifier (used inside the chip summary text).
    pub id: u64,
    /// Original pasted content (verbatim — no normalization).
    pub content: String,
    /// Cached line count.
    pub line_count: usize,
    /// Cached byte length.
    pub byte_len: usize,
}

impl PasteRecord {
    /// Render the summary string used as the inline chip.
    pub fn summary(&self) -> String {
        format!(
            "[paste #{id}: {lines} lines, {bytes}]",
            id = self.id,
            lines = self.line_count,
            bytes = HumanBytes(self.byte_len)
        )
    }
}

/// Side buffer that stores every paste replaced with an inline chip.
#[derive(Clone, Debug, Default)]
pub struct PasteBuffer {
    records: Vec<PasteRecord>,
    next_id: u64,
}

impl PasteBuffer {
    /// Build an empty buffer.
    pub fn new() -> Self {
        Self::default()
    }

    /// Stash `content` and return the new record. Counts lines and bytes once.
    pub fn stash(&mut self, content: impl Into<String>) -> PasteRecord {
        let content = content.into();
        let line_count = if content.is_empty() {
            0
        } else {
            content.matches('\n').count() + 1
        };
        let byte_len = content.len();
        self.next_id += 1;
        let record = PasteRecord {
            id: self.next_id,
            content,
            line_count,
            byte_len,
        };
        self.records.push(record.clone());
        record
    }

    /// Return every stashed record in insertion order.
    pub fn records(&self) -> &[PasteRecord] {
        &self.records
    }

    /// Look up a record by id.
    pub fn get(&self, id: u64) -> Option<&PasteRecord> {
        self.records.iter().find(|r| r.id == id)
    }

    /// Drop every record.
    pub fn clear(&mut self) {
        self.records.clear();
    }

    /// True if the paste content meets the threshold for chip collapsing.
    pub fn should_collapse(content: &str) -> bool {
        let lines = content.matches('\n').count() + 1;
        lines >= PASTE_LINE_THRESHOLD || content.len() >= PASTE_BYTE_THRESHOLD
    }

    /// Expand every chip summary in `visible` back to its content.
    pub fn expand(&self, visible: &str) -> String {
        if self.records.is_empty() {
            return visible.to_string();
        }
        let mut out = visible.to_string();
        for record in &self.records {
            let chip = record.summary();
            if let Some(pos) = out.find(&chip) {
                out.replace_range(pos..pos + chip.len(), &record.content);
            }
        }
        out
    }
}

/// Tiny humanized byte display (B / KB / MB) shared with the chip renderer.
struct HumanBytes(usize);

impl fmt::Display for HumanBytes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let n = self.0;
        if n < 1024 {
            return write!(f, "{n}B");
        }
        if n < 1024 * 1024 {
            let kb = n as f64 / 1024.0;
            return write!(f, "{kb:.1}KB");
        }
        let mb = n as f64 / (1024.0 * 1024.0);
        write!(f, "{mb:.1}MB")
    }
}
