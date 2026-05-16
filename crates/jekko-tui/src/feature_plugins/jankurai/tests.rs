//! Integration tests for the Jankurai panel.

#![cfg(test)]

use super::sparkline::{BLANK_GLYPH, GLYPHS};
use super::*;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::Widget;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

#[test]
fn delta_zero_is_neutral() {
    let d = compute_delta(Some(50.0), Some(50.0), DeltaMetric::Score);
    assert_eq!(d.direction, DeltaDirection::Neutral);
    assert_eq!(d.glyph, "=");
}

#[test]
fn delta_score_higher_is_better() {
    let d = compute_delta(Some(75.0), Some(60.0), DeltaMetric::Score);
    assert_eq!(d.direction, DeltaDirection::Improving);
    assert_eq!(d.glyph, "▲▲");

    let d = compute_delta(Some(45.0), Some(60.0), DeltaMetric::Score);
    assert_eq!(d.direction, DeltaDirection::Worsening);
    assert_eq!(d.glyph, "▼▼");
}

#[test]
fn delta_findings_lower_is_better() {
    let d = compute_delta(Some(2.0), Some(7.0), DeltaMetric::Hard);
    assert_eq!(d.direction, DeltaDirection::Improving);
    let d = compute_delta(Some(9.0), Some(4.0), DeltaMetric::Soft);
    assert_eq!(d.direction, DeltaDirection::Worsening);
}

#[test]
fn delta_unknown_when_missing_side() {
    let d = compute_delta(None, Some(10.0), DeltaMetric::Score);
    assert_eq!(d.direction, DeltaDirection::Unknown);
    assert_eq!(d.glyph, "-");
    assert!(d.delta.is_none());
}

#[test]
fn format_delta_handles_zero_and_signs() {
    let zero = compute_delta(Some(1.0), Some(1.0), DeltaMetric::Score);
    assert_eq!(format_delta(&zero), "= 0");
    let pos = compute_delta(Some(5.0), Some(3.0), DeltaMetric::Score);
    assert!(format_delta(&pos).starts_with("+2"));
    let neg = compute_delta(Some(2.0), Some(5.0), DeltaMetric::Hard);
    assert!(format_delta(&neg).starts_with("-3"));
}

#[test]
fn sparkline_empty_returns_blanks() {
    let s = sparkline(&[], 10);
    assert_eq!(s.chars().count(), 10);
    assert!(s.chars().all(|c| c == BLANK_GLYPH));
}

#[test]
fn sparkline_width_zero_returns_empty() {
    assert_eq!(sparkline(&[1.0, 2.0, 3.0], 0), "");
}

#[test]
fn sparkline_constant_values_picks_mid_glyph() {
    let s = sparkline(&[5.0, 5.0, 5.0], 3);
    let mid = GLYPHS[GLYPHS.len() / 2];
    assert_eq!(s, format!("{}{}{}", mid, mid, mid));
}

#[test]
fn sparkline_widens_with_blanks() {
    let s = sparkline(&[1.0, 2.0], 5);
    assert_eq!(s.chars().count(), 5);
    assert!(s.starts_with(BLANK_GLYPH));
}

#[test]
fn sparkline_tails_when_oversize() {
    let s = sparkline(&[1.0, 2.0, 3.0, 4.0, 5.0], 3);
    assert_eq!(s.chars().count(), 3);
}

#[test]
fn panel_consumes_q_and_esc_only() {
    let reset_fn: fn(&mut JankuraiPanel) = JankuraiPanel::clear_exit;
    let mut panel = JankuraiPanel::new(JankuraiSnapshot::default());
    assert!(!panel.exit_requested());
    assert!(!panel.dispatch_key(key(KeyCode::Char('j'))));
    assert!(panel.dispatch_key(key(KeyCode::Char('q'))));
    assert!(panel.exit_requested());
    reset_fn(&mut panel);
    assert!(!panel.exit_requested());
    assert!(panel.dispatch_key(key(KeyCode::Esc)));
}

#[test]
fn renders_empty_panel_with_setup_hint() {
    let area = Rect::new(0, 0, 100, 30);
    let mut buf = Buffer::empty(area);
    let panel = JankuraiPanel::new(JankuraiSnapshot::default());
    (&panel).render(area, &mut buf);
}

#[test]
fn renders_populated_panel_at_200x60() {
    let area = Rect::new(0, 0, 200, 60);
    let mut buf = Buffer::empty(area);
    let panel = JankuraiPanel::new(JankuraiSnapshot {
        jankurai_installed: true,
        score: Some(82.4),
        decision: Some("pass".to_string()),
        conformance_level: Some("A".to_string()),
        caps_applied: Some(3.0),
        hard_findings: Some(1.0),
        soft_findings: Some(4.0),
        auditor_version: Some("3.1.0".to_string()),
        history: vec![70.0, 72.0, 74.0, 78.0, 79.0, 82.4],
        baseline_score: Some(78.0),
        baseline_caps: Some(2.0),
        baseline_hard: Some(2.0),
        baseline_soft: Some(5.0),
        workers: vec![
            JankuraiWorker {
                id: "lane-a".to_string(),
                kind: "tail".to_string(),
            },
            JankuraiWorker {
                id: "lane-b".to_string(),
                kind: "score".to_string(),
            },
        ],
        last_run_age: Some("23s".to_string()),
    });
    (&panel).render(area, &mut buf);
}
