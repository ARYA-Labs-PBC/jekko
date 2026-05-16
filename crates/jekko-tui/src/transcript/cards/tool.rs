//! Tool invocation card and its status enum.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget, Wrap};

use crate::transcript::diff::{DiffFile, DiffLineKind};

use super::theme::{
    COLOR_ACCENT, COLOR_BORDER, COLOR_DIFF_ADD, COLOR_DIFF_DEL, COLOR_ERROR, COLOR_SUCCESS,
    COLOR_TEXT, COLOR_TEXT_MUTED,
};

/// Tool execution status. Mirrors the `state.status` enum from the SDK.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ToolStatus {
    /// Awaiting permission or scheduler.
    Pending,
    /// Currently running.
    Running,
    /// Completed normally.
    Completed,
    /// Failed.
    Error,
    /// Cancelled.
    Cancelled,
}

impl ToolStatus {
    pub(super) fn glyph(self) -> &'static str {
        match self {
            ToolStatus::Pending => "…",
            ToolStatus::Running => "▸",
            ToolStatus::Completed => "✓",
            ToolStatus::Error => "✗",
            ToolStatus::Cancelled => "⊘",
        }
    }
    pub(super) fn color(self) -> Color {
        match self {
            ToolStatus::Pending => COLOR_TEXT_MUTED,
            ToolStatus::Running => COLOR_ACCENT,
            ToolStatus::Completed => COLOR_SUCCESS,
            ToolStatus::Error => COLOR_ERROR,
            ToolStatus::Cancelled => COLOR_TEXT_MUTED,
        }
    }
}

/// One tool invocation card.
#[derive(Clone, Debug)]
pub struct ToolCard {
    /// Tool call id (e.g. `tool_abc123`).
    pub tool_id: String,
    /// Tool name (e.g. `shell`, `read`, `edit`).
    pub name: String,
    /// Current status.
    pub status: ToolStatus,
    /// Short summary of the input (single line).
    pub input_summary: Option<String>,
    /// Short summary of the output (multi-line allowed).
    pub output_summary: Option<String>,
    /// Optional unified diff payload (rendered as a coloured patch).
    pub diff: Option<Vec<DiffFile>>,
    /// Whether the card is expanded.
    pub expanded: bool,
}

impl ToolCard {
    /// Build a tool card.
    pub fn new(tool_id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            tool_id: tool_id.into(),
            name: name.into(),
            status: ToolStatus::Pending,
            input_summary: None,
            output_summary: None,
            diff: None,
            expanded: false,
        }
    }
    /// Attach a status.
    pub fn with_status(mut self, status: ToolStatus) -> Self {
        self.status = status;
        self
    }
    /// Attach an input summary line.
    pub fn with_input(mut self, summary: impl Into<String>) -> Self {
        self.input_summary = Some(summary.into());
        self
    }
    /// Attach an output summary block.
    pub fn with_output(mut self, summary: impl Into<String>) -> Self {
        self.output_summary = Some(summary.into());
        self
    }
    /// Attach a unified diff payload.
    pub fn with_diff(mut self, diff: Vec<DiffFile>) -> Self {
        self.diff = Some(diff);
        self
    }
    /// Expand the card.
    pub fn expand(mut self) -> Self {
        self.expanded = true;
        self
    }
    /// Toggle expanded state.
    pub fn toggle_expanded(&mut self) {
        self.expanded = !self.expanded;
    }
    /// Cheap row estimate.
    pub fn estimated_rows(&self) -> u16 {
        let mut rows: u16 = 2;
        if self.input_summary.is_some() {
            rows = rows.saturating_add(1);
        }
        if let Some(out) = &self.output_summary {
            if self.expanded {
                rows = rows.saturating_add(out.lines().count().max(1) as u16);
            } else {
                rows = rows.saturating_add(1);
            }
        }
        if let Some(diff) = &self.diff {
            let total: usize = diff
                .iter()
                .map(|f| f.hunks.iter().map(|h| h.lines.len()).sum::<usize>())
                .sum();
            if self.expanded {
                rows = rows.saturating_add(total.min(40) as u16);
            } else {
                rows = rows.saturating_add(1);
            }
        }
        rows
    }
    /// Snapshot.
    pub fn snapshot(&self) -> String {
        let status = self.status.glyph();
        let input = self.input_summary.as_deref().unwrap_or("");
        let out = self.output_summary.as_deref().unwrap_or("");
        format!(
            "tool[{}] {} {} input='{}' output_lines={} diff_files={}",
            self.tool_id,
            status,
            self.name,
            input,
            out.lines().count(),
            self.diff.as_ref().map(|d| d.len()).unwrap_or(0)
        )
    }
}

impl Widget for &ToolCard {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let header = Line::from(vec![
            Span::styled(
                format!(" {} ", self.status.glyph()),
                Style::default().fg(self.status.color()),
            ),
            Span::styled(
                self.name.clone(),
                Style::default()
                    .fg(COLOR_ACCENT)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::styled(
                format!("[{}]", self.tool_id),
                Style::default().fg(COLOR_TEXT_MUTED),
            ),
        ]);

        let mut lines = vec![header];
        if let Some(input) = &self.input_summary {
            lines.push(Line::from(vec![
                Span::styled("  $ ", Style::default().fg(COLOR_TEXT_MUTED)),
                Span::styled(input.clone(), Style::default().fg(COLOR_TEXT)),
            ]));
        }
        if let Some(out) = &self.output_summary {
            let preview: Vec<&str> = if self.expanded {
                out.lines().collect()
            } else {
                out.lines().take(1).collect()
            };
            for raw in preview {
                lines.push(Line::from(vec![
                    Span::styled("  | ", Style::default().fg(COLOR_BORDER)),
                    Span::styled(raw.to_string(), Style::default().fg(COLOR_TEXT_MUTED)),
                ]));
            }
            if !self.expanded && out.lines().count() > 1 {
                lines.push(Line::from(Span::styled(
                    format!(
                        "  ↳ {} more lines (Ctrl+F to expand)",
                        out.lines().count().saturating_sub(1)
                    ),
                    Style::default().fg(COLOR_TEXT_MUTED),
                )));
            }
        }
        if let Some(diff) = &self.diff {
            render_diff(diff, self.expanded, &mut lines);
        }
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .render(area, buf);
    }
}

fn render_diff(diff: &[DiffFile], expanded: bool, lines: &mut Vec<Line<'_>>) {
    if expanded {
        for file in diff {
            lines.push(Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(
                    file.filename.clone(),
                    Style::default()
                        .fg(COLOR_ACCENT)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("  "),
                Span::styled(
                    format!("+{}", file.additions),
                    Style::default().fg(COLOR_DIFF_ADD),
                ),
                Span::raw(" "),
                Span::styled(
                    format!("-{}", file.deletions),
                    Style::default().fg(COLOR_DIFF_DEL),
                ),
            ]));
            for hunk in &file.hunks {
                for line in &hunk.lines {
                    let style = match line.kind {
                        DiffLineKind::Add => Style::default().fg(COLOR_DIFF_ADD),
                        DiffLineKind::Del => Style::default().fg(COLOR_DIFF_DEL),
                        DiffLineKind::Ctx => Style::default().fg(COLOR_TEXT_MUTED),
                    };
                    let sign = match line.kind {
                        DiffLineKind::Add => "+",
                        DiffLineKind::Del => "-",
                        DiffLineKind::Ctx => " ",
                    };
                    lines.push(Line::from(vec![
                        Span::raw("    "),
                        Span::styled(sign, style),
                        Span::raw(" "),
                        Span::styled(line.text.clone(), style),
                    ]));
                }
            }
        }
    } else {
        let total_files = diff.len();
        let total_add: usize = diff.iter().map(|f| f.additions).sum();
        let total_del: usize = diff.iter().map(|f| f.deletions).sum();
        lines.push(Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(
                format!("{total_files} file"),
                Style::default().fg(COLOR_TEXT_MUTED),
            ),
            Span::raw(" "),
            Span::styled(format!("+{total_add}"), Style::default().fg(COLOR_DIFF_ADD)),
            Span::raw(" "),
            Span::styled(format!("-{total_del}"), Style::default().fg(COLOR_DIFF_DEL)),
        ]));
    }
}
