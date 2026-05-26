//! Key-event handling for the `Prompt` widget.
//!
//! Owns the `handle_key` dispatch loop along with the popup-first routing
//! helpers (`handle_slash_key`, `handle_mention_key`), the history-nav
//! heuristic, popup refresh, and the `crossterm` → `tui_textarea::Input`
//! adapter. Kept in a sibling module so `widget.rs` stays under the LOC budget.
//!
//! Methods live in a separate `impl Prompt` block; the child module sees the
//! parent's private fields by virtue of Rust's privacy rules (children see
//! their parent's privates).

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use tui_textarea::{CursorMove, Input, Key};

use super::{Prompt, PromptOutcome};

impl Prompt {
    /// Dispatch one key event. Returns what the host should do.
    pub fn handle_key(&mut self, key: KeyEvent) -> PromptOutcome {
        if key.kind == KeyEventKind::Release {
            return PromptOutcome::Passthrough;
        }

        // Popup-first routing.
        if self.slash.is_open() {
            if let Some(outcome) = self.handle_slash_key(key) {
                return outcome;
            }
        }
        if self.mention.is_open() {
            if let Some(outcome) = self.handle_mention_key(key) {
                return outcome;
            }
        }

        match (key.code, key.modifiers) {
            (KeyCode::Char('c'), m) if m.contains(KeyModifiers::CONTROL) => {
                self.clear();
                return PromptOutcome::ClearRequested;
            }
            (KeyCode::Char('v'), m) if m.contains(KeyModifiers::CONTROL) => {
                return PromptOutcome::PasteRequested;
            }
            (KeyCode::Enter, m)
                if m.contains(KeyModifiers::SHIFT)
                    || m.contains(KeyModifiers::ALT)
                    || m.contains(KeyModifiers::CONTROL) =>
            {
                self.textarea.insert_newline();
                self.refresh_popups();
                return PromptOutcome::Consumed;
            }
            (KeyCode::Enter, _) => {
                return PromptOutcome::Submit;
            }
            (KeyCode::Char('a'), m) if m == KeyModifiers::CONTROL => {
                self.textarea.move_cursor(CursorMove::Head);
                return PromptOutcome::Consumed;
            }
            (KeyCode::Char('e'), m) if m == KeyModifiers::CONTROL => {
                self.textarea.move_cursor(CursorMove::End);
                return PromptOutcome::Consumed;
            }
            (KeyCode::Up, _) => {
                if self.should_history_nav() {
                    if let Some(prev) = self.history.nav_up() {
                        let prev = prev.to_string();
                        self.replace_buffer(&prev);
                        return PromptOutcome::Consumed;
                    }
                }
                self.textarea.move_cursor(CursorMove::Up);
                return PromptOutcome::Consumed;
            }
            (KeyCode::Down, _) => {
                if self.should_history_nav() {
                    match self.history.nav_down() {
                        Some(next) => {
                            let next = next.to_string();
                            self.replace_buffer(&next);
                        }
                        None => self.replace_buffer(""),
                    }
                    return PromptOutcome::Consumed;
                }
                self.textarea.move_cursor(CursorMove::Down);
                return PromptOutcome::Consumed;
            }
            _ => {}
        }

        // Fall through to tui-textarea for normal editing keys.
        let consumed = self.textarea.input(crossterm_to_input(key));
        self.refresh_popups();
        if consumed {
            PromptOutcome::Consumed
        } else {
            PromptOutcome::Passthrough
        }
    }

    fn handle_slash_key(&mut self, key: KeyEvent) -> Option<PromptOutcome> {
        match (key.code, key.modifiers) {
            (KeyCode::Esc, _) => {
                self.slash.close();
                Some(PromptOutcome::PopupCancelled)
            }
            (KeyCode::Up, _) => {
                self.slash.move_cursor(-1);
                Some(PromptOutcome::Consumed)
            }
            (KeyCode::Down, _) => {
                self.slash.move_cursor(1);
                Some(PromptOutcome::Consumed)
            }
            (KeyCode::Enter, m) if m.is_empty() => {
                let selected = self.slash.selected();
                if let Some(cmd) = selected {
                    // Wipe the trigger from the buffer so the host inserts the
                    // canonical command via the returned outcome.
                    self.replace_buffer("");
                    self.slash.close();
                    Some(PromptOutcome::SlashSelected(cmd))
                } else {
                    Some(PromptOutcome::Consumed)
                }
            }
            _ => None,
        }
    }

    fn handle_mention_key(&mut self, key: KeyEvent) -> Option<PromptOutcome> {
        match (key.code, key.modifiers) {
            (KeyCode::Esc, _) => {
                self.mention.close();
                Some(PromptOutcome::PopupCancelled)
            }
            (KeyCode::Up, _) => {
                self.mention.move_cursor(-1);
                Some(PromptOutcome::Consumed)
            }
            (KeyCode::Down, _) => {
                self.mention.move_cursor(1);
                Some(PromptOutcome::Consumed)
            }
            (KeyCode::Enter, m) if m.is_empty() => {
                let selected = self.mention.selected();
                if let Some(candidate) = selected {
                    self.mention.close();
                    Some(PromptOutcome::MentionSelected(candidate))
                } else {
                    Some(PromptOutcome::Consumed)
                }
            }
            _ => None,
        }
    }

    fn should_history_nav(&self) -> bool {
        // Walk history whenever the buffer is empty *or* the cursor is already
        // inside a recalled entry, *or* the buffer is single-line (so the user
        // can browse without committing to "open editor" mode).
        if self.history.current().is_some() {
            return true;
        }
        let lines = self.textarea.lines();
        lines.len() <= 1
    }

    pub(super) fn refresh_popups(&mut self) {
        let buffer = self.buffer_string();

        // Slash popup driven by the first column of the first line.
        let first_line = buffer.split('\n').next().unwrap_or("");
        if crate::prompt::slash::buffer_triggers_slash(first_line) {
            if !self.slash.is_open() {
                self.slash.open();
            }
            self.slash
                .set_query(crate::prompt::slash::query_from_buffer(first_line));
        } else if self.slash.is_open() {
            self.slash.close();
        }

        // Mention popup driven by the most recent `@…` token (no whitespace
        // after the `@`).
        if let Some(off) = buffer.rfind('@') {
            let tail = &buffer[off + 1..];
            if !tail.contains(char::is_whitespace) {
                if !self.mention.is_open() {
                    self.mention.open(off);
                }
                self.mention.set_query(tail);
            } else if self.mention.is_open() {
                self.mention.close();
            }
        } else if self.mention.is_open() {
            self.mention.close();
        }
    }
}

pub(super) fn crossterm_to_input(key: KeyEvent) -> Input {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let alt = key.modifiers.contains(KeyModifiers::ALT);
    let shift = key.modifiers.contains(KeyModifiers::SHIFT);
    let k = match key.code {
        KeyCode::Char(c) => Key::Char(c),
        KeyCode::Backspace => Key::Backspace,
        KeyCode::Enter => Key::Enter,
        KeyCode::Left => Key::Left,
        KeyCode::Right => Key::Right,
        KeyCode::Up => Key::Up,
        KeyCode::Down => Key::Down,
        KeyCode::Tab => Key::Tab,
        KeyCode::Delete => Key::Delete,
        KeyCode::Home => Key::Home,
        KeyCode::End => Key::End,
        KeyCode::PageUp => Key::PageUp,
        KeyCode::PageDown => Key::PageDown,
        KeyCode::Esc => Key::Esc,
        KeyCode::F(n) => Key::F(n),
        _ => Key::Null,
    };
    Input {
        key: k,
        ctrl,
        alt,
        shift,
    }
}
