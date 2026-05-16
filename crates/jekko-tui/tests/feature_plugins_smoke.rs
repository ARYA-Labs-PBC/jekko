//! Integration smoke tests for the Packet J feature plugin modules.
//!
//! The orchestrator wires `pub mod feature_plugins;` into `lib.rs` after this
//! subagent completes, so during development the tests use `#[path]` includes
//! to pull each module directly. Once `lib.rs` is wired the unit tests inside
//! each source file run via `cargo test -p jekko-tui --lib feature_plugins`;
//! the smoke tests here continue to live as an outer-crate sanity layer.
//!
//! `plugin_manager.rs` references `crate::dialog::frame::*`. When pulled into
//! the test crate via `#[path]`, `crate` resolves to this test binary, not
//! `jekko_tui`, so we publish a `dialog` shim here that re-exports the
//! `DialogFrame` helpers from `jekko_tui::dialog`.

pub mod dialog {
    pub mod frame {
        pub use jekko_tui::dialog::frame::{render_frame, DialogFrame};
    }
}

pub mod theme {
    pub use jekko_tui::theme::*;
}

pub mod action {
    pub use jekko_tui::action::{Action, AuditFinding, AuditSummary};
}

#[path = "../src/feature_plugins/jankurai/mod.rs"]
mod jankurai;
#[path = "../src/feature_plugins/jnoccio/mod.rs"]
mod jnoccio;
#[path = "../src/feature_plugins/plugin_manager/mod.rs"]
mod plugin_manager;
#[path = "../src/feature_plugins/sidebar.rs"]
mod sidebar;
#[path = "../src/feature_plugins/zyal/mod.rs"]
mod zyal;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::Widget;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

#[test]
fn jnoccio_panel_renders_and_dispatches() {
    let mut panel = jnoccio::JnoccioPanel::new(jnoccio::JnoccioSnapshot::default());
    panel.set_connection(jnoccio::JnoccioConnection::Live);
    assert!(panel.dispatch_key(key(KeyCode::Char('5'))));
    assert_eq!(panel.tab(), jnoccio::JnoccioTab::Feed);

    let area = Rect::new(0, 0, 100, 30);
    let mut buf = Buffer::empty(area);
    (&panel).render(area, &mut buf);
}

#[test]
fn jankurai_panel_renders() {
    let panel = jankurai::JankuraiPanel::new(jankurai::JankuraiSnapshot {
        score: Some(80.0),
        history: vec![70.0, 72.0, 75.0, 78.0, 80.0],
        ..Default::default()
    });
    let area = Rect::new(0, 0, 100, 30);
    let mut buf = Buffer::empty(area);
    (&panel).render(area, &mut buf);
}

#[test]
fn zyal_panel_consumes_q() {
    let mut panel = zyal::ZyalPanel::new(zyal::ZyalSnapshot::default());
    assert!(panel.dispatch_key(key(KeyCode::Char('q'))));
    assert!(panel.exit_requested());

    let area = Rect::new(0, 0, 100, 30);
    let mut buf = Buffer::empty(area);
    (&panel).render(area, &mut buf);
}

#[test]
fn plugin_manager_lists_and_toggles() {
    let mut mgr = plugin_manager::PluginManager::new(vec![
        plugin_manager::PluginRow::internal("internal:home", "0.1.0").with_themes(1),
        plugin_manager::PluginRow::external("acme.demo", "1.2.3").with_commands(3),
    ]);
    assert!(mgr.dispatch_key(key(KeyCode::Down)));
    assert_eq!(mgr.cursor(), 1);
    assert!(mgr.dispatch_key(key(KeyCode::Char(' '))));
    assert!(mgr.toggle_requested());

    let area = Rect::new(0, 0, 100, 30);
    let mut buf = Buffer::empty(area);
    (&mgr).render(area, &mut buf);
}

#[test]
fn sidebar_navigates_and_activates() {
    let mut s = sidebar::Sidebar::new(vec![
        sidebar::SidebarEntry::new("jnoccio", "Jnoccio").with_status(sidebar::SidebarStatus::Live),
        sidebar::SidebarEntry::new("jankurai", "Jankurai")
            .with_status(sidebar::SidebarStatus::Booting),
    ]);
    s.dispatch_key(key(KeyCode::Down));
    assert_eq!(s.cursor(), 1);
    s.dispatch_key(key(KeyCode::Enter));
    assert!(s.activate_requested());

    let area = Rect::new(0, 0, 100, 30);
    let mut buf = Buffer::empty(area);
    (&s).render(area, &mut buf);
}
