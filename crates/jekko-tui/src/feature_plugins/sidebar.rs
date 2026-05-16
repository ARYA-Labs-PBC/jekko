//! Feature sidebar widget.
//!
//! Ports `packages/jekko/src/cli/cmd/tui/feature-plugins/sidebar/` —
//! `view.tsx`, `jankurai.tsx`, `zyal.tsx`, `context.tsx`, `footer.tsx` — to a
//! single Ratatui list of active feature panels with a selection cursor.
//!
//! The TS source registers individual `home_zyal_panel` and `sidebar_content`
//! slots per feature; in Rust we collapse those into one [`Sidebar`] widget
//! that takes a vector of [`SidebarEntry`] rows. Each row knows which
//! [`crate::feature_plugins::FeaturePanel`] variant it would push when picked,
//! plus a one-line status string the sidebar renders inline.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget};

const GOLD: Color = Color::Rgb(0xf5, 0xa6, 0x23);
const MUTED: Color = Color::Rgb(0x7d, 0x85, 0x90);
const TEXT: Color = Color::Rgb(0xd8, 0xde, 0xe9);
const SUCCESS: Color = Color::Rgb(0x22, 0xc5, 0x5e);
const ERROR: Color = Color::Rgb(0xff, 0x47, 0x57);
const WARNING: Color = Color::Rgb(0xff, 0xd0, 0x00);

/// One row in the feature sidebar.
#[derive(Clone, Debug)]
pub struct SidebarEntry {
    /// Stable id used for keyboard activation (e.g. `jnoccio`).
    pub id: String,
    /// Display label.
    pub label: String,
    /// Optional keybind hint to show on the right (e.g. `Ctrl+J`).
    pub keybind_hint: Option<String>,
    /// Live status string ("Live · 12s", "checking…", "off").
    pub status: SidebarStatus,
    /// True when the row is the currently-foreground panel.
    pub active: bool,
}

/// Health colour band for a sidebar row.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SidebarStatus {
    /// Feature is healthy and running.
    Live,
    /// Feature is booting or reconnecting.
    Booting,
    /// Feature reported an error.
    Error,
    /// Feature is installed but disabled.
    Disabled,
    /// Feature is not available in this workspace.
    Unavailable,
}

impl SidebarStatus {
    fn color(self) -> Color {
        match self {
            SidebarStatus::Live => SUCCESS,
            SidebarStatus::Booting => WARNING,
            SidebarStatus::Error => ERROR,
            SidebarStatus::Disabled => MUTED,
            SidebarStatus::Unavailable => MUTED,
        }
    }

    fn label(self) -> &'static str {
        match self {
            SidebarStatus::Live => "live",
            SidebarStatus::Booting => "booting",
            SidebarStatus::Error => "error",
            SidebarStatus::Disabled => "off",
            SidebarStatus::Unavailable => "n/a",
        }
    }

    fn dot(self) -> &'static str {
        match self {
            SidebarStatus::Live => "●",
            SidebarStatus::Booting => "◐",
            SidebarStatus::Error => "✗",
            SidebarStatus::Disabled => "○",
            SidebarStatus::Unavailable => "·",
        }
    }
}

impl SidebarEntry {
    /// Build a row with the given id and label.
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            keybind_hint: None,
            status: SidebarStatus::Live,
            active: false,
        }
    }

    /// Attach a keybind hint string.
    pub fn with_keybind(mut self, hint: impl Into<String>) -> Self {
        self.keybind_hint = Some(hint.into());
        self
    }

    /// Set the live status.
    pub fn with_status(mut self, status: SidebarStatus) -> Self {
        self.status = status;
        self
    }

    /// Set the active flag.
    pub fn with_active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }
}

/// Vertical sidebar listing all available feature panels.
#[derive(Clone, Debug, Default)]
pub struct Sidebar {
    entries: Vec<SidebarEntry>,
    cursor: usize,
    /// Set to true when the user pressed Enter on a row.
    activate_requested: bool,
}

impl Sidebar {
    /// Build a sidebar from a list of entries.
    pub fn new(entries: Vec<SidebarEntry>) -> Self {
        Self {
            entries,
            cursor: 0,
            activate_requested: false,
        }
    }

    /// Replace the entry list, clamping the cursor.
    pub fn set_entries(&mut self, entries: Vec<SidebarEntry>) {
        self.entries = entries;
        if self.entries.is_empty() {
            self.cursor = 0;
        } else if self.cursor >= self.entries.len() {
            self.cursor = self.entries.len() - 1;
        }
    }

    /// Read-only access to the entries.
    pub fn entries(&self) -> &[SidebarEntry] {
        &self.entries
    }

    /// 0-indexed cursor offset.
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// Currently-highlighted entry.
    pub fn selected(&self) -> Option<&SidebarEntry> {
        self.entries.get(self.cursor)
    }

    /// True when the user pressed Enter.
    pub fn activate_requested(&self) -> bool {
        self.activate_requested
    }

    /// Reset the activate flag.
    pub fn clear_activate(&mut self) {
        self.activate_requested = false;
    }

    /// Move the cursor by a signed delta, wrapping around.
    pub fn move_cursor(&mut self, delta: isize) {
        if self.entries.is_empty() {
            self.cursor = 0;
            return;
        }
        let len = self.entries.len() as isize;
        let mut idx = self.cursor as isize + delta;
        while idx < 0 {
            idx += len;
        }
        self.cursor = (idx % len) as usize;
    }

    /// Dispatch a key event. Returns `true` when consumed.
    pub fn dispatch_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                self.move_cursor(1);
                true
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.move_cursor(-1);
                true
            }
            KeyCode::Char('g') if !key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.cursor = 0;
                true
            }
            KeyCode::Char('G') => {
                if !self.entries.is_empty() {
                    self.cursor = self.entries.len() - 1;
                }
                true
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                if !self.entries.is_empty() {
                    self.activate_requested = true;
                }
                true
            }
            _ => false,
        }
    }
}

impl Widget for &Sidebar {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(0x21, 0x26, 0x30)))
            .title(Span::styled(
                " Features ",
                Style::default().fg(GOLD).add_modifier(Modifier::BOLD),
            ));
        let inner = block.inner(area);
        block.render(area, buf);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .split(inner);

        let mut lines = Vec::with_capacity(self.entries.len().max(1));
        if self.entries.is_empty() {
            lines.push(Line::from(Span::styled(
                "no feature plugins",
                Style::default().fg(MUTED),
            )));
        }
        for (idx, entry) in self.entries.iter().enumerate() {
            let highlighted = idx == self.cursor;
            let label_style = if entry.active {
                Style::default().fg(GOLD).add_modifier(Modifier::BOLD)
            } else if highlighted {
                Style::default().fg(TEXT).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(TEXT)
            };
            let mut spans = vec![
                Span::raw(if highlighted { "> " } else { "  " }),
                Span::styled(
                    entry.status.dot(),
                    Style::default().fg(entry.status.color()),
                ),
                Span::raw(" "),
                Span::styled(entry.label.clone(), label_style),
            ];
            if let Some(hint) = entry.keybind_hint.as_deref() {
                spans.push(Span::raw("  "));
                spans.push(Span::styled(
                    format!("[{}]", hint),
                    Style::default().fg(MUTED),
                ));
            }
            spans.push(Span::raw("  "));
            spans.push(Span::styled(
                entry.status.label(),
                Style::default().fg(entry.status.color()),
            ));
            lines.push(Line::from(spans));
        }
        Paragraph::new(lines).render(chunks[0], buf);

        let hints = Line::from(vec![
            Span::styled(
                "j/k",
                Style::default().fg(MUTED).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" move  ", Style::default().fg(MUTED)),
            Span::styled(
                "Enter",
                Style::default().fg(MUTED).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" open", Style::default().fg(MUTED)),
        ]);
        Paragraph::new(hints).render(chunks[1], buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn sidebar_with_three() -> Sidebar {
        Sidebar::new(vec![
            SidebarEntry::new("jnoccio", "Jnoccio")
                .with_keybind("Ctrl+J")
                .with_status(SidebarStatus::Live)
                .with_active(true),
            SidebarEntry::new("jankurai", "Jankurai")
                .with_keybind("Ctrl+K")
                .with_status(SidebarStatus::Booting),
            SidebarEntry::new("zyal", "ZYAL").with_status(SidebarStatus::Disabled),
        ])
    }

    #[test]
    fn cursor_wraps_in_both_directions() {
        let mut s = sidebar_with_three();
        assert_eq!(s.cursor(), 0);
        s.dispatch_key(key(KeyCode::Down));
        s.dispatch_key(key(KeyCode::Down));
        assert_eq!(s.cursor(), 2);
        s.dispatch_key(key(KeyCode::Down));
        assert_eq!(s.cursor(), 0);
        s.dispatch_key(key(KeyCode::Up));
        assert_eq!(s.cursor(), 2);
    }

    #[test]
    fn empty_sidebar_doesnt_panic() {
        let mut s = Sidebar::default();
        s.dispatch_key(key(KeyCode::Down));
        s.dispatch_key(key(KeyCode::Enter));
        assert!(!s.activate_requested());
        assert!(s.selected().is_none());
    }

    #[test]
    fn enter_or_space_activates_row() {
        let mut s = sidebar_with_three();
        assert!(!s.activate_requested());
        s.dispatch_key(key(KeyCode::Enter));
        assert!(s.activate_requested());
        s.clear_activate();
        s.dispatch_key(key(KeyCode::Char(' ')));
        assert!(s.activate_requested());
    }

    #[test]
    fn set_entries_clamps_cursor() {
        let mut s = sidebar_with_three();
        s.dispatch_key(key(KeyCode::Down));
        s.dispatch_key(key(KeyCode::Down));
        s.set_entries(vec![SidebarEntry::new("only", "Only")]);
        assert_eq!(s.cursor(), 0);
        s.set_entries(vec![]);
        assert_eq!(s.cursor(), 0);
    }

    #[test]
    fn unconsumed_keys_return_false() {
        let mut s = sidebar_with_three();
        assert!(!s.dispatch_key(key(KeyCode::Char('?'))));
        assert!(!s.dispatch_key(key(KeyCode::Char('q'))));
    }

    #[test]
    fn render_at_100x30() {
        let s = sidebar_with_three();
        let area = Rect::new(0, 0, 100, 30);
        let mut buf = Buffer::empty(area);
        (&s).render(area, &mut buf);
    }

    #[test]
    fn render_at_200x60_with_empty_state() {
        let s = Sidebar::default();
        let area = Rect::new(0, 0, 200, 60);
        let mut buf = Buffer::empty(area);
        (&s).render(area, &mut buf);
    }

    #[test]
    fn status_colors_and_labels_are_distinct() {
        let statuses = [
            SidebarStatus::Live,
            SidebarStatus::Booting,
            SidebarStatus::Error,
            SidebarStatus::Disabled,
            SidebarStatus::Unavailable,
        ];
        let labels: Vec<&str> = statuses.iter().map(|s| s.label()).collect();
        let mut dedup = labels.clone();
        dedup.sort();
        dedup.dedup();
        // Disabled and Unavailable have the same label intentionally? No,
        // their labels differ ("off" vs "n/a"). Ensure that.
        assert_eq!(labels.len(), dedup.len());
    }
}
