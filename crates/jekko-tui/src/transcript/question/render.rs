//! Ratatui rendering for `QuestionCard`.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget, Wrap};

use super::model::{QuestionCard, QuestionMode};

const COLOR_TEXT: Color = Color::Rgb(0xd8, 0xde, 0xe9);
const COLOR_TEXT_MUTED: Color = Color::Rgb(0x7d, 0x85, 0x90);
const COLOR_ACCENT: Color = Color::Rgb(0xd4, 0xa8, 0x43);
const COLOR_SUCCESS: Color = Color::Rgb(0x8a, 0xc8, 0x6a);
const COLOR_PANEL: Color = Color::Rgb(0x12, 0x15, 0x1c);

impl Widget for &QuestionCard {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .borders(Borders::LEFT)
            .border_style(Style::default().fg(COLOR_ACCENT))
            .style(Style::default().bg(COLOR_PANEL));
        let inner = block.inner(area);
        block.render(area, buf);

        let mut lines: Vec<Line> = Vec::new();
        let suffix = match self.mode {
            QuestionMode::Multi => " (select all that apply)",
            QuestionMode::Single => "",
        };
        lines.push(Line::from(vec![
            Span::styled(
                " ? ",
                Style::default()
                    .fg(COLOR_ACCENT)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{}{}", self.prompt, suffix),
                Style::default().fg(COLOR_TEXT).add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(Span::raw("")));

        for (i, option) in self.options.iter().enumerate() {
            lines.push(option_line(self, i, &option.label));
            if let Some(desc) = &option.description {
                lines.push(Line::from(Span::styled(
                    format!("       {desc}"),
                    Style::default().fg(COLOR_TEXT_MUTED),
                )));
            }
        }
        if self.allow_custom {
            let idx = self.options.len();
            let label = if self.custom_text.is_empty() {
                "Type your own answer".to_string()
            } else {
                self.custom_text.clone()
            };
            lines.push(option_line(self, idx, &label));
            if self.editing_custom {
                lines.push(Line::from(vec![
                    Span::raw("       "),
                    Span::styled("> ", Style::default().fg(COLOR_ACCENT)),
                    Span::styled(self.custom_text.clone(), Style::default().fg(COLOR_TEXT)),
                    Span::styled(
                        "_",
                        Style::default()
                            .fg(COLOR_ACCENT)
                            .add_modifier(Modifier::SLOW_BLINK),
                    ),
                ]));
            }
        }
        lines.push(Line::from(Span::raw("")));
        lines.push(Line::from(Span::styled(
            match self.mode {
                QuestionMode::Single => "  ↑↓ select · enter confirm · esc dismiss",
                QuestionMode::Multi => "  ↑↓ select · enter toggle · esc dismiss",
            },
            Style::default().fg(COLOR_TEXT_MUTED),
        )));

        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .render(inner, buf);
    }
}

fn option_line(card: &QuestionCard, idx: usize, label: &str) -> Line<'static> {
    let active = card.cursor == idx;
    let picked_label = if card.is_custom_slot(idx) {
        card.custom_text.clone()
    } else {
        label.to_string()
    };
    let picked = !picked_label.is_empty() && card.picked.iter().any(|l| l == &picked_label);

    let bullet = match (card.mode, picked) {
        (QuestionMode::Multi, true) => "  [✓] ",
        (QuestionMode::Multi, false) => "  [ ] ",
        (QuestionMode::Single, true) => "  ✓ ",
        (QuestionMode::Single, false) => "  • ",
    };
    let style_label = if active {
        Style::default()
            .fg(COLOR_ACCENT)
            .add_modifier(Modifier::BOLD)
    } else if picked {
        Style::default().fg(COLOR_SUCCESS)
    } else {
        Style::default().fg(COLOR_TEXT)
    };
    let style_num = if active {
        Style::default().fg(COLOR_ACCENT)
    } else {
        Style::default().fg(COLOR_TEXT_MUTED)
    };
    Line::from(vec![
        Span::styled(format!(" {}.", idx + 1), style_num),
        Span::styled(bullet.to_string(), Style::default().fg(COLOR_TEXT_MUTED)),
        Span::styled(label.to_string(), style_label),
    ])
}
