use super::*;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::Widget;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn shift(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::SHIFT)
}

#[test]
fn cursor_moves_and_wraps() {
    let rows = vec![
        PluginRow::internal("a", "0.1.0"),
        PluginRow::internal("b", "0.2.0"),
        PluginRow::internal("c", "0.3.0"),
    ];
    let mut mgr = PluginManager::new(rows);
    assert_eq!(mgr.cursor(), 0);
    mgr.dispatch_key(key(KeyCode::Down));
    mgr.dispatch_key(key(KeyCode::Down));
    assert_eq!(mgr.cursor(), 2);
    // Wrap down.
    mgr.dispatch_key(key(KeyCode::Down));
    assert_eq!(mgr.cursor(), 0);
    // Wrap up.
    mgr.dispatch_key(key(KeyCode::Up));
    assert_eq!(mgr.cursor(), 2);
}

#[test]
fn empty_rows_no_cursor_panic() {
    let mut mgr = PluginManager::new(vec![]);
    mgr.dispatch_key(key(KeyCode::Down));
    mgr.dispatch_key(key(KeyCode::Down));
    assert_eq!(mgr.cursor(), 0);
    assert!(mgr.selected().is_none());
    // Toggle on empty list does NOT set the flag.
    mgr.dispatch_key(key(KeyCode::Enter));
    assert!(!mgr.toggle_requested());
}

#[test]
fn sort_puts_internal_before_external() {
    let mut mgr = PluginManager::new(vec![
        PluginRow::external("zeta", "1.0.0"),
        PluginRow::internal("alpha", "0.1.0"),
        PluginRow::external("alpha-ext", "0.2.0"),
        PluginRow::internal("beta", "0.1.1"),
    ]);
    mgr.sort();
    let kinds: Vec<PluginRowKind> = mgr.rows().iter().map(|r| r.kind).collect();
    assert_eq!(
        kinds,
        vec![
            PluginRowKind::Internal,
            PluginRowKind::Internal,
            PluginRowKind::External,
            PluginRowKind::External
        ]
    );
    assert_eq!(mgr.rows()[0].id, "alpha");
    assert_eq!(mgr.rows()[1].id, "beta");
}

#[test]
fn shift_i_requests_install() {
    let mut mgr = PluginManager::new(vec![PluginRow::internal("x", "0.1.0")]);
    assert!(!mgr.install_requested());
    mgr.dispatch_key(shift(KeyCode::Char('I')));
    assert!(mgr.install_requested());
    mgr.clear_install();
    assert!(!mgr.install_requested());
}

#[test]
fn space_or_enter_requests_toggle() {
    let mut mgr = PluginManager::new(vec![
        PluginRow::internal("x", "0.1.0"),
        PluginRow::internal("y", "0.2.0"),
    ]);
    mgr.dispatch_key(key(KeyCode::Char(' ')));
    assert!(mgr.toggle_requested());
    mgr.clear_toggle();
    assert!(!mgr.toggle_requested());
    mgr.dispatch_key(key(KeyCode::Enter));
    assert!(mgr.toggle_requested());
}

#[test]
fn quit_keys_set_exit_flag() {
    let reset_fn: fn(&mut PluginManager) = PluginManager::clear_exit;
    let mut mgr = PluginManager::new(vec![PluginRow::internal("a", "0.1.0")]);
    mgr.dispatch_key(key(KeyCode::Char('q')));
    assert!(mgr.exit_requested());
    reset_fn(&mut mgr);
    mgr.dispatch_key(key(KeyCode::Esc));
    assert!(mgr.exit_requested());
}

#[test]
fn g_jumps_to_first_and_shift_g_to_last() {
    let mut mgr = PluginManager::new(vec![
        PluginRow::internal("a", "0.1.0"),
        PluginRow::internal("b", "0.1.0"),
        PluginRow::internal("c", "0.1.0"),
    ]);
    mgr.dispatch_key(shift(KeyCode::Char('G')));
    assert_eq!(mgr.cursor(), 2);
    mgr.dispatch_key(key(KeyCode::Char('g')));
    assert_eq!(mgr.cursor(), 0);
}

#[test]
fn set_rows_clamps_cursor() {
    let mut mgr = PluginManager::new(vec![
        PluginRow::internal("a", "0.1.0"),
        PluginRow::internal("b", "0.1.0"),
        PluginRow::internal("c", "0.1.0"),
    ]);
    mgr.dispatch_key(shift(KeyCode::Char('G')));
    assert_eq!(mgr.cursor(), 2);
    mgr.set_rows(vec![PluginRow::internal("only", "0.1.0")]);
    assert_eq!(mgr.cursor(), 0);
    mgr.set_rows(vec![]);
    assert_eq!(mgr.cursor(), 0);
}

#[test]
fn renders_at_100x30_and_200x60() {
    let mgr = PluginManager::new(vec![
        PluginRow::internal("internal:home", "0.1.0")
            .with_themes(1)
            .with_commands(3)
            .with_model_presets(0),
        PluginRow::external("acme.demo", "1.2.3")
            .with_themes(2)
            .with_commands(4)
            .with_model_presets(1)
            .with_description("Demo plugin"),
    ]);

    let small = Rect::new(0, 0, 100, 30);
    let mut buf = Buffer::empty(small);
    (&mgr).render(small, &mut buf);

    let large = Rect::new(0, 0, 200, 60);
    let mut buf = Buffer::empty(large);
    (&mgr).render(large, &mut buf);
}
