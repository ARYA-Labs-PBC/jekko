//! The composite `Prompt` widget.
//!
//! Glues together the textarea, slash + mention popups, paste buffer, history,
//! frecency and per-route stash. Exposes the small set of public methods the
//! TUI loop drives (`handle_key`, `handle_paste`, `submit`, …).

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Style, Stylize};
use ratatui::widgets::Widget;
use tui_textarea::{CursorMove, Input, Key, TextArea};

use super::frecency::Frecency;
use super::history::PromptHistory;
use super::mentions::{MentionCandidate, MentionPopup};
use super::paste::{PasteBuffer, PasteRecord};
use super::slash::{SlashCommand, SlashPopup};
use super::stash::{PromptStash, RouteKey};
use crate::glyph_set;
use crate::theme::codex::BLUE_PATH;

/// The `›` glyph (U+203A) painted at column 0 of the composer's first row.
/// Single source of truth so tests can reference the same constant.
///
/// NOTE: this `pub const` is the Unicode literal kept for back-compat with
/// existing callers (tests + a couple of internal modules). New code should
/// prefer [`prompt_glyph()`] which honors the active `GlyphMode` (Unicode vs
/// ASCII) when `JEKKO_ASCII=1` / `LC_ALL=C` is set.
pub const PROMPT_GLYPH: &str = "›";

/// Accessibility-aware composer prompt prefix glyph.
///
/// Returns `›` (Unicode) when the active `GlyphMode` is `Unicode`, or `>`
/// (ASCII) when monochrome / ASCII fallback is in effect. Prefer this over
/// the [`PROMPT_GLYPH`] const for any new render code.
pub fn prompt_glyph() -> &'static str {
    glyph_set::current().composer_prefix
}

/// Empty-prompt placeholder hint (`"Ask Jekko<ellipsis>"`). The trailing
/// ellipsis honors the active `GlyphMode` (`"…"` Unicode vs `"..."` ASCII).
fn empty_prompt_hint() -> String {
    format!("Ask Jekko{}", glyph_set::current().ellipsis)
}

/// Columns reserved at the left edge for the prompt prefix.
///
/// Layout is `›·` (glyph + one blank space), then the textarea body starts at
/// column 2. Continuation rows (wrapped or Shift+Enter newlines) leave both
/// columns blank so body text stays vertically aligned under the first
/// character of the prompt.
pub const PROMPT_PREFIX_WIDTH: u16 = 2;

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
        let hint = empty_prompt_hint();
        textarea.set_placeholder_text(hint.clone());
        Self {
            textarea,
            history: PromptHistory::default(),
            frecency: Frecency::new(),
            stash: PromptStash::new(),
            slash: SlashPopup::default(),
            mention: MentionPopup::default(),
            paste: PasteBuffer::new(),
            empty_hint: hint,
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
        if area.width == 0 || area.height == 0 {
            return;
        }

        // Paint the `›` prefix gutter (column 0) + space (column 1) on every
        // text row. The first row gets the blue `›` glyph; continuation rows
        // get blank columns so wrapped/multi-line text aligns under the first
        // body character.
        //
        // We only paint into the gutter if the area is wide enough to leave
        // at least one body column. Below that, fall back to the legacy
        // gutter-less render so we don't lose all editable space on a 1-col
        // sliver.
        if area.width <= PROMPT_PREFIX_WIDTH {
            Widget::render(&self.textarea, area, buf);
            self.render_model_label(area, buf);
            return;
        }

        let prefix_style = Style::default().fg(BLUE_PATH);
        let blank_style = Style::default();
        let glyph = prompt_glyph();
        for row_offset in 0..area.height {
            let y = area.y + row_offset;
            if row_offset == 0 {
                buf.set_string(area.x, y, glyph, prefix_style);
                buf.set_string(area.x + 1, y, " ", blank_style);
            } else {
                buf.set_string(area.x, y, "  ", blank_style);
            }
        }

        // Render the textarea into the inner area (shifted right by the
        // prefix width). `tui_textarea` owns its own viewport and cursor
        // placement; because we shifted `area.x`, the cursor it emits will
        // also land in the shifted region automatically.
        let inner = Rect {
            x: area.x + PROMPT_PREFIX_WIDTH,
            y: area.y,
            width: area.width - PROMPT_PREFIX_WIDTH,
            height: area.height,
        };
        Widget::render(&self.textarea, inner, buf);

        self.render_model_label(area, buf);
    }
}

impl Prompt {
    /// Paint the optional right-aligned model label (e.g. `claude-opus-4-7`).
    ///
    /// Drawn on the first row of the prompt area, right-justified inside the
    /// full render rect (not the post-prefix inner rect) so the label hugs the
    /// far right column regardless of prefix width.
    fn render_model_label(&self, area: Rect, buf: &mut Buffer) {
        let Some(label) = &self.model_label else {
            return;
        };
        let label_width = label.chars().count() as u16;
        if label_width == 0 || label_width >= area.width {
            return;
        }
        let x = area.x + area.width - label_width;
        // Dim the label so it doesn't compete with the active composer text.
        buf.set_string(x, area.y, label, Style::default().dim());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn render_to_string(prompt: &Prompt, width: u16, height: u16) -> String {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|f| f.render_widget(prompt, f.area()))
            .unwrap();
        let buf = terminal.backend().buffer().clone();
        let area = buf.area();
        let mut out = String::new();
        for y in 0..area.height {
            for x in 0..area.width {
                out.push_str(buf[(area.x + x, area.y + y)].symbol());
            }
            if y + 1 < area.height {
                out.push('\n');
            }
        }
        out.lines()
            .map(|l| l.trim_end())
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn type_chars(prompt: &mut Prompt, text: &str) {
        for ch in text.chars() {
            prompt.handle_key(KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE));
        }
    }

    #[test]
    fn empty_prompt_renders_chevron_then_placeholder() {
        let prompt = Prompt::new();
        let out = render_to_string(&prompt, 30, 1);
        // First two columns must be the chevron + blank.
        assert!(out.starts_with("› "), "expected `› ` prefix, got: {out:?}");
        // Placeholder text follows after the prefix.
        assert!(out.contains("Ask Jekko"), "missing placeholder: {out:?}");
    }

    #[test]
    fn single_line_text_starts_after_prefix() {
        let mut prompt = Prompt::new();
        type_chars(&mut prompt, "hello");
        let out = render_to_string(&prompt, 20, 1);
        // Expected layout: "› hello…" — chars `h`,`e`,`l`,`l`,`o` start at col 2.
        assert!(
            out.starts_with("› hello"),
            "expected `› hello` prefix, got: {out:?}"
        );
    }

    #[test]
    fn shift_enter_multiline_only_first_row_has_chevron() {
        let mut prompt = Prompt::new();
        type_chars(&mut prompt, "first");
        prompt.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::SHIFT));
        type_chars(&mut prompt, "second");
        let out = render_to_string(&prompt, 20, 3);
        let mut lines = out.lines();
        let l0 = lines.next().unwrap_or("");
        let l1 = lines.next().unwrap_or("");
        assert!(l0.starts_with("› first"), "row 0 mismatch: {l0:?}");
        // Continuation row: cols 0-1 blank, body text starts at col 2.
        assert!(
            l1.starts_with("  second"),
            "row 1 should start with two blanks then body: {l1:?}"
        );
        assert!(!l1.starts_with("›"), "continuation row must not carry `›`");
    }

    fn render_buffer(prompt: &Prompt, width: u16, height: u16) -> Buffer {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|f| f.render_widget(prompt, f.area()))
            .unwrap();
        terminal.backend().buffer().clone()
    }

    #[test]
    fn multi_row_blank_gutter_on_each_continuation() {
        let mut prompt = Prompt::new();
        type_chars(&mut prompt, "a");
        prompt.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::SHIFT));
        type_chars(&mut prompt, "b");
        prompt.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::SHIFT));
        type_chars(&mut prompt, "c");
        // Render at 10 cols × 4 rows. Row 0: `› a`. Rows 1-2: `  b` / `  c`.
        // Row 3 has no textarea content; gutter must still be blank, never
        // a stray chevron.
        let buf = render_buffer(&prompt, 10, 4);
        assert_eq!(buf[(0, 0)].symbol(), "›");
        assert_eq!(buf[(1, 0)].symbol(), " ");
        assert_eq!(buf[(2, 0)].symbol(), "a");

        for y in 1u16..4 {
            assert_ne!(
                buf[(0, y)].symbol(),
                "›",
                "row {y} must not carry the chevron"
            );
            assert_eq!(buf[(0, y)].symbol(), " ", "row {y} col 0 must be blank");
            assert_eq!(buf[(1, y)].symbol(), " ", "row {y} col 1 must be blank");
        }
        // Body chars on rows 1 and 2.
        assert_eq!(buf[(2, 1)].symbol(), "b");
        assert_eq!(buf[(2, 2)].symbol(), "c");
    }

    #[test]
    fn chevron_cell_uses_blue_path_color() {
        let prompt = Prompt::new();
        let backend = TestBackend::new(20, 1);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|f| f.render_widget(&prompt, f.area()))
            .unwrap();
        let buf = terminal.backend().buffer();
        let cell = &buf[(0, 0)];
        assert_eq!(cell.symbol(), PROMPT_GLYPH);
        assert_eq!(cell.style().fg, Some(BLUE_PATH));
    }

    #[test]
    fn zero_width_area_does_not_panic() {
        let prompt = Prompt::new();
        let mut buf = Buffer::empty(Rect::new(0, 0, 0, 0));
        // No assertions — just verifying no panic on degenerate input.
        (&prompt).render(Rect::new(0, 0, 0, 0), &mut buf);
    }

    #[test]
    fn narrow_area_below_prefix_width_falls_back_to_textarea() {
        // 2-col area cannot afford the prefix + at least one body column.
        // Render must still succeed without panic and without overflow.
        let prompt = Prompt::new();
        let mut buf = Buffer::empty(Rect::new(0, 0, 2, 1));
        (&prompt).render(Rect::new(0, 0, 2, 1), &mut buf);
    }

    #[test]
    fn model_label_renders_right_aligned_when_fits() {
        let mut prompt = Prompt::new();
        prompt.set_model_label("claude-opus-4-7");
        let out = render_to_string(&prompt, 40, 1);
        // Label hugs the right edge of the area.
        assert!(out.ends_with("claude-opus-4-7"), "got: {out:?}");
        // Prefix still on the left.
        assert!(out.starts_with("› "), "prefix lost: {out:?}");
    }
}
