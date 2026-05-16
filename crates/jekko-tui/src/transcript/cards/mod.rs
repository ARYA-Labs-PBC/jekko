//! Transcript cards.
//!
//! Each variant of [`super::transcript::TranscriptEntry`] resolves to a card
//! in this module. Cards implement `ratatui::widgets::Widget` for a borrowed
//! reference, mirroring the rest of the crate's component style.

use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Widget};

mod assistant;
mod reasoning;
mod system;
mod theme;
mod tool;
mod user;

pub use assistant::{AssistantCard, AssistantPart, AssistantPartKind};
pub use reasoning::ReasoningCard;
pub use system::{SystemCard, SystemKind};
pub use tool::{ToolCard, ToolStatus};
pub use user::UserCard;

use theme::COLOR_PANEL;

/// Reusable boxed-panel chrome (shared shape across cards). Caller draws
/// children into the returned inner rectangle.
pub fn render_panel(area: Rect, buf: &mut Buffer, color: Color) -> Rect {
    let block = Block::default()
        .borders(Borders::LEFT)
        .border_style(Style::default().fg(color))
        .style(Style::default().bg(COLOR_PANEL));
    let inner = block.inner(area);
    block.render(area, buf);
    let padded = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(0),
            Constraint::Min(0),
            Constraint::Length(0),
        ])
        .split(inner);
    padded[1]
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    #[test]
    fn user_card_snapshot_contains_text() {
        let card = UserCard::new("hello".into()).with_timestamp_label("00:01");
        let snap = card.snapshot();
        assert!(snap.contains("hello"));
        assert!(snap.contains("00:01"));
    }

    #[test]
    fn user_card_renders_visible_text() {
        let card = UserCard::new("ping".into());
        let mut terminal = Terminal::new(TestBackend::new(20, 4)).unwrap();
        terminal.draw(|f| f.render_widget(&card, f.area())).unwrap();
        let buf = terminal.backend().buffer().clone();
        let rendered: String = buf.content.iter().map(|c| c.symbol()).collect();
        assert!(rendered.contains("you"));
        assert!(rendered.contains("ping"));
    }

    #[test]
    fn assistant_card_renders_parts() {
        let card = AssistantCard::new(vec![
            AssistantPart::new(AssistantPartKind::Text, "hi".into()),
            AssistantPart::new(AssistantPartKind::Reasoning, "...".into()),
        ])
        .with_model("model-x")
        .with_duration_secs(1.25);
        let mut terminal = Terminal::new(TestBackend::new(40, 6)).unwrap();
        terminal.draw(|f| f.render_widget(&card, f.area())).unwrap();
        let buf = terminal.backend().buffer().clone();
        let rendered: String = buf.content.iter().map(|c| c.symbol()).collect();
        assert!(rendered.contains("Jekko"));
        assert!(rendered.contains("hi"));
    }

    #[test]
    fn assistant_card_snapshot_includes_kinds() {
        let card = AssistantCard::new(vec![
            AssistantPart::new(AssistantPartKind::Text, "yes".into()),
            AssistantPart::new(AssistantPartKind::ToolCall, "shell".into()),
        ]);
        let snap = card.snapshot();
        assert!(snap.contains("text"));
        assert!(snap.contains("tool"));
    }

    #[test]
    fn tool_card_toggle_expanded() {
        let mut card = ToolCard::new("tool_1", "shell")
            .with_output("line1\nline2\nline3")
            .with_status(ToolStatus::Completed);
        let rows_collapsed = card.estimated_rows();
        card.toggle_expanded();
        let rows_expanded = card.estimated_rows();
        assert!(rows_expanded >= rows_collapsed);
    }

    #[test]
    fn tool_card_status_glyphs() {
        assert_eq!(ToolStatus::Pending.glyph(), "…");
        assert_eq!(ToolStatus::Running.glyph(), "▸");
        assert_eq!(ToolStatus::Completed.glyph(), "✓");
        assert_eq!(ToolStatus::Error.glyph(), "✗");
        assert_eq!(ToolStatus::Cancelled.glyph(), "⊘");
    }

    #[test]
    fn tool_card_renders_input_line() {
        let card = ToolCard::new("tool_1", "shell")
            .with_input("ls -la")
            .with_status(ToolStatus::Running);
        let mut terminal = Terminal::new(TestBackend::new(40, 4)).unwrap();
        terminal.draw(|f| f.render_widget(&card, f.area())).unwrap();
        let buf = terminal.backend().buffer().clone();
        let rendered: String = buf.content.iter().map(|c| c.symbol()).collect();
        assert!(rendered.contains("shell"));
        assert!(rendered.contains("ls -la"));
    }

    #[test]
    fn reasoning_card_collapsed_estimated_rows() {
        let mut card = ReasoningCard::new("a\nb\nc");
        assert!(card.estimated_rows() >= 3);
        card.toggle_collapsed();
        assert_eq!(card.estimated_rows(), 2);
    }

    #[test]
    fn reasoning_card_renders_thinking_label() {
        let card = ReasoningCard::new("hmm");
        let mut terminal = Terminal::new(TestBackend::new(20, 3)).unwrap();
        terminal.draw(|f| f.render_widget(&card, f.area())).unwrap();
        let buf = terminal.backend().buffer().clone();
        let rendered: String = buf.content.iter().map(|c| c.symbol()).collect();
        assert!(rendered.contains("thinking"));
        assert!(rendered.contains("hmm"));
    }

    #[test]
    fn system_card_snapshot_includes_kind() {
        let card = SystemCard::new("daemon up", SystemKind::Success);
        let snap = card.snapshot();
        assert!(snap.contains("daemon up"));
        assert!(snap.contains("Success"));
    }

    #[test]
    fn system_card_renders_icon() {
        let card = SystemCard::new("ok", SystemKind::Success);
        let mut terminal = Terminal::new(TestBackend::new(10, 1)).unwrap();
        terminal.draw(|f| f.render_widget(&card, f.area())).unwrap();
        let buf = terminal.backend().buffer().clone();
        let rendered: String = buf.content.iter().map(|c| c.symbol()).collect();
        assert!(rendered.contains("ok"));
    }

    #[test]
    fn render_panel_returns_nonzero_inner() {
        let area = Rect::new(0, 0, 20, 6);
        let mut buf = Buffer::empty(area);
        let inner = render_panel(area, &mut buf, Color::Rgb(0xd4, 0xa8, 0x43));
        assert!(inner.width > 0);
        assert!(inner.height > 0);
    }
}
