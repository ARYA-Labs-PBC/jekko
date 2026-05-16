//! Sidebar panel and assorted footer/banner widgets used by the session
//! route. Ports `routes/session/sidebar.tsx`, `subagent-footer.tsx`,
//! `daemon-banner.tsx`, and the small sticky-bottom indicator pill.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget, Wrap};

const COLOR_TEXT: Color = Color::Rgb(0xd8, 0xde, 0xe9);
const COLOR_TEXT_MUTED: Color = Color::Rgb(0x7d, 0x85, 0x90);
const COLOR_ACCENT: Color = Color::Rgb(0xd4, 0xa8, 0x43);
const COLOR_PANEL: Color = Color::Rgb(0x12, 0x15, 0x1c);
const COLOR_BORDER: Color = Color::Rgb(0x3a, 0x40, 0x4a);
const COLOR_SUCCESS: Color = Color::Rgb(0x8a, 0xc8, 0x6a);
const COLOR_WARN: Color = Color::Rgb(0xf5, 0xa6, 0x23);
const COLOR_ERROR: Color = Color::Rgb(0xe0, 0x6c, 0x75);

/// Daemon connection status. Used by the banner and sidebar pulse.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum DaemonStatus {
    /// Daemon healthy and connected.
    Online,
    /// Reconnecting.
    Reconnecting,
    /// Disconnected.
    Offline,
    /// Daemon reports an error.
    Error,
}

impl DaemonStatus {
    fn color(self) -> Color {
        match self {
            DaemonStatus::Online => COLOR_SUCCESS,
            DaemonStatus::Reconnecting => COLOR_WARN,
            DaemonStatus::Offline => COLOR_TEXT_MUTED,
            DaemonStatus::Error => COLOR_ERROR,
        }
    }
    fn label(self) -> &'static str {
        match self {
            DaemonStatus::Online => "online",
            DaemonStatus::Reconnecting => "reconnecting",
            DaemonStatus::Offline => "offline",
            DaemonStatus::Error => "error",
        }
    }
}

/// Sidebar panel state. Carries the bits the JS sidebar reads from `useSync`
/// already pre-resolved to plain strings.
#[derive(Clone, Debug)]
pub struct SidebarPanel {
    /// Session title.
    pub title: String,
    /// Stable session id (shown muted under the title).
    pub session_id: Option<String>,
    /// Workspace label (e.g. `"jekko (main)"`).
    pub workspace: Option<String>,
    /// Daemon status.
    pub daemon_status: DaemonStatus,
    /// Optional daemon banner line (e.g. forever-loop summary).
    pub daemon_banner: Option<String>,
    /// Optional sidebar footer label (Jekko version, etc.).
    pub footer: Option<String>,
    /// Width hint — sidebar renders at most this width.
    pub width: u16,
}

impl SidebarPanel {
    /// Build a sidebar panel.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            session_id: None,
            workspace: None,
            daemon_status: DaemonStatus::Online,
            daemon_banner: None,
            footer: None,
            width: 42,
        }
    }
    /// Attach a session id label.
    pub fn with_session_id(mut self, id: impl Into<String>) -> Self {
        self.session_id = Some(id.into());
        self
    }
    /// Attach a workspace label.
    pub fn with_workspace(mut self, workspace: impl Into<String>) -> Self {
        self.workspace = Some(workspace.into());
        self
    }
    /// Attach a daemon status indicator.
    pub fn with_daemon_status(mut self, status: DaemonStatus) -> Self {
        self.daemon_status = status;
        self
    }
    /// Attach a daemon banner.
    pub fn with_daemon_banner(mut self, banner: impl Into<String>) -> Self {
        self.daemon_banner = Some(banner.into());
        self
    }
    /// Attach a footer line.
    pub fn with_footer(mut self, footer: impl Into<String>) -> Self {
        self.footer = Some(footer.into());
        self
    }
    /// Override the panel width.
    pub fn with_width(mut self, width: u16) -> Self {
        self.width = width;
        self
    }
    /// Snapshot.
    pub fn snapshot(&self) -> String {
        format!(
            "sidebar[{}|daemon={}] {} ws={:?}",
            self.session_id.as_deref().unwrap_or("--"),
            self.daemon_status.label(),
            self.title,
            self.workspace
        )
    }
}

impl Widget for &SidebarPanel {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .borders(Borders::LEFT)
            .border_style(Style::default().fg(COLOR_BORDER))
            .style(Style::default().bg(COLOR_PANEL));
        let inner = block.inner(area);
        block.render(area, buf);

        let mut lines: Vec<Line> = Vec::new();
        lines.push(Line::from(Span::styled(
            self.title.clone(),
            Style::default().fg(COLOR_TEXT).add_modifier(Modifier::BOLD),
        )));
        if let Some(id) = &self.session_id {
            lines.push(Line::from(Span::styled(
                id.clone(),
                Style::default().fg(COLOR_TEXT_MUTED),
            )));
        }
        if let Some(ws) = &self.workspace {
            lines.push(Line::from(vec![
                Span::styled("⚑ ", Style::default().fg(COLOR_ACCENT)),
                Span::styled(ws.clone(), Style::default().fg(COLOR_TEXT_MUTED)),
            ]));
        }
        lines.push(Line::from(Span::raw("")));
        lines.push(Line::from(vec![
            Span::styled("● ", Style::default().fg(self.daemon_status.color())),
            Span::styled(
                format!("daemon {}", self.daemon_status.label()),
                Style::default().fg(COLOR_TEXT),
            ),
        ]));
        if let Some(banner) = &self.daemon_banner {
            lines.push(Line::from(Span::styled(
                banner.clone(),
                Style::default().fg(COLOR_WARN),
            )));
        }
        if let Some(footer) = &self.footer {
            lines.push(Line::from(Span::raw("")));
            lines.push(Line::from(Span::styled(
                footer.clone(),
                Style::default().fg(COLOR_TEXT_MUTED),
            )));
        }
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .render(inner, buf);
    }
}

/// Subagent footer row. Ports `subagent-footer.tsx`.
#[derive(Clone, Debug)]
pub struct SubagentFooter {
    /// Subagent label (e.g. `"Builder"`).
    pub label: String,
    /// 1-based sibling index.
    pub index: u32,
    /// Total sibling count (0 when no parent context).
    pub total: u32,
    /// Optional context-token line (e.g. `"12,345 (8%)"`).
    pub context: Option<String>,
    /// Optional cost line (e.g. `"$0.23"`).
    pub cost: Option<String>,
}

impl SubagentFooter {
    /// Build a subagent footer.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            index: 0,
            total: 0,
            context: None,
            cost: None,
        }
    }
    /// Attach sibling indexing.
    pub fn with_position(mut self, index: u32, total: u32) -> Self {
        self.index = index;
        self.total = total;
        self
    }
    /// Attach the context-token line.
    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }
    /// Attach the cost line.
    pub fn with_cost(mut self, cost: impl Into<String>) -> Self {
        self.cost = Some(cost.into());
        self
    }
    /// Snapshot.
    pub fn snapshot(&self) -> String {
        format!(
            "subagent_footer[{} {}/{}] context={:?} cost={:?}",
            self.label, self.index, self.total, self.context, self.cost
        )
    }
}

impl Widget for &SubagentFooter {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .borders(Borders::LEFT)
            .border_style(Style::default().fg(COLOR_BORDER))
            .style(Style::default().bg(COLOR_PANEL));
        let inner = block.inner(area);
        block.render(area, buf);

        let mut spans: Vec<Span<'static>> = vec![
            Span::styled(" ", Style::default()),
            Span::styled(
                self.label.clone(),
                Style::default().fg(COLOR_TEXT).add_modifier(Modifier::BOLD),
            ),
        ];
        if self.total > 0 {
            spans.push(Span::raw(" "));
            spans.push(Span::styled(
                format!("({} of {})", self.index, self.total),
                Style::default().fg(COLOR_TEXT_MUTED),
            ));
        }
        if let Some(ctx) = &self.context {
            spans.push(Span::raw(" · "));
            spans.push(Span::styled(
                ctx.clone(),
                Style::default().fg(COLOR_TEXT_MUTED),
            ));
        }
        if let Some(cost) = &self.cost {
            spans.push(Span::raw(" · "));
            spans.push(Span::styled(
                cost.clone(),
                Style::default().fg(COLOR_TEXT_MUTED),
            ));
        }
        Paragraph::new(Line::from(spans)).render(inner, buf);
    }
}

/// Sticky-bottom indicator pill. Shown when the user has scrolled away from
/// the tail. Renders a tiny right-aligned banner.
#[derive(Clone, Debug)]
pub struct StickyBottomIndicator {
    /// How many new entries arrived while scrolled away.
    pub pending: u32,
}

impl StickyBottomIndicator {
    /// Build a new indicator.
    pub fn new(pending: u32) -> Self {
        Self { pending }
    }
}

impl Widget for &StickyBottomIndicator {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let label = if self.pending == 0 {
            "↓ jump to latest".to_string()
        } else {
            format!("↓ {} new · jump to latest", self.pending)
        };
        let para = Paragraph::new(Line::from(Span::styled(
            label,
            Style::default()
                .fg(COLOR_PANEL)
                .bg(COLOR_ACCENT)
                .add_modifier(Modifier::BOLD),
        )));
        para.render(area, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    #[test]
    fn sidebar_snapshot_includes_title() {
        let panel = SidebarPanel::new("session title").with_session_id("sess_abc");
        let snap = panel.snapshot();
        assert!(snap.contains("session title"));
        assert!(snap.contains("sess_abc"));
    }

    #[test]
    fn sidebar_renders_daemon_pulse() {
        let panel = SidebarPanel::new("session")
            .with_daemon_status(DaemonStatus::Online)
            .with_footer("Jekko v1.0".to_string());
        let mut terminal = Terminal::new(TestBackend::new(40, 8)).unwrap();
        terminal
            .draw(|f| f.render_widget(&panel, f.area()))
            .unwrap();
        let buf = terminal.backend().buffer().clone();
        let rendered: String = buf.content.iter().map(|c| c.symbol()).collect();
        assert!(rendered.contains("daemon online"));
        assert!(rendered.contains("Jekko"));
    }

    #[test]
    fn sidebar_renders_offline_status() {
        let panel = SidebarPanel::new("session").with_daemon_status(DaemonStatus::Offline);
        let mut terminal = Terminal::new(TestBackend::new(40, 6)).unwrap();
        terminal
            .draw(|f| f.render_widget(&panel, f.area()))
            .unwrap();
        let buf = terminal.backend().buffer().clone();
        let rendered: String = buf.content.iter().map(|c| c.symbol()).collect();
        assert!(rendered.contains("daemon offline"));
    }

    #[test]
    fn subagent_footer_includes_index() {
        let footer = SubagentFooter::new("Builder")
            .with_position(2, 5)
            .with_context("12k (8%)")
            .with_cost("$0.10");
        let snap = footer.snapshot();
        assert!(snap.contains("Builder"));
        assert!(snap.contains("2/5"));
    }

    #[test]
    fn subagent_footer_renders() {
        let footer = SubagentFooter::new("Builder").with_position(1, 3);
        let mut terminal = Terminal::new(TestBackend::new(40, 2)).unwrap();
        terminal
            .draw(|f| f.render_widget(&footer, f.area()))
            .unwrap();
        let buf = terminal.backend().buffer().clone();
        let rendered: String = buf.content.iter().map(|c| c.symbol()).collect();
        assert!(rendered.contains("Builder"));
        assert!(rendered.contains("1 of 3"));
    }

    #[test]
    fn sticky_indicator_no_pending() {
        let pill = StickyBottomIndicator::new(0);
        let mut terminal = Terminal::new(TestBackend::new(30, 1)).unwrap();
        terminal.draw(|f| f.render_widget(&pill, f.area())).unwrap();
        let buf = terminal.backend().buffer().clone();
        let rendered: String = buf.content.iter().map(|c| c.symbol()).collect();
        assert!(rendered.contains("jump"));
    }

    #[test]
    fn sticky_indicator_with_pending_count() {
        let pill = StickyBottomIndicator::new(3);
        let mut terminal = Terminal::new(TestBackend::new(40, 1)).unwrap();
        terminal.draw(|f| f.render_widget(&pill, f.area())).unwrap();
        let buf = terminal.backend().buffer().clone();
        let rendered: String = buf.content.iter().map(|c| c.symbol()).collect();
        assert!(rendered.contains("3 new"));
    }

    #[test]
    fn daemon_status_label_is_stable() {
        assert_eq!(DaemonStatus::Online.label(), "online");
        assert_eq!(DaemonStatus::Reconnecting.label(), "reconnecting");
        assert_eq!(DaemonStatus::Offline.label(), "offline");
        assert_eq!(DaemonStatus::Error.label(), "error");
    }
}
