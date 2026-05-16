//! Feature plugin panels for the Jekko TUI.
//!
//! Ports the per-feature dashboards from
//! `packages/jekko/src/cli/cmd/tui/feature-plugins/` (Jnoccio, Jankurai, ZYAL,
//! plugin manager, sidebar) to native Ratatui widgets.
//!
//! Each panel is a self-contained Ratatui `Widget` that the App can switch on
//! via [`FeaturePanel`]. Panels also expose `dispatch_key` so the App can route
//! key events to whichever panel currently owns the foreground.
//!
//! Integrate `crates/jankurai-runner` as a workspace path dep so the Jankurai
//! panel can read a live audit snapshot. The same applies to `crates/zyalc`
//! for ZYAL runbook parsing. Until those runtimes are wired, these panels use
//! snapshot structs so they can be tested independently.
//!
//! The orchestrator must add `pub mod feature_plugins;` and
//! `pub use feature_plugins::*;` to `crates/jekko-tui/src/lib.rs` when the
//! module becomes part of the crate root.

use crossterm::event::KeyEvent;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::Widget;

pub mod jankurai;
pub mod jnoccio;
pub mod plugin_manager;
pub mod shell_layout;
pub mod sidebar;
pub mod zyal;

pub use jankurai::{JankuraiPanel, JankuraiSnapshot};
pub use jnoccio::{JnoccioPanel, JnoccioSnapshot, JnoccioTab};
pub use plugin_manager::{PluginManager, PluginRow, PluginRowKind};
pub use sidebar::{Sidebar, SidebarEntry};
pub use zyal::{ZyalPanel, ZyalSnapshot};

/// Top-level tab selector for the Shell route's LEFT panel cluster.
///
/// Mirrors the TS-era `shell` view that surfaced `Jnoccio`, `Repo-Intel`
/// (jankurai), and a session `History` list as three sibling tabs the user
/// could cycle with `Tab` / `Shift+Tab` or jump to with `1` / `2` / `3`.
///
/// State persisted on [`crate::app::App::shell_tab`] in Phase 1 of the Shell
/// rebuild — the actual panel composition lands in Phase 2B.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ShellTab {
    /// Jnoccio Fusion dashboard.
    #[default]
    Jnoccio,
    /// Repo-Intel (jankurai audit) dashboard.
    RepoIntel,
    /// Session history list.
    History,
}

impl ShellTab {
    /// Wrapping next tab (Jnoccio → RepoIntel → History → Jnoccio).
    pub fn next(self) -> Self {
        match self {
            ShellTab::Jnoccio => ShellTab::RepoIntel,
            ShellTab::RepoIntel => ShellTab::History,
            ShellTab::History => ShellTab::Jnoccio,
        }
    }

    /// Wrapping previous tab.
    pub fn prev(self) -> Self {
        match self {
            ShellTab::Jnoccio => ShellTab::History,
            ShellTab::RepoIntel => ShellTab::Jnoccio,
            ShellTab::History => ShellTab::RepoIntel,
        }
    }

    /// Lookup the tab from a 0-based ordinal (used by the `1` / `2` / `3`
    /// keybinds, which translate to `0` / `1` / `2` here).
    pub fn from_index(i: usize) -> Option<Self> {
        match i {
            0 => Some(ShellTab::Jnoccio),
            1 => Some(ShellTab::RepoIntel),
            2 => Some(ShellTab::History),
            _ => None,
        }
    }

    /// 0-based ordinal.
    pub fn index(self) -> usize {
        match self {
            ShellTab::Jnoccio => 0,
            ShellTab::RepoIntel => 1,
            ShellTab::History => 2,
        }
    }

    /// Single-character indicator used in the narrow tab bar.
    pub fn short(self) -> char {
        match self {
            ShellTab::Jnoccio => 'J',
            ShellTab::RepoIntel => 'R',
            ShellTab::History => 'H',
        }
    }

    /// Long-form label shown in the wide tab bar.
    pub fn label(self) -> &'static str {
        match self {
            ShellTab::Jnoccio => "Jnoccio",
            ShellTab::RepoIntel => "Repo-Intel",
            ShellTab::History => "History",
        }
    }
}

/// Switchable feature panel. The TUI App holds at most one active panel of this
/// enum and forwards keystrokes via [`FeaturePanel::dispatch_key`].
#[derive(Clone, Debug)]
pub enum FeaturePanel {
    /// Jnoccio Fusion dashboard (Phase 11 — packet J).
    Jnoccio(JnoccioPanel),
    /// Jankurai lints / score dashboard (Phase 11 — packet J).
    Jankurai(JankuraiPanel),
    /// ZYAL runbook executor panel (Phase 11 — packet J).
    Zyal(ZyalPanel),
    /// Internal/external plugin browser dialog (Phase 11 — packet J).
    PluginManager(PluginManager),
}

impl FeaturePanel {
    /// Forward a key event to the active panel.
    ///
    /// Returns `true` when the panel consumed the key — App code should NOT
    /// fall through to global keymaps in that case.
    pub fn dispatch_key(&mut self, key: KeyEvent) -> bool {
        match self {
            FeaturePanel::Jnoccio(p) => p.dispatch_key(key),
            FeaturePanel::Jankurai(p) => p.dispatch_key(key),
            FeaturePanel::Zyal(p) => p.dispatch_key(key),
            FeaturePanel::PluginManager(p) => p.dispatch_key(key),
        }
    }

    /// Short id useful for routing and trace logs.
    pub fn id(&self) -> &'static str {
        match self {
            FeaturePanel::Jnoccio(_) => "jnoccio",
            FeaturePanel::Jankurai(_) => "jankurai",
            FeaturePanel::Zyal(_) => "zyal",
            FeaturePanel::PluginManager(_) => "plugin_manager",
        }
    }
}

impl Widget for &FeaturePanel {
    fn render(self, area: Rect, buf: &mut Buffer) {
        match self {
            FeaturePanel::Jnoccio(p) => p.render(area, buf),
            FeaturePanel::Jankurai(p) => p.render(area, buf),
            FeaturePanel::Zyal(p) => p.render(area, buf),
            FeaturePanel::PluginManager(p) => p.render(area, buf),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyModifiers};
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn feature_panel_dispatches_to_inner_jnoccio() {
        let mut panel = FeaturePanel::Jnoccio(JnoccioPanel::new(JnoccioSnapshot::default()));
        // `?` toggles help in the Jnoccio panel.
        assert!(panel.dispatch_key(key(KeyCode::Char('?'))));
        if let FeaturePanel::Jnoccio(inner) = &panel {
            assert!(inner.help_open());
        } else {
            panic!("expected Jnoccio variant");
        }
    }

    #[test]
    fn feature_panel_dispatches_to_inner_plugin_manager() {
        let rows = vec![
            PluginRow::internal("internal:a", "0.1.0").with_themes(1),
            PluginRow::internal("internal:b", "0.2.0"),
        ];
        let mut panel = FeaturePanel::PluginManager(PluginManager::new(rows));
        assert!(panel.dispatch_key(key(KeyCode::Down)));
        if let FeaturePanel::PluginManager(inner) = &panel {
            assert_eq!(inner.cursor(), 1);
        } else {
            panic!("expected PluginManager variant");
        }
    }

    #[test]
    fn feature_panel_renders_each_variant() {
        let area = Rect::new(0, 0, 100, 30);
        let mut buf = Buffer::empty(area);

        let panels = vec![
            FeaturePanel::Jnoccio(JnoccioPanel::new(JnoccioSnapshot::default())),
            FeaturePanel::Jankurai(JankuraiPanel::new(JankuraiSnapshot::default())),
            FeaturePanel::Zyal(ZyalPanel::new(ZyalSnapshot::default())),
            FeaturePanel::PluginManager(PluginManager::new(vec![])),
        ];
        for p in &panels {
            p.render(area, &mut buf);
        }
    }

    #[test]
    fn shell_tab_cycles_forward_and_backward() {
        assert_eq!(ShellTab::default(), ShellTab::Jnoccio);
        assert_eq!(ShellTab::Jnoccio.next(), ShellTab::RepoIntel);
        assert_eq!(ShellTab::RepoIntel.next(), ShellTab::History);
        assert_eq!(ShellTab::History.next(), ShellTab::Jnoccio);
        assert_eq!(ShellTab::Jnoccio.prev(), ShellTab::History);
        assert_eq!(ShellTab::RepoIntel.prev(), ShellTab::Jnoccio);
        assert_eq!(ShellTab::History.prev(), ShellTab::RepoIntel);
    }

    #[test]
    fn shell_tab_index_round_trips() {
        for tab in [ShellTab::Jnoccio, ShellTab::RepoIntel, ShellTab::History] {
            assert_eq!(ShellTab::from_index(tab.index()), Some(tab));
        }
        assert_eq!(ShellTab::from_index(3), None);
    }

    #[test]
    fn shell_tab_labels_match_design() {
        assert_eq!(ShellTab::Jnoccio.short(), 'J');
        assert_eq!(ShellTab::RepoIntel.short(), 'R');
        assert_eq!(ShellTab::History.short(), 'H');
        assert_eq!(ShellTab::Jnoccio.label(), "Jnoccio");
        assert_eq!(ShellTab::RepoIntel.label(), "Repo-Intel");
        assert_eq!(ShellTab::History.label(), "History");
    }

    #[test]
    fn feature_panel_ids_are_stable() {
        assert_eq!(
            FeaturePanel::Jnoccio(JnoccioPanel::new(JnoccioSnapshot::default())).id(),
            "jnoccio"
        );
        assert_eq!(
            FeaturePanel::Jankurai(JankuraiPanel::new(JankuraiSnapshot::default())).id(),
            "jankurai"
        );
        assert_eq!(
            FeaturePanel::Zyal(ZyalPanel::new(ZyalSnapshot::default())).id(),
            "zyal"
        );
        assert_eq!(
            FeaturePanel::PluginManager(PluginManager::new(vec![])).id(),
            "plugin_manager"
        );
    }
}
