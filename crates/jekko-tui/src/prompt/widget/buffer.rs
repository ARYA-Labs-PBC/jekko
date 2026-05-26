//! Buffer-mutation methods on `Prompt`.
//!
//! Houses `clear`, `replace_buffer`, `handle_paste`, `submit`, and the
//! per-route stash bridge (`save_stash` / `restore_stash`). Split from the
//! parent so `widget.rs` stays under the per-file LOC budget.

use ratatui::style::Style;
use tui_textarea::TextArea;

use super::{Prompt, PromptOutcome};
use crate::prompt::paste::{PasteBuffer, PasteRecord};
use crate::prompt::stash::RouteKey;

impl Prompt {
    /// Clear the buffer and reset every popup.
    pub fn clear(&mut self) {
        self.textarea = TextArea::default();
        self.textarea.set_cursor_line_style(Style::default());
        self.textarea.set_placeholder_text(self.empty_hint.clone());
        self.slash.close();
        self.mention.close();
        self.paste.clear();
        self.history.reset_cursor();
    }

    /// Save the current buffer to the per-route stash.
    pub fn save_stash(&mut self, route: impl Into<RouteKey>) {
        let text = self.buffer_string();
        self.stash.save(route, text);
    }

    /// Restore a draft saved with [`Self::save_stash`].
    pub fn restore_stash(&mut self, route: impl Into<RouteKey>) -> bool {
        if let Some(text) = self.stash.restore(route) {
            self.replace_buffer(&text);
            true
        } else {
            false
        }
    }

    /// Replace the entire buffer with `text`.
    pub fn replace_buffer(&mut self, text: &str) {
        let lines: Vec<String> = if text.is_empty() {
            vec![String::new()]
        } else {
            text.split('\n').map(str::to_string).collect()
        };
        self.textarea = TextArea::new(lines);
        self.textarea.set_cursor_line_style(Style::default());
        self.textarea.set_placeholder_text(self.empty_hint.clone());
        self.refresh_popups();
    }

    /// Handle a pasted block of text (bracketed paste).
    pub fn handle_paste(&mut self, text: String) -> PromptOutcome {
        if text.is_empty() {
            return PromptOutcome::Consumed;
        }
        if PasteBuffer::should_collapse(&text) {
            let record: PasteRecord = self.paste.stash(text);
            let chip = record.summary();
            self.textarea.insert_str(chip);
        } else {
            // Insert verbatim, splitting on '\n' so each line lands on its own
            // row in the textarea.
            for (i, line) in text.split('\n').enumerate() {
                if i > 0 {
                    self.textarea.insert_newline();
                }
                if !line.is_empty() {
                    self.textarea.insert_str(line);
                }
            }
        }
        self.refresh_popups();
        PromptOutcome::Consumed
    }

    /// Submit the current buffer and return the expanded payload.
    pub fn submit(&mut self) -> Option<String> {
        let visible = self.buffer_string();
        if visible.trim().is_empty() {
            return None;
        }
        let expanded = self.paste.expand(&visible);
        self.history.push(visible);
        self.clear();
        Some(expanded)
    }
}
