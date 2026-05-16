use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

use super::frame::{render_frame, DialogFrame};

/// One pickable row.
#[derive(Clone, Debug)]
pub struct SelectOption {
    pub id: String,
    pub label: String,
    pub hint: Option<String>,
    pub disabled: bool,
}

impl SelectOption {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            hint: None,
            disabled: false,
        }
    }
    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }
}

/// A vertical list picker. Ports the core of `ui/dialog-select.tsx`.
#[derive(Clone, Debug)]
pub struct SelectDialog {
    pub title: String,
    pub options: Vec<SelectOption>,
    pub cursor: usize,
    pub width: u16,
    pub height: u16,
}

const GOLD: Color = Color::Rgb(0xd4, 0xa8, 0x43);
const TEXT: Color = Color::Rgb(0xd8, 0xde, 0xe9);
const TEXT_MUTED: Color = Color::Rgb(0x7d, 0x85, 0x90);
const TEXT_DIM: Color = Color::Rgb(0x52, 0x57, 0x60);
const ACCENT_DIVIDER: Color = Color::Rgb(0x3a, 0x40, 0x4a);

impl SelectDialog {
    pub fn new(title: impl Into<String>, options: Vec<SelectOption>) -> Self {
        // Height budget:
        //   2 borders (top/bottom)
        // + 1 title bar
        // + 1 subtitle
        // + 1 divider
        // + 1 bottom pad
        // + 1 sub-divider (options list body)
        // + 1 hover hint row (options list body)
        // = 8 fixed rows. Add `options.len()` for the list itself, clamped to
        // a 12-row maximum so very long lists don't grow the dialog past
        // typical terminal heights.
        let height = options.len().min(12) as u16 + 8;
        Self {
            title: title.into(),
            options,
            cursor: 0,
            width: 60,
            height,
        }
    }

    pub fn move_cursor(&mut self, delta: isize) {
        if self.options.is_empty() {
            self.cursor = 0;
            return;
        }
        let len = self.options.len() as isize;
        // Recover from any externally tampered `pub cursor` (out-of-bounds set
        // bypasses `set_cursor`/`move_cursor`) by clamping back into range
        // before applying the delta.
        let base = (self.cursor as isize).min(len - 1).max(0);
        // Use `saturating_add` so an attacker-supplied huge delta (e.g.
        // `isize::MIN`/`isize::MAX`) cannot overflow the index calculation,
        // and `rem_euclid` so the wrap-around math handles negative deltas
        // without an unbounded loop.
        let idx = base.saturating_add(delta).rem_euclid(len);
        self.cursor = idx as usize;
    }

    /// Externally set the cursor, clamping into `[0, options.len())`.
    /// Callers should prefer this over mutating the `pub cursor` field so
    /// untrusted indices cannot escape the option range and trigger a panic
    /// in callers that bypass `selected()` (which uses `.get`).
    pub fn set_cursor(&mut self, idx: usize) {
        if self.options.is_empty() {
            self.cursor = 0;
            return;
        }
        let max = self.options.len() - 1;
        self.cursor = idx.min(max);
    }

    pub fn cursor_is_valid(&self) -> bool {
        self.options.is_empty() || self.cursor < self.options.len()
    }

    fn render_text(text: &str, max_chars: usize) -> String {
        let mut out = String::with_capacity(max_chars.min(text.len()));
        for ch in text.chars().filter(|c| !c.is_control()) {
            if out.chars().count() == max_chars {
                break;
            }
            out.push(ch);
        }
        out
    }

    pub fn selected(&self) -> Option<&SelectOption> {
        self.options.get(self.cursor)
    }
}

impl Widget for &SelectDialog {
    fn render(self, outer: Rect, buf: &mut Buffer) {
        let frame = DialogFrame::new(self.width, self.height)
            .with_title(&self.title)
            .with_subtitle("↑↓ navigate • ↵ select • esc cancel");
        let inner = render_frame(&frame, outer, buf);

        // Body chunks: options list + footer hover hint row.
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .split(inner);
        let max_chars = chunks[0].width.saturating_sub(4) as usize;

        let lines: Vec<Line> = self
            .options
            .iter()
            .enumerate()
            .map(|(i, opt)| {
                let active = i == self.cursor;
                let mut spans = vec![Span::raw(if active { "\u{25b8} " } else { "  " })];
                let label_style = if opt.disabled {
                    Style::default().fg(TEXT_DIM)
                } else if active {
                    Style::default().fg(GOLD).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(TEXT)
                };
                spans.push(Span::styled(
                    SelectDialog::render_text(&opt.label, max_chars),
                    label_style,
                ));
                if let Some(hint) = opt.hint.as_deref() {
                    spans.push(Span::raw("  "));
                    spans.push(Span::styled(
                        SelectDialog::render_text(hint, max_chars),
                        Style::default().fg(TEXT_MUTED),
                    ));
                }
                Line::from(spans)
            })
            .collect();
        Paragraph::new(lines).render(chunks[0], buf);

        // Sub-divider above the hover hint row.
        Paragraph::new(Line::from(Span::styled(
            "\u{2500}".repeat(chunks[1].width as usize),
            Style::default().fg(ACCENT_DIVIDER),
        )))
        .render(chunks[1], buf);

        // Hover hint row echoes the highlighted option's hint under the
        // currently-selected row.
        let hint = self
            .options
            .get(self.cursor)
            .and_then(|opt| opt.hint.as_deref())
            .unwrap_or("");
        let hover = Line::from(vec![
            Span::styled(
                "hover: ",
                Style::default().fg(TEXT_DIM).add_modifier(Modifier::DIM),
            ),
            Span::styled(
                SelectDialog::render_text(hint, max_chars),
                Style::default().fg(TEXT_MUTED),
            ),
        ]);
        Paragraph::new(hover).render(chunks[2], buf);
    }
}
