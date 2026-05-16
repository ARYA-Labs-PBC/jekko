//! The composite `Prompt` widget.
//!
//! Glues together the textarea, slash + mention popups, paste buffer, history,
//! frecency and per-route stash. Exposes the small set of public methods the
//! TUI loop drives (`handle_key`, `handle_paste`, `submit`, …).

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::Widget;
use tui_textarea::{CursorMove, Input, Key, TextArea};

use super::frecency::Frecency;
use super::history::PromptHistory;
use super::mentions::{MentionCandidate, MentionPopup};
use super::paste::{PasteBuffer, PasteRecord};
use super::slash::{SlashCommand, SlashPopup};
use super::stash::{PromptStash, RouteKey};

/// What a key handler decided about the input. Surfaced so the host loop can
/// react (e.g. translate `Submit` into an action).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PromptOutcome {
    /// The widget consumed the event and updated its internal state.
    Consumed,
    /// The widget consumed the event by submitting (caller should drain
    /// the prompt via [`Prompt::submit`]).
    Submit,
    /// The user pressed Ctrl+C — caller may clear the buffer or interpret it
    /// as a higher-level quit if the buffer was already empty.
    ClearRequested,
    /// The user requested an explicit paste (`Ctrl+V`); the host wires
    /// clipboard glue here.
    PasteRequested,
    /// The user picked a slash command — host dispatches the action.
    SlashSelected(SlashCommand),
    /// The user picked a `@` mention — host inserts the resolved reference.
    MentionSelected(MentionCandidate),
    /// The user cancelled an open popup with `Esc`.
    PopupCancelled,
    /// The event was not handled (e.g. an unknown key).
    Passthrough,
}

/// Snapshot of the prompt's externally observable state.
#[derive(Clone, Debug)]
pub struct PromptSnapshot {
    /// Visible buffer text (what the user sees on screen, paste chips included).
    pub visible: String,
    /// Fully expanded text (paste chips replaced with their original payload).
    pub expanded: String,
    /// Total visual rows after wrapping.
    pub line_count: usize,
    /// Cursor position (row, column) in the textarea.
    pub cursor: (usize, usize),
    /// True if the slash popup is open.
    pub slash_open: bool,
    /// True if the mention popup is open.
    pub mention_open: bool,
}

/// Composite prompt widget.
pub struct Prompt {
    textarea: TextArea<'static>,
    history: PromptHistory,
    frecency: Frecency,
    stash: PromptStash,
    slash: SlashPopup,
    mention: MentionPopup,
    paste: PasteBuffer,
    /// Optional hint shown when the buffer is empty.
    empty_hint: String,
    /// Right-aligned label (e.g. model name).
    model_label: Option<String>,
}

impl std::fmt::Debug for Prompt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Prompt")
            .field("buffer", &self.buffer_string())
            .field("slash_open", &self.slash.is_open())
            .field("mention_open", &self.mention.is_open())
            .field("paste_records", &self.paste.records().len())
            .field("history_len", &self.history.len())
            .finish()
    }
}

impl Default for Prompt {
    fn default() -> Self {
        Self::new()
    }
}

impl Prompt {
    /// Construct an empty prompt.
    pub fn new() -> Self {
        let mut textarea = TextArea::default();
        textarea.set_cursor_line_style(Style::default());
        textarea.set_placeholder_text("Ask Jekko…");
        Self {
            textarea,
            history: PromptHistory::default(),
            frecency: Frecency::new(),
            stash: PromptStash::new(),
            slash: SlashPopup::default(),
            mention: MentionPopup::default(),
            paste: PasteBuffer::new(),
            empty_hint: "Ask Jekko…".to_string(),
            model_label: None,
        }
    }

    /// Replace the slash catalog (used by the command-palette host).
    pub fn set_slash_catalog(&mut self, catalog: Vec<SlashCommand>) {
        self.slash.set_catalog(catalog);
    }

    /// Read access to the slash popup state — used by the host to render the
    /// overlay widget above the composer panel.
    pub fn slash_popup(&self) -> &SlashPopup {
        &self.slash
    }

    /// Replace the mention candidate list.
    pub fn set_mention_candidates(&mut self, candidates: Vec<MentionCandidate>) {
        self.mention.set_candidates(candidates);
    }

    /// Override the empty-buffer hint text.
    pub fn set_empty_hint(&mut self, hint: impl Into<String>) {
        let hint = hint.into();
        self.empty_hint = hint.clone();
        self.textarea.set_placeholder_text(hint);
    }

    /// Set the right-aligned model label (e.g. `"claude-opus-4-7"`).
    pub fn set_model_label(&mut self, label: impl Into<String>) {
        self.model_label = Some(label.into());
    }

    /// Read access to the embedded textarea (for tests/host inspection).
    pub fn textarea(&self) -> &TextArea<'static> {
        &self.textarea
    }

    /// Read access to the history.
    pub fn history(&self) -> &PromptHistory {
        &self.history
    }

    /// Read access to the frecency table.
    pub fn frecency(&self) -> &Frecency {
        &self.frecency
    }

    /// Read access to the stash.
    pub fn stash(&self) -> &PromptStash {
        &self.stash
    }

    /// Mutable access to the stash so callers can persist drafts on route
    /// transitions.
    pub fn stash_mut(&mut self) -> &mut PromptStash {
        &mut self.stash
    }

    /// Read access to the paste buffer.
    pub fn paste_buffer(&self) -> &PasteBuffer {
        &self.paste
    }

    /// Read access to the slash popup.
    pub fn slash(&self) -> &SlashPopup {
        &self.slash
    }

    /// Read access to the mention popup.
    pub fn mention(&self) -> &MentionPopup {
        &self.mention
    }

    /// True if the slash popup is currently visible.
    pub fn is_slash_open(&self) -> bool {
        self.slash.is_open()
    }

    /// True if the mention popup is currently visible.
    pub fn is_mention_open(&self) -> bool {
        self.mention.is_open()
    }

    /// Visible buffer joined into a single string with `\n` separators.
    pub fn buffer_string(&self) -> String {
        self.textarea.lines().join("\n")
    }

    /// Total character count of the visible buffer (used by Composer status).
    pub fn buffer_char_count(&self) -> usize {
        self.textarea
            .lines()
            .iter()
            .map(|l| l.chars().count())
            .sum()
    }

    /// Fully expanded buffer (paste chips substituted back in).
    pub fn expanded_buffer(&self) -> String {
        self.paste.expand(&self.buffer_string())
    }

    /// Snapshot the externally observable state.
    pub fn snapshot(&self) -> PromptSnapshot {
        let visible = self.buffer_string();
        let expanded = self.paste.expand(&visible);
        PromptSnapshot {
            line_count: self.textarea.lines().len(),
            cursor: self.textarea.cursor(),
            slash_open: self.slash.is_open(),
            mention_open: self.mention.is_open(),
            visible,
            expanded,
        }
    }

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

    fn refresh_popups(&mut self) {
        let buffer = self.buffer_string();

        // Slash popup driven by the first column of the first line.
        let first_line = buffer.split('\n').next().unwrap_or("");
        if super::slash::buffer_triggers_slash(first_line) {
            if !self.slash.is_open() {
                self.slash.open();
            }
            self.slash
                .set_query(super::slash::query_from_buffer(first_line));
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

fn crossterm_to_input(key: KeyEvent) -> Input {
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

impl Widget for &Prompt {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Render the textarea directly — no internal block. The outer
        // panel_block in draw_shell_body provides the chrome already.
        Widget::render(&self.textarea, area, buf);
        if let Some(label) = &self.model_label {
            if area.width == 0 || area.height == 0 {
                return;
            }
            let label_width = label.chars().count() as u16;
            if label_width >= area.width {
                return;
            }
            let x = area.x + area.width - label_width;
            buf.set_string(x, area.y, label, Style::default());
        }
    }
}
