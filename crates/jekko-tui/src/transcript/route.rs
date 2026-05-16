//! Session route composer.
//!
//! Ports `routes/session/session-view.tsx` into a single Ratatui widget that
//! lays out:
//!
//! ```text
//! ┌──────────────────────────────┬──────────────┐
//! │ transcript                   │  sidebar     │
//! │                              │              │
//! ├──────────────────────────────┴──────────────┤
//! │ prompt slot (caller-provided widget)        │
//! ├─────────────────────────────────────────────┤
//! │ footer band                                  │
//! └─────────────────────────────────────────────┘
//! ```
//!
//! The prompt slot accepts an `impl Widget` so the `Prompt` widget (from
//! `crate::prompt`) plugs in directly. Callers may also pass a simple
//! `Paragraph` in tests that don't need a real prompt.

use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget, Wrap};

use super::cards::SystemKind;
use super::sidebar::{SidebarPanel, StickyBottomIndicator, SubagentFooter};
use super::transcript::{Transcript, TranscriptEntry};

const COLOR_TEXT: Color = Color::Rgb(0xd8, 0xde, 0xe9);
const COLOR_TEXT_MUTED: Color = Color::Rgb(0x7d, 0x85, 0x90);
const COLOR_PANEL: Color = Color::Rgb(0x12, 0x15, 0x1c);
const COLOR_BORDER: Color = Color::Rgb(0x3a, 0x40, 0x4a);
const COLOR_SUCCESS: Color = Color::Rgb(0x8a, 0xc8, 0x6a);
const COLOR_WARN: Color = Color::Rgb(0xf5, 0xa6, 0x23);
const COLOR_ERROR: Color = Color::Rgb(0xe0, 0x6c, 0x75);
const COLOR_ACCENT: Color = Color::Rgb(0xd4, 0xa8, 0x43);

/// Default hint shown by the preview prompt when the caller supplies none.
const DEFAULT_PROMPT_HINT: &str = "submit · ctrl+c clear";

/// One-shot composer that draws the session route in a single `render` pass.
///
/// Holds references rather than owned widgets so the caller can compose
/// state from across the app without paying for clones. The lifetime `'a`
/// spans a single render call.
pub struct SessionRoute<'a, P>
where
    P: Widget,
{
    /// Transcript buffer.
    pub transcript: &'a Transcript,
    /// Sidebar panel widget.
    pub sidebar: Option<&'a SidebarPanel>,
    /// Optional subagent footer.
    pub subagent_footer: Option<&'a SubagentFooter>,
    /// Optional sticky-bottom indicator (caller computes pending count).
    pub sticky_indicator: Option<&'a StickyBottomIndicator>,
    /// Prompt slot. Caller supplies any `impl Widget` — typically the H
    /// subagent's `Prompt`. Passed by value so `Widget::render` can consume.
    pub prompt: P,
    /// Optional footer hint string.
    pub footer_hint: Option<&'a str>,
}

impl<'a, P> SessionRoute<'a, P>
where
    P: Widget,
{
    /// Build a session route around a transcript and prompt slot.
    pub fn new(transcript: &'a Transcript, prompt: P) -> Self {
        Self {
            transcript,
            sidebar: None,
            subagent_footer: None,
            sticky_indicator: None,
            prompt,
            footer_hint: None,
        }
    }
    /// Attach a sidebar.
    pub fn with_sidebar(mut self, sidebar: &'a SidebarPanel) -> Self {
        self.sidebar = Some(sidebar);
        self
    }
    /// Attach a subagent footer.
    pub fn with_subagent_footer(mut self, footer: &'a SubagentFooter) -> Self {
        self.subagent_footer = Some(footer);
        self
    }
    /// Attach a sticky-bottom indicator.
    pub fn with_sticky_indicator(mut self, indicator: &'a StickyBottomIndicator) -> Self {
        self.sticky_indicator = Some(indicator);
        self
    }
    /// Attach a footer hint.
    pub fn with_footer_hint(mut self, hint: &'a str) -> Self {
        self.footer_hint = Some(hint);
        self
    }
}

impl<'a, P> Widget for SessionRoute<'a, P>
where
    P: Widget,
{
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Outer split: main column + sidebar.
        let sidebar_width = self
            .sidebar
            .map(|s| s.width.min(area.width.saturating_sub(20)))
            .unwrap_or(0);
        let main_width = area.width.saturating_sub(sidebar_width);
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(main_width),
                Constraint::Length(sidebar_width),
            ])
            .split(area);

        let main_area = columns[0];
        let sidebar_area = columns[1];

        // Sidebar.
        if let (Some(sidebar), true) = (self.sidebar, sidebar_area.width > 0) {
            sidebar.render(sidebar_area, buf);
        }

        // Main column: transcript / inline cards / subagent footer / prompt / hint.
        let prompt_height: u16 = 5;
        let subagent_height: u16 = if self.subagent_footer.is_some() { 2 } else { 0 };
        let hint_height: u16 = if self.footer_hint.is_some() { 1 } else { 0 };
        let sticky_height: u16 = if self.sticky_indicator.is_some() {
            1
        } else {
            0
        };
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(1),
                Constraint::Length(sticky_height),
                Constraint::Length(subagent_height),
                Constraint::Length(prompt_height),
                Constraint::Length(hint_height),
            ])
            .split(main_area);
        let transcript_area = rows[0];
        let sticky_area = rows[1];
        let subagent_area = rows[2];
        let prompt_area = rows[3];
        let hint_area = rows[4];

        render_transcript(self.transcript, transcript_area, buf);
        if let Some(indicator) = self.sticky_indicator {
            indicator.render(sticky_area, buf);
        }
        if let Some(footer) = self.subagent_footer {
            footer.render(subagent_area, buf);
        }
        // Prompt slot — draw the bordered area then delegate inside.
        let prompt_block = Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(COLOR_BORDER))
            .style(Style::default().bg(COLOR_PANEL));
        let inner_prompt = prompt_block.inner(prompt_area);
        prompt_block.render(prompt_area, buf);
        self.prompt.render(inner_prompt, buf);

        if let Some(hint) = self.footer_hint {
            Paragraph::new(Line::from(Span::styled(
                hint.to_string(),
                Style::default().fg(COLOR_TEXT_MUTED),
            )))
            .render(hint_area, buf);
        }
    }
}

/// Paint the transcript's visible window into `area`. The renderer iterates
/// entries from the top, skipping the rows covered by the current scroll
/// offset.
fn render_transcript(transcript: &Transcript, area: Rect, buf: &mut Buffer) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let mut cursor_y = area.y;
    let max_y = area.y + area.height;
    // Clamp scroll_offset by the viewport actually being rendered. The
    // Transcript tracks a `viewport_rows` hint internally for sticky-bottom
    // math, but callers historically forgot to update it before rendering;
    // when it stays at zero the sticky-bottom logic pushes the offset all
    // the way past the entries and nothing paints. Recomputing the max
    // offset against the live area height keeps the renderer authoritative
    // and protects against that stale hint.
    let total_rows = transcript
        .entries()
        .iter()
        .map(|e| u32::from(e.estimated_rows()))
        .sum::<u32>();
    let live_max_offset = total_rows.saturating_sub(u32::from(area.height));
    let mut skip_rows = u32::from(transcript.scroll_offset()).min(live_max_offset);

    for entry in transcript.entries() {
        if cursor_y >= max_y {
            break;
        }
        let entry_rows = u32::from(entry.estimated_rows());
        if skip_rows >= entry_rows {
            skip_rows = skip_rows.saturating_sub(entry_rows);
            continue;
        }
        let visible_rows = (entry_rows - skip_rows) as u16;
        let height = visible_rows.min(max_y - cursor_y);
        skip_rows = 0;
        let slot = Rect {
            x: area.x,
            y: cursor_y,
            width: area.width,
            height,
        };
        render_entry(entry, slot, buf);
        cursor_y = cursor_y.saturating_add(height);
    }
    // If we ran out of entries before filling the area, leave the bottom
    // blank — the underlying buffer was already cleared by the caller.
    let _ = (
        COLOR_PANEL,
        COLOR_TEXT,
        COLOR_SUCCESS,
        COLOR_WARN,
        COLOR_ERROR,
        COLOR_ACCENT,
    );
}

fn render_entry(entry: &TranscriptEntry, slot: Rect, buf: &mut Buffer) {
    match entry {
        TranscriptEntry::User(card) => card.render(slot, buf),
        TranscriptEntry::Assistant(card) => card.render(slot, buf),
        TranscriptEntry::Tool(card) => card.render(slot, buf),
        TranscriptEntry::Reasoning(card) => card.render(slot, buf),
        TranscriptEntry::System(card) => card.render(slot, buf),
        TranscriptEntry::Permission(card) => card.render(slot, buf),
        TranscriptEntry::Question(card) => card.render(slot, buf),
    }
}

/// Convenience helper to render a single transcript window into a buffer
/// without the full route chrome. Useful for tests and snapshot tooling.
pub fn render_transcript_window(transcript: &Transcript, area: Rect, buf: &mut Buffer) {
    render_transcript(transcript, area, buf);
}

/// Format a system card's kind for a debug column.
pub fn system_kind_label(kind: SystemKind) -> &'static str {
    match kind {
        SystemKind::Info => "info",
        SystemKind::Success => "success",
        SystemKind::Warning => "warning",
        SystemKind::Error => "error",
    }
}

/// Simple prompt widget used by the route's own tests when the real `Prompt`
/// from packet H is not yet wired in. Exported so downstream tests in the
/// orchestrator can re-use it.
#[derive(Clone, Debug, Default)]
pub struct PreviewPrompt {
    /// Optional helper line.
    pub hint: Option<String>,
}

impl PreviewPrompt {
    /// Build a test prompt widget.
    pub fn new() -> Self {
        Self::default()
    }
    /// Attach a hint.
    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }
}

impl Widget for PreviewPrompt {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let lines = vec![
            Line::from(vec![
                Span::styled(
                    " > ",
                    Style::default()
                        .fg(COLOR_ACCENT)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "Awaiting Prompt widget (packet H)",
                    Style::default().fg(COLOR_TEXT),
                ),
            ]),
            Line::from(Span::styled(
                match self.hint {
                    Some(h) => h,
                    None => DEFAULT_PROMPT_HINT.to_string(),
                },
                Style::default().fg(COLOR_TEXT_MUTED),
            )),
        ];
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .render(area, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transcript::cards::{SystemCard, UserCard};
    use crate::transcript::sidebar::{DaemonStatus, SidebarPanel, StickyBottomIndicator};
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn sample_transcript() -> Transcript {
        let mut t = Transcript::new();
        t.set_viewport_rows(10);
        t.push_user(UserCard::new("hi".into()));
        t.push_system(SystemCard::new("daemon online", SystemKind::Success));
        t
    }

    #[test]
    fn route_renders_transcript_and_prompt() {
        let transcript = sample_transcript();
        let prompt = PreviewPrompt::new();
        let route = SessionRoute::new(&transcript, prompt);
        let mut terminal = Terminal::new(TestBackend::new(60, 12)).unwrap();
        terminal.draw(|f| f.render_widget(route, f.area())).unwrap();
        let buf = terminal.backend().buffer().clone();
        let rendered: String = buf.content.iter().map(|c| c.symbol()).collect();
        assert!(rendered.contains("hi"));
        assert!(rendered.contains("daemon online"));
        assert!(rendered.contains("Awaiting Prompt"));
    }

    #[test]
    fn route_with_sidebar_includes_title() {
        let transcript = sample_transcript();
        let sidebar = SidebarPanel::new("My Session")
            .with_session_id("sess_abc")
            .with_daemon_status(DaemonStatus::Online);
        let prompt = PreviewPrompt::new();
        let route = SessionRoute::new(&transcript, prompt).with_sidebar(&sidebar);
        let mut terminal = Terminal::new(TestBackend::new(80, 14)).unwrap();
        terminal.draw(|f| f.render_widget(route, f.area())).unwrap();
        let buf = terminal.backend().buffer().clone();
        let rendered: String = buf.content.iter().map(|c| c.symbol()).collect();
        assert!(rendered.contains("My Session"));
        assert!(rendered.contains("daemon online"));
    }

    #[test]
    fn route_with_sticky_indicator_renders_pill() {
        let transcript = sample_transcript();
        let pill = StickyBottomIndicator::new(2);
        let prompt = PreviewPrompt::new();
        let route = SessionRoute::new(&transcript, prompt).with_sticky_indicator(&pill);
        let mut terminal = Terminal::new(TestBackend::new(80, 14)).unwrap();
        terminal.draw(|f| f.render_widget(route, f.area())).unwrap();
        let buf = terminal.backend().buffer().clone();
        let rendered: String = buf.content.iter().map(|c| c.symbol()).collect();
        assert!(rendered.contains("2 new"));
    }

    #[test]
    fn route_with_footer_hint_renders() {
        let transcript = sample_transcript();
        let prompt = PreviewPrompt::new();
        let route = SessionRoute::new(&transcript, prompt).with_footer_hint("/help · ctrl+c quit");
        let mut terminal = Terminal::new(TestBackend::new(80, 14)).unwrap();
        terminal.draw(|f| f.render_widget(route, f.area())).unwrap();
        let buf = terminal.backend().buffer().clone();
        let rendered: String = buf.content.iter().map(|c| c.symbol()).collect();
        assert!(rendered.contains("help"));
    }

    #[test]
    fn preview_prompt_renders() {
        let prompt = PreviewPrompt::new().with_hint("custom hint");
        let mut terminal = Terminal::new(TestBackend::new(60, 3)).unwrap();
        terminal
            .draw(|f| f.render_widget(prompt, f.area()))
            .unwrap();
        let buf = terminal.backend().buffer().clone();
        let rendered: String = buf.content.iter().map(|c| c.symbol()).collect();
        assert!(rendered.contains("custom hint"));
    }

    #[test]
    fn render_transcript_window_paints_inside_area() {
        let transcript = sample_transcript();
        let area = Rect::new(0, 0, 60, 6);
        let mut buf = Buffer::empty(area);
        render_transcript_window(&transcript, area, &mut buf);
        let rendered: String = buf.content.iter().map(|c| c.symbol()).collect();
        assert!(rendered.contains("hi"));
    }

    #[test]
    fn system_kind_label_round_trip() {
        assert_eq!(system_kind_label(SystemKind::Info), "info");
        assert_eq!(system_kind_label(SystemKind::Success), "success");
        assert_eq!(system_kind_label(SystemKind::Warning), "warning");
        assert_eq!(system_kind_label(SystemKind::Error), "error");
    }
}
