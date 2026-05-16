use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

use super::frame::{render_frame, DialogFrame};

/// Command palette entry. Ports the row shape from `dialog-command.tsx`.
#[derive(Clone, Debug)]
pub struct CommandEntry {
    pub id: String,
    pub label: String,
    pub description: Option<String>,
    pub keybind_hint: Option<String>,
}

impl CommandEntry {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            description: None,
            keybind_hint: None,
        }
    }
    pub fn with_description(mut self, d: impl Into<String>) -> Self {
        self.description = Some(d.into());
        self
    }
    pub fn with_keybind(mut self, k: impl Into<String>) -> Self {
        self.keybind_hint = Some(k.into());
        self
    }
}

/// Ctrl+P command palette. Maintains a fuzzy filter buffer and a cursor over
/// the visible entries.
#[derive(Clone, Debug)]
pub struct CommandPalette {
    pub query: String,
    pub entries: Vec<CommandEntry>,
    pub cursor: usize,
}

impl CommandPalette {
    pub fn new(entries: Vec<CommandEntry>) -> Self {
        Self {
            query: String::new(),
            entries,
            cursor: 0,
        }
    }

    /// Filtered entries (substring match, case-insensitive). Stable order.
    pub fn visible(&self) -> Vec<&CommandEntry> {
        if self.query.is_empty() {
            return self.entries.iter().collect();
        }
        let needle = self.query.to_lowercase();
        self.entries
            .iter()
            .filter(|e| {
                e.label.to_lowercase().contains(&needle)
                    || e.id.to_lowercase().contains(&needle)
                    || e.description
                        .as_deref()
                        .map(|d| d.to_lowercase().contains(&needle))
                        .unwrap_or(false)
            })
            .collect()
    }

    pub fn move_cursor(&mut self, delta: isize) {
        let len = self.visible().len() as isize;
        if len == 0 {
            self.cursor = 0;
            return;
        }
        let mut idx = self.cursor as isize + delta;
        while idx < 0 {
            idx += len;
        }
        self.cursor = (idx % len) as usize;
    }

    pub fn type_char(&mut self, ch: char) {
        self.query.push(ch);
        self.cursor = 0;
    }

    pub fn backspace(&mut self) {
        self.query.pop();
        self.cursor = 0;
    }

    pub fn selected(&self) -> Option<&CommandEntry> {
        self.visible().get(self.cursor).copied()
    }
}

const GOLD: Color = Color::Rgb(0xd4, 0xa8, 0x43);
const TEXT: Color = Color::Rgb(0xd8, 0xde, 0xe9);
const TEXT_MUTED: Color = Color::Rgb(0x7d, 0x85, 0x90);
const TEXT_DIM: Color = Color::Rgb(0x52, 0x57, 0x60);
const ACCENT_DIVIDER: Color = Color::Rgb(0x3a, 0x40, 0x4a);

impl Widget for &CommandPalette {
    fn render(self, outer: Rect, buf: &mut Buffer) {
        let frame = DialogFrame::new(64, 18)
            .with_title("Commands")
            .with_subtitle("Type to filter • ↑↓ navigate • ↵ run");
        let inner = render_frame(&frame, outer, buf);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Min(0),
            ])
            .split(inner);

        // Prompt row. The trailing underline cursor block gives the palette an
        // explicit cursor even though this is a rendered terminal widget.
        let prompt = Line::from(vec![
            Span::styled("> ", Style::default().fg(GOLD).add_modifier(Modifier::BOLD)),
            Span::styled(self.query.clone(), Style::default().fg(TEXT)),
            Span::styled(
                " ",
                Style::default().bg(GOLD).add_modifier(Modifier::UNDERLINED),
            ),
        ]);
        Paragraph::new(prompt).render(chunks[0], buf);

        // Sub-divider beneath the prompt.
        let sep = Line::from(Span::styled(
            "\u{2500}".repeat(chunks[1].width as usize),
            Style::default().fg(ACCENT_DIVIDER),
        ));
        Paragraph::new(sep).render(chunks[1], buf);

        let visible = self.visible();
        let lines: Vec<Line> = visible
            .iter()
            .enumerate()
            .map(|(i, e)| {
                let active = i == self.cursor;
                let style = if active {
                    Style::default().fg(GOLD).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(TEXT)
                };
                let mut spans = vec![
                    Span::raw(if active { "\u{25b8} " } else { "  " }),
                    Span::styled(e.label.clone(), style),
                ];
                if let Some(k) = e.keybind_hint.as_deref() {
                    spans.push(Span::raw("  "));
                    spans.push(Span::styled(
                        format!("[{k}]"),
                        Style::default().fg(TEXT_MUTED),
                    ));
                }
                if let Some(d) = e.description.as_deref() {
                    spans.push(Span::raw("  "));
                    spans.push(Span::styled(d.to_string(), Style::default().fg(TEXT_DIM)));
                }
                Line::from(spans)
            })
            .collect();
        Paragraph::new(lines).render(chunks[2], buf);
    }
}
