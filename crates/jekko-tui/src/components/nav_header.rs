//! Section: Header Chrome
//!
//! Two-row application header: StatusBar (row 1) + NavBar (row 2).
//!
//! ```text
//!  Jnoccio  repo: jnoccio  branch: main  audit: idle         models 78/79
//!  [F1] Chat   [F2] Repo Intel   [F3] History                  [Esc] Back
//! ```
//!
//! The header has no border — it reads as product chrome, not a panel.
//!
//! # Back-compat
//! `NavigationHeader` is kept as a thin wrapper so existing callers compile.
//! New code should use `AppHeader` directly.

use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

use crate::feature_plugins::ShellTab;
use crate::theme;

// ── Constants ────────────────────────────────────────────────────────────────

const GOLD: Color = theme::ACCENT;
const MUTED: Color = theme::TEXT_MUTED;
const TEXT: Color = theme::TEXT;
const BG: Color = theme::BG;

// ── StatusBar (row 1) ────────────────────────────────────────────────────────

/// Row 1 — product identity + current run state.
///
/// ```text
///  Jnoccio  repo: jnoccio  branch: main  audit: idle         models 78/79
/// ```
pub struct StatusBar {
    pub repo_name: String,
    pub branch: String,
    pub audit_status: AuditStatus,
    pub enabled_models: u32,
    pub total_models: u32,
}

#[derive(Clone, Debug, Default)]
pub enum AuditStatus {
    #[default]
    Idle,
    Running,
    Failed,
}

impl AuditStatus {
    fn label(&self) -> &'static str {
        match self {
            AuditStatus::Idle => "idle",
            AuditStatus::Running => "running",
            AuditStatus::Failed => "failed",
        }
    }
    fn color(&self) -> Color {
        match self {
            AuditStatus::Idle => MUTED,
            AuditStatus::Running => theme::INFO,
            AuditStatus::Failed => theme::DANGER,
        }
    }
}

impl Widget for &StatusBar {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Left side: product name + metadata
        let left = vec![
            Span::styled(
                " Jnoccio",
                Style::default().fg(GOLD).add_modifier(Modifier::BOLD),
            ),
            Span::styled("  repo: ", Style::default().fg(MUTED)),
            Span::styled(self.repo_name.clone(), Style::default().fg(TEXT)),
            Span::styled("  branch: ", Style::default().fg(MUTED)),
            Span::styled(self.branch.clone(), Style::default().fg(TEXT)),
            Span::styled("  audit: ", Style::default().fg(MUTED)),
            Span::styled(
                self.audit_status.label(),
                Style::default().fg(self.audit_status.color()),
            ),
        ];

        // Right side: model count — degrade when no models available
        let right_text = if self.total_models > 0 {
            format!("models {}/{}", self.enabled_models, self.total_models)
        } else {
            String::new()
        };

        // Render left-aligned, then right-aligned
        let left_para = Paragraph::new(Line::from(left)).style(Style::default().bg(BG));
        let right_para = Paragraph::new(Line::from(Span::styled(
            right_text,
            Style::default().fg(MUTED).bg(BG),
        )))
        .alignment(Alignment::Right);

        left_para.render(area, buf);
        right_para.render(area, buf);
    }
}

// ── NavBar (row 2) ───────────────────────────────────────────────────────────

/// Row 2 — tab navigation + Back affordance.
///
/// ```text
///  [F1] Chat   [F2] Repo Intel   [F3] History                  [Esc] Back
/// ```
pub struct NavBar {
    pub active_tab: ShellTab,
}

impl NavBar {
    /// Tab entries: (F-key, label, ShellTab)
    const TABS: &'static [(&'static str, &'static str, ShellTab)] = &[
        ("F1", "Chat", ShellTab::Jnoccio),
        ("F2", "Repo Intel", ShellTab::RepoIntel),
        ("F3", "History", ShellTab::History),
    ];
}

impl Widget for &NavBar {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut left_spans: Vec<Span> = vec![Span::raw(" ")];

        for (i, (key, label, tab)) in NavBar::TABS.iter().enumerate() {
            if i > 0 {
                left_spans.push(Span::raw("   "));
            }
            let is_active = *tab == self.active_tab;
            let key_style = if is_active {
                Style::default().fg(GOLD).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(MUTED)
            };
            let label_style = if is_active {
                Style::default().fg(TEXT).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(MUTED)
            };
            left_spans.push(Span::styled(format!("[{key}]"), key_style));
            left_spans.push(Span::raw(" "));
            left_spans.push(Span::styled(*label, label_style));
        }

        let right_text = "[Esc] Back";
        let right_para = Paragraph::new(Line::from(Span::styled(
            right_text,
            Style::default().fg(MUTED).bg(BG),
        )))
        .alignment(Alignment::Right)
        .style(Style::default().bg(BG));
        right_para.render(area, buf);

        let left_para = Paragraph::new(Line::from(left_spans)).style(Style::default().bg(BG));
        left_para.render(area, buf);
    }
}

// ── AppHeader (2 rows) ───────────────────────────────────────────────────────

/// Composite 2-row header. Renders into a 2-row `area`.
pub struct AppHeader {
    pub status: StatusBar,
    pub nav: NavBar,
}

impl AppHeader {
    pub fn new(
        repo_name: impl Into<String>,
        branch: impl Into<String>,
        audit_status: AuditStatus,
        enabled_models: u32,
        total_models: u32,
        active_tab: ShellTab,
    ) -> Self {
        Self {
            status: StatusBar {
                repo_name: repo_name.into(),
                branch: branch.into(),
                audit_status,
                enabled_models,
                total_models,
            },
            nav: NavBar { active_tab },
        }
    }
}

impl Widget for &AppHeader {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height == 0 {
            return;
        }
        if area.height == 1 {
            self.status.render(area, buf);
            return;
        }
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Length(1)])
            .split(area);
        self.status.render(rows[0], buf);
        self.nav.render(rows[1], buf);
    }
}

// ── Back-compat shim ────────────────────────────────────────────────────────

/// Kept for compile compatibility. New code should use `AppHeader`.
#[derive(Clone, Debug)]
pub struct NavigationTab {
    pub label: &'static str,
    pub shortcut: &'static str,
    pub visible: bool,
    pub active: bool,
}

/// Kept for compile compatibility. Renders an empty row.
#[derive(Clone, Debug)]
pub struct NavigationHeader {
    pub tabs: Vec<NavigationTab>,
}

impl NavigationHeader {
    pub fn home_back_jnoccio(
        _home_active: bool,
        _jnoccio_visible: bool,
        _jnoccio_active: bool,
    ) -> Self {
        Self { tabs: vec![] }
    }
}

impl Widget for &NavigationHeader {
    fn render(self, _area: Rect, _buf: &mut Buffer) {
        // Intentionally empty — replaced by AppHeader.
    }
}
