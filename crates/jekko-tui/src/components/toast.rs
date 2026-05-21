use std::time::Instant;

use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget};

use crate::glyph_set;

/// Toast severity. Ports `ui/toast.tsx`'s tone-based color palette.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ToastKind {
    Info,
    Success,
    Warning,
    Error,
}

impl ToastKind {
    pub fn label(self) -> &'static str {
        match self {
            ToastKind::Info => "info",
            ToastKind::Success => "success",
            ToastKind::Warning => "warning",
            ToastKind::Error => "error",
        }
    }

    fn accent(self) -> Color {
        match self {
            ToastKind::Info => Color::Rgb(0x55, 0xa3, 0xff),
            ToastKind::Success => Color::Rgb(0x55, 0xc7, 0x7c),
            ToastKind::Warning => Color::Rgb(0xf5, 0xa6, 0x23),
            ToastKind::Error => Color::Rgb(0xe2, 0x4a, 0x4a),
        }
    }

    fn sigil(self) -> &'static str {
        // T-A11Y-MIGRATION / T-GLYPH-WAVE2: honor the active GlyphMode for
        // every toast sigil (Unicode `ⓘ`/`✓`/`▲`/`✕` vs ASCII
        // `(i)`/`[v]`/`!`/`x`).
        let g = glyph_set::current();
        match self {
            ToastKind::Info => g.info_marker,
            ToastKind::Success => g.agent_done,
            ToastKind::Warning => g.warning_marker,
            ToastKind::Error => g.error_marker,
        }
    }
}

/// One toast entry.
#[derive(Clone, Debug)]
pub struct Toast {
    pub message: String,
    pub kind: ToastKind,
    pub created_at: Instant,
}

impl Toast {
    pub fn info(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            kind: ToastKind::Info,
            created_at: Instant::now(),
        }
    }
    pub fn success(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            kind: ToastKind::Success,
            created_at: Instant::now(),
        }
    }
    pub fn warning(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            kind: ToastKind::Warning,
            created_at: Instant::now(),
        }
    }
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            kind: ToastKind::Error,
            created_at: Instant::now(),
        }
    }
}

impl Widget for &Toast {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.kind.accent()));
        let inner = block.inner(area);
        block.render(area, buf);
        let line = Line::from(vec![
            Span::styled(
                format!("{} ", self.kind.sigil()),
                Style::default().fg(self.kind.accent()),
            ),
            Span::styled(
                self.message.clone(),
                Style::default().fg(Color::Rgb(0xd8, 0xde, 0xe9)),
            ),
        ]);
        Paragraph::new(line).render(inner, buf);
    }
}

/// A small stack of toasts pinned to the bottom-right. Renders up to
/// `MAX_VISIBLE` entries.
#[derive(Clone, Debug, Default)]
pub struct ToastStack {
    pub toasts: Vec<Toast>,
}

const MAX_VISIBLE: usize = 3;

impl ToastStack {
    pub fn push(&mut self, t: Toast) {
        self.toasts.push(t);
        while self.toasts.len() > MAX_VISIBLE * 2 {
            self.toasts.remove(0);
        }
    }

    /// Most recent toasts, newest first.
    pub fn recent(&self, limit: usize) -> Vec<&Toast> {
        self.toasts.iter().rev().take(limit).collect()
    }
}

impl Widget for &ToastStack {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let visible: Vec<&Toast> = self.toasts.iter().rev().take(MAX_VISIBLE).collect();
        if visible.is_empty() {
            return;
        }
        let rows = visible.len() as u16 * 3;
        let stack_area = Rect {
            x: area.x + area.width.saturating_sub(40),
            y: area.y + area.height.saturating_sub(rows + 1),
            width: 38.min(area.width),
            height: rows.min(area.height),
        };
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                visible
                    .iter()
                    .map(|_| Constraint::Length(3))
                    .collect::<Vec<_>>(),
            )
            .split(stack_area);
        for (idx, toast) in visible.iter().enumerate() {
            toast.render(chunks[idx], buf);
        }
    }
}
