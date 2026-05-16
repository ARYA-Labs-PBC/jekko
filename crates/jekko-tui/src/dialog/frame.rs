use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph, Widget};

/// Reusable dialog chrome. Mirrors `ui/dialog-frame.tsx`: clear backdrop,
/// rounded borders, two-cell horizontal padding around the inner body, gold
/// accent bar across the top of the frame with a centered title.
#[derive(Clone, Debug)]
pub struct DialogFrame<'a> {
    pub title: Option<&'a str>,
    pub subtitle: Option<&'a str>,
    pub width: u16,
    pub height: u16,
}

const GOLD: Color = Color::Rgb(0xd4, 0xa8, 0x43);
const ACCENT_BG: Color = Color::Rgb(0x2a, 0x22, 0x10);
const BORDER: Color = Color::Rgb(0x4a, 0x52, 0x60);
const BG_PANEL: Color = Color::Rgb(0x0b, 0x0f, 0x14);
const ACCENT_DIVIDER: Color = Color::Rgb(0x3a, 0x40, 0x4a);
const TEXT_MUTED: Color = Color::Rgb(0x7d, 0x85, 0x90);

impl<'a> DialogFrame<'a> {
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            title: None,
            subtitle: None,
            width,
            height,
        }
    }
    pub fn with_title(mut self, title: &'a str) -> Self {
        self.title = Some(title);
        self
    }

    pub fn with_subtitle(mut self, subtitle: &'a str) -> Self {
        self.subtitle = Some(subtitle);
        self
    }

    /// Compute the centered rect for this dialog within `outer`.
    pub fn area_in(&self, outer: Rect) -> Rect {
        let w = self.width.min(outer.width.saturating_sub(4));
        let h = self.height.min(outer.height.saturating_sub(2));
        let x = outer.x + (outer.width.saturating_sub(w)) / 2;
        let y = outer.y + (outer.height.saturating_sub(h)) / 2;
        Rect {
            x,
            y,
            width: w,
            height: h,
        }
    }
}

/// Draws the chrome and returns the inner content area.
pub fn render_frame(frame: &DialogFrame, outer: Rect, buf: &mut Buffer) -> Rect {
    let area = frame.area_in(outer);
    Clear.render(area, buf);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(BORDER))
        .style(Style::default().bg(BG_PANEL));
    let inner = block.inner(area);
    block.render(area, buf);

    // Title bar: gold-on-accent ribbon spanning the inner width. Mirrors the
    // `<text attributes={TextAttributes.BOLD} fg={theme.text}>` row in
    // `dialog-frame.tsx`, plus an "esc" hint on the right and an optional
    // subtitle row directly underneath.
    let has_subtitle = frame.subtitle.is_some();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(if has_subtitle {
            vec![
                Constraint::Length(1), // title bar
                Constraint::Length(1), // subtitle
                Constraint::Length(1), // divider
                Constraint::Min(0),    // body
                Constraint::Length(1), // bottom pad
            ]
        } else {
            vec![
                Constraint::Length(1), // title bar
                Constraint::Length(1), // divider
                Constraint::Min(0),    // body
                Constraint::Length(1), // bottom pad
            ]
        })
        .split(inner);

    if let Some(title) = frame.title {
        let title_span = Span::styled(
            format!("  {title}  "),
            Style::default()
                .fg(GOLD)
                .bg(ACCENT_BG)
                .add_modifier(Modifier::BOLD),
        );
        let esc_span = Span::styled(
            "esc",
            Style::default().fg(TEXT_MUTED).add_modifier(Modifier::DIM),
        );
        let bar_width = chunks[0].width as usize;
        let title_text_width: usize = title_span.content.chars().count();
        let esc_width = 4; // "esc " incl. trailing pad
        let fill_width = bar_width
            .saturating_sub(title_text_width)
            .saturating_sub(esc_width);
        let mut spans = vec![
            title_span,
            Span::styled(" ".repeat(fill_width), Style::default().bg(BG_PANEL)),
            esc_span,
            Span::raw(" "),
        ];
        // Guard against terminals too narrow to fit the esc hint.
        if fill_width == 0 {
            spans = vec![Span::styled(
                format!("  {title}  "),
                Style::default()
                    .fg(GOLD)
                    .bg(ACCENT_BG)
                    .add_modifier(Modifier::BOLD),
            )];
        }
        Paragraph::new(Line::from(spans)).render(chunks[0], buf);
    } else {
        Paragraph::new("").render(chunks[0], buf);
    }

    let body_idx = if has_subtitle {
        if let Some(subtitle) = frame.subtitle {
            Paragraph::new(Line::from(Span::styled(
                subtitle.to_string(),
                Style::default()
                    .fg(TEXT_MUTED)
                    .add_modifier(Modifier::ITALIC),
            )))
            .render(chunks[1], buf);
        }
        // Divider.
        Paragraph::new(Line::from(Span::styled(
            "\u{2500}".repeat(chunks[2].width as usize),
            Style::default().fg(ACCENT_DIVIDER),
        )))
        .render(chunks[2], buf);
        3
    } else {
        // Divider.
        Paragraph::new(Line::from(Span::styled(
            "\u{2500}".repeat(chunks[1].width as usize),
            Style::default().fg(ACCENT_DIVIDER),
        )))
        .render(chunks[1], buf);
        2
    };

    let row = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(0),
            Constraint::Length(2),
        ])
        .split(chunks[body_idx]);
    row[1]
}
