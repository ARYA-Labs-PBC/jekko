//! The composite `Prompt` widget.
//!
//! Glues together the textarea, slash + mention popups, paste buffer, history,
//! frecency and per-route stash. Exposes the small set of public methods the
//! TUI loop drives (`handle_key`, `handle_paste`, `submit`, …).
//!
//! Key handling lives in [`keys`] and the `Widget` render impl in [`render`];
//! both are sibling modules so this file stays under the per-file LOC budget.

use ratatui::style::Style;
use tui_textarea::TextArea;

use super::frecency::Frecency;
use super::history::PromptHistory;
use super::mentions::{MentionCandidate, MentionPopup};
use super::paste::PasteBuffer;
use super::slash::{SlashCommand, SlashPopup};
use super::stash::PromptStash;
use crate::glyph_set;

mod buffer;
mod keys;
mod render;

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
    pub(super) textarea: TextArea<'static>,
    pub(super) history: PromptHistory,
    pub(super) frecency: Frecency,
    pub(super) stash: PromptStash,
    pub(super) slash: SlashPopup,
    pub(super) mention: MentionPopup,
    pub(super) paste: PasteBuffer,
    /// Optional hint shown when the buffer is empty.
    pub(super) empty_hint: String,
    /// Right-aligned label (e.g. model name).
    pub(super) model_label: Option<String>,
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use ratatui::backend::TestBackend;
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;
    use ratatui::widgets::Widget;
    use ratatui::Terminal;

    use crate::theme::codex::BLUE_PATH;

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
