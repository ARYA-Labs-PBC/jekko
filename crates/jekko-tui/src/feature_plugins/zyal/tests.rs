use super::palette::fmt_n;
use super::*;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::Widget;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

#[test]
fn panel_consumes_exit_keys_only() {
    let reset_fn: fn(&mut ZyalPanel) = ZyalPanel::clear_exit;
    let mut panel = ZyalPanel::new(ZyalSnapshot::default());
    assert!(!panel.dispatch_key(key(KeyCode::Char('j'))));
    assert!(panel.dispatch_key(key(KeyCode::Esc)));
    assert!(panel.exit_requested());
    reset_fn(&mut panel);
    assert!(panel.dispatch_key(key(KeyCode::Char('q'))));
    assert!(panel.exit_requested());
}

#[test]
fn fmt_n_matches_expectations() {
    assert_eq!(fmt_n(0), "0");
    assert_eq!(fmt_n(1_234), "1.2K");
    assert_eq!(fmt_n(1_500_000), "1.5M");
    assert_eq!(fmt_n(2_300_000_000), "2.3B");
}

#[test]
fn exit_tone_labels_are_distinct() {
    assert_ne!(ZyalExitTone::Success.label(), ZyalExitTone::Warning.label());
    assert_ne!(ZyalExitTone::Warning.label(), ZyalExitTone::Error.label());
    assert_ne!(ZyalExitTone::Success.color(), ZyalExitTone::Error.color());
}

#[test]
fn renders_empty_panel_at_100x30() {
    let area = Rect::new(0, 0, 100, 30);
    let mut buf = Buffer::empty(area);
    let panel = ZyalPanel::new(ZyalSnapshot::default());
    (&panel).render(area, &mut buf);
}

#[test]
fn renders_full_panel_at_200x60() {
    let area = Rect::new(0, 0, 200, 60);
    let mut buf = Buffer::empty(area);
    let panel = ZyalPanel::new(ZyalSnapshot {
        run_id: Some("zyal_abc123".to_string()),
        status: Some("active".to_string()),
        loops_completed: 42,
        tasks_completed: 7,
        tasks_incubated: 3,
        total_tokens: 1_234_567,
        input_tokens: 800_000,
        output_tokens: 400_000,
        cache_tokens: 34_567,
        workers_active: 4,
        workers_max: 16,
        cost_usd: 12.34,
        uptime: Some("00:34:12".to_string()),
        jankurai_findings: Some(2),
        paste_signature: Some("sha:abc".to_string()),
        paste_bytes: 8_192,
        runbook_preview: vec![
            ZyalRunbookLine {
                step: 1,
                text: "scan for HLT-001 violations".to_string(),
            },
            ZyalRunbookLine {
                step: 2,
                text: "open pull request".to_string(),
            },
        ],
        exit: Some(ZyalExitRecord {
            tone: ZyalExitTone::Success,
            status: "satisfied".to_string(),
            reason: "runbook completed clean".to_string(),
        }),
    });
    (&panel).render(area, &mut buf);
}
