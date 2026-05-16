//! `Transcript` state container: appends, scroll math, sticky-bottom.

use crate::transcript::cards::{
    AssistantCard, AssistantPart, AssistantPartKind, ReasoningCard, SystemCard, ToolCard, UserCard,
};
use crate::transcript::permission::PermissionCard;
use crate::transcript::question::QuestionCard;

use super::entry::TranscriptEntry;
use super::scroll::{ScrollAcceleration, ScrollIntent, ScrollState};

/// Transcript state container. Append-only on the entry side; scroll state is
/// mutable. Owns no rendering — see [`crate::transcript::route::SessionRoute`]
/// for the canonical composer.
#[derive(Clone, Debug, Default)]
pub struct Transcript {
    entries: Vec<TranscriptEntry>,
    state: ScrollState,
    accel_up: ScrollAcceleration,
    accel_down: ScrollAcceleration,
}

impl Transcript {
    /// Construct an empty transcript pinned to the bottom.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the entries slice in chronological order.
    pub fn entries(&self) -> &[TranscriptEntry] {
        &self.entries
    }

    /// Number of stored entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// True when there are no entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Current scroll offset.
    pub fn scroll_offset(&self) -> u16 {
        self.state.offset
    }

    /// Sticky-bottom flag (true means new appends auto-scroll).
    pub fn is_sticky_bottom(&self) -> bool {
        self.state.sticky_bottom
    }

    /// Update the viewport height tracker (used for paging math).
    pub fn set_viewport_rows(&mut self, rows: u16) {
        self.state.viewport_rows = rows;
    }

    /// Total row estimate across all entries.
    pub fn total_rows(&self) -> u32 {
        self.entries
            .iter()
            .map(|e| u32::from(e.estimated_rows()))
            .sum()
    }

    /// Largest valid scroll offset for the given viewport.
    pub fn max_offset(&self) -> u16 {
        let total = self.total_rows();
        let viewport = u32::from(self.state.viewport_rows);
        total.saturating_sub(viewport).min(u32::from(u16::MAX)) as u16
    }

    /// Push a user card. New entries respect the sticky-bottom rule.
    pub fn push_user(&mut self, card: UserCard) {
        self.push_entry(TranscriptEntry::User(card));
    }

    /// Push an assistant card.
    pub fn push_assistant(&mut self, card: AssistantCard) {
        self.push_entry(TranscriptEntry::Assistant(card));
    }

    /// Clear the pending/spinner state on the last assistant card. Used by
    /// the app on `AssistantCompleted` and `AssistantFailed` to stop the
    /// animation once the request has resolved (success or error).
    pub fn clear_pending_on_last_assistant(&mut self) {
        if let Some(TranscriptEntry::Assistant(card)) = self.entries.last_mut() {
            card.mark_streaming();
        }
    }

    /// Push a tool card.
    pub fn push_tool(&mut self, card: ToolCard) {
        self.push_entry(TranscriptEntry::Tool(card));
    }

    /// Push a standalone reasoning card.
    pub fn push_reasoning(&mut self, card: ReasoningCard) {
        self.push_entry(TranscriptEntry::Reasoning(card));
    }

    /// Start a live-streaming reasoning card (empty, marked as streaming).
    pub fn push_reasoning_start(&mut self) {
        let card = ReasoningCard::new_streaming();
        self.push_entry(TranscriptEntry::Reasoning(card));
    }

    /// Append text into the last reasoning card. If the last entry isn't a
    /// reasoning card, creates a new streaming one first.
    pub fn append_to_last_reasoning(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        match self.entries.last_mut() {
            Some(TranscriptEntry::Reasoning(card)) => {
                card.append(text);
            }
            _ => {
                let mut card = ReasoningCard::new_streaming();
                card.append(text);
                self.push_entry(TranscriptEntry::Reasoning(card));
            }
        }
        if self.state.sticky_bottom {
            self.state.offset = self.max_offset();
        }
    }

    /// Mark the last reasoning card as finalized (no longer streaming).
    pub fn finalize_reasoning(&mut self) {
        if let Some(TranscriptEntry::Reasoning(card)) = self.entries.last_mut() {
            card.mark_complete();
        }
    }

    /// Push a system status card.
    pub fn push_system(&mut self, card: SystemCard) {
        self.push_entry(TranscriptEntry::System(card));
    }

    /// Push an inline permission request.
    pub fn push_permission(&mut self, card: PermissionCard) {
        self.push_entry(TranscriptEntry::Permission(card));
    }

    /// Push an inline question request.
    pub fn push_question(&mut self, card: QuestionCard) {
        self.push_entry(TranscriptEntry::Question(card));
    }

    /// Remove and return the last entry. Useful when the caller wants to
    /// replace a streaming card with its completed form.
    pub fn pop(&mut self) -> Option<TranscriptEntry> {
        self.entries.pop()
    }

    /// Replace the last entry. Returns the previous occupant.
    pub fn replace_last(&mut self, entry: TranscriptEntry) -> Option<TranscriptEntry> {
        let prev = self.entries.pop();
        self.entries.push(entry);
        if self.state.sticky_bottom {
            self.state.offset = self.max_offset();
        }
        prev
    }

    /// Append text into the last assistant card's last text part. If the
    /// trailing entry isn't an assistant card (or the transcript is empty),
    /// pushes a fresh assistant card with the text. Used by streaming
    /// `RuntimeEvent::AssistantTextDelta` events to incrementally fill the
    /// assistant reply without re-rendering the full transcript.
    pub fn append_to_last_assistant(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        match self.entries.last_mut() {
            Some(TranscriptEntry::Assistant(card)) => {
                // First delta — drop the spinner so it doesn't clobber the
                // streamed text.
                card.mark_streaming();
                match card
                    .parts
                    .iter_mut()
                    .rev()
                    .find(|p| matches!(p.kind, AssistantPartKind::Text))
                {
                    Some(part) => {
                        // Strip leading whitespace on the very first chunk so
                        // models that prefix replies with "\n\n" don't waste
                        // visible rows before the content starts.
                        let chunk = if part.text.is_empty() {
                            text.trim_start()
                        } else {
                            text
                        };
                        part.text.push_str(chunk);
                    }
                    None => {
                        card.parts.push(AssistantPart::new(
                            AssistantPartKind::Text,
                            text.trim_start().to_string(),
                        ));
                    }
                }
            }
            _ => {
                let card = AssistantCard::new(vec![AssistantPart::new(
                    AssistantPartKind::Text,
                    text.trim_start().to_string(),
                )]);
                self.entries.push(TranscriptEntry::Assistant(card));
            }
        }
        if self.state.sticky_bottom {
            self.state.offset = self.max_offset();
        }
    }

    /// Iterates through the text parts of the last assistant card and removes
    /// any lines that are empty or contain only whitespace to maximize vertical
    /// screen space, while preserving trailing newlines for streaming chunks.
    pub fn collapse_last_assistant_newlines(&mut self) {
        if let Some(TranscriptEntry::Assistant(card)) = self.entries.last_mut() {
            for part in card.parts.iter_mut() {
                if matches!(part.kind, AssistantPartKind::Text) {
                    let lines: Vec<&str> = part.text.split('\n').collect();
                    let mut cleaned = String::with_capacity(part.text.len());
                    let mut first = true;

                    for (i, line) in lines.iter().enumerate() {
                        let is_last = i == lines.len() - 1;
                        // Keep the line if it has visible text, OR if it's the very last
                        // element (representing a trailing \n from the stream)
                        if !line.trim().is_empty() || (is_last && line.is_empty()) {
                            if !first {
                                cleaned.push('\n');
                            }
                            cleaned.push_str(line);
                            first = false;
                        }
                    }
                    part.text = cleaned;
                }
            }
        }
    }

    fn push_entry(&mut self, entry: TranscriptEntry) {
        self.entries.push(entry);
        if self.state.sticky_bottom {
            self.state.offset = self.max_offset();
        }
    }

    /// Scroll up by `delta` rows. Disables sticky-bottom whenever the user
    /// moves away from the tail.
    pub fn scroll_up(&mut self, delta: u16) {
        self.state.offset = self.state.offset.saturating_sub(delta);
        if self.state.offset < self.max_offset() {
            self.state.sticky_bottom = false;
        }
    }

    /// Scroll down by `delta` rows. Re-engages sticky-bottom when the user
    /// catches up to the tail.
    pub fn scroll_down(&mut self, delta: u16) {
        let next = self
            .state
            .offset
            .saturating_add(delta)
            .min(self.max_offset());
        self.state.offset = next;
        if next >= self.max_offset() {
            self.state.sticky_bottom = true;
        }
    }

    /// Jump up by one viewport page.
    pub fn page_up(&mut self) {
        let step = self.state.viewport_rows.max(1);
        self.scroll_up(step);
    }

    /// Jump down by one viewport page.
    pub fn page_down(&mut self) {
        let step = self.state.viewport_rows.max(1);
        self.scroll_down(step);
    }

    /// Jump to the top of the transcript.
    pub fn top(&mut self) {
        self.state.offset = 0;
        self.state.sticky_bottom = false;
    }

    /// Jump to the bottom of the transcript and re-engage sticky-bottom.
    pub fn bottom(&mut self) {
        self.state.offset = self.max_offset();
        self.state.sticky_bottom = true;
    }

    /// Run one acceleration tick in the requested direction and apply the
    /// resulting step to the scroll offset. Returns the velocity used.
    pub fn accelerated_scroll(&mut self, intent: ScrollIntent) -> u16 {
        let velocity = match intent {
            ScrollIntent::Up => {
                self.accel_down.reset();
                self.accel_up.tick()
            }
            ScrollIntent::Down => {
                self.accel_up.reset();
                self.accel_down.tick()
            }
        };
        match intent {
            ScrollIntent::Up => self.scroll_up(velocity),
            ScrollIntent::Down => self.scroll_down(velocity),
        }
        velocity
    }

    /// Clear the buffer. Sticky-bottom remains enabled.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.state = ScrollState::default();
    }

    /// Snapshot suitable for `insta::assert_snapshot!`.
    pub fn snapshot(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "transcript: offset={} sticky={} entries={}\n",
            self.state.offset,
            self.state.sticky_bottom,
            self.entries.len()
        ));
        for (i, entry) in self.entries.iter().enumerate() {
            out.push_str(&format!(
                "  [{}] {} ~{} rows\n",
                i,
                entry.kind_label(),
                entry.estimated_rows()
            ));
        }
        out
    }
}
