use std::time::Duration;

use ratatui::backend::TestBackend;
use ratatui::layout::Rect;
use ratatui::Terminal;
use unicode_width::UnicodeWidthStr;

use super::*;

fn draw(width: u16, elapsed: Option<Duration>, bg: usize) -> (bool, String) {
    draw_with_pack(width, elapsed, bg, false, None)
}

fn draw_with_pack(
    width: u16,
    elapsed: Option<Duration>,
    bg: usize,
    pack_single_line: bool,
    cfg: Option<&jekko_core::config::ui::UiConfig>,
) -> (bool, String) {
    let backend = TestBackend::new(width, 1);
    let mut terminal = Terminal::new(backend).unwrap();
    let drawn = std::cell::Cell::new(false);
    terminal
        .draw(|frame| {
            let d = render_working_strip(
                frame.buffer_mut(),
                Rect::new(0, 0, width, 1),
                elapsed,
                bg,
                pack_single_line,
                cfg,
            );
            drawn.set(d);
        })
        .unwrap();
    let buf = terminal.backend().buffer();
    let mut out = String::new();
    for x in 0..width {
        out.push_str(buf[(x, 0)].symbol());
    }
    (drawn.get(), out.trim_end().to_string())
}

#[test]
fn hides_when_idle_and_no_background() {
    let (drawn, out) = draw(80, None, 0);
    assert!(!drawn, "should not draw when idle + no background");
    assert!(out.is_empty(), "buffer untouched: {out:?}");
}

#[test]
fn in_flight_shows_working_and_elapsed() {
    let (drawn, out) = draw(120, Some(Duration::from_secs(65)), 0);
    assert!(drawn, "should draw when in-flight");
    assert!(out.contains("Working"), "missing label: {out:?}");
    assert!(out.contains("1m 5s"), "missing elapsed: {out:?}");
    assert!(out.contains("esc to interrupt"), "missing hint: {out:?}");
}

#[test]
fn background_only_shows_background_label() {
    let (drawn, out) = draw(120, None, 2);
    assert!(drawn, "should draw when background > 0");
    assert!(
        out.contains("Background"),
        "should label as Background when idle: {out:?}"
    );
    assert!(
        out.contains("2 background terminals running"),
        "background segment: {out:?}"
    );
}

#[test]
fn singular_terminal_uses_singular_noun() {
    let (_drawn, out) = draw(120, None, 1);
    assert!(
        out.contains("1 background terminal running"),
        "singular noun: {out:?}"
    );
}

#[test]
fn in_flight_with_background_shows_both() {
    let (drawn, out) = draw(140, Some(Duration::from_secs(5)), 3);
    assert!(drawn);
    assert!(out.contains("Working"));
    assert!(out.contains("5s"));
    assert!(out.contains("3 background terminals running"));
}

#[test]
fn narrow_width_drops_stop_then_ps_first() {
    // Wide enough for anchor + time + background; should drop /stop + /ps.
    let (drawn, out) = draw(60, Some(Duration::from_secs(5)), 1);
    assert!(drawn);
    assert!(out.contains("Working"), "anchor preserved: {out:?}");
    assert!(
        !out.contains("/stop to close"),
        "stop dropped first: {out:?}"
    );
}

#[test]
fn very_narrow_width_keeps_only_anchor() {
    // Width = anchor "◦ Working" (~9 cells) but not enough for separator
    // + time segment.
    let (drawn, out) = draw(10, Some(Duration::from_secs(5)), 0);
    assert!(drawn);
    assert!(out.contains("Working"), "anchor: {out:?}");
    assert!(!out.contains("esc to interrupt"), "time dropped: {out:?}");
}

#[test]
fn zero_area_returns_false() {
    let backend = TestBackend::new(80, 1);
    let mut terminal = Terminal::new(backend).unwrap();
    let drawn = std::cell::Cell::new(false);
    terminal
        .draw(|frame| {
            let d = render_working_strip(
                frame.buffer_mut(),
                Rect::new(0, 0, 0, 0),
                Some(Duration::from_secs(1)),
                0,
                false,
                None,
            );
            drawn.set(d);
        })
        .unwrap();
    assert!(!drawn.get(), "zero-area should not draw");
}

#[test]
fn reduced_motion_keeps_pulse_static() {
    let cfg = jekko_core::config::ui::UiConfig {
        accessibility: jekko_core::config::ui::AccessibilitySection {
            reduced_motion: Some(true),
            ..Default::default()
        },
        ..Default::default()
    };
    let (_drawn, out) = draw_with_pack(80, Some(Duration::from_secs(5)), 0, false, Some(&cfg));
    assert!(out.contains("Working"));
    assert!(out.contains("●"), "static pulse glyph expected: {out:?}");
}

#[test]
fn rendered_width_never_exceeds_budget() {
    for w in [10u16, 20, 40, 60, 80, 120] {
        let (_drawn, out) = draw(w, Some(Duration::from_secs(125)), 2);
        assert!(
            UnicodeWidthStr::width(out.as_str()) <= w as usize,
            "width {w}: rendered {} cells, text={out:?}",
            UnicodeWidthStr::width(out.as_str())
        );
    }
}

#[test]
fn working_strip_single_line_packs_under_narrow_width() {
    // T-COMPONENT-PLUMBING: at 40 cols with `pack_single_line == true`
    // the row must fit a single line (no wrap) and prefer the short
    // anchor (`◦ Working`) so trailing segments can shed in priority
    // order: /stop (4) → /ps (3) → background-count (1).
    let (drawn, out) = draw_with_pack(40, Some(Duration::from_secs(125)), 2, true, None);
    assert!(drawn, "packed mode should still draw");
    assert!(out.contains("Working"), "anchor preserved: {out:?}");
    assert!(
        !out.contains("esc to interrupt"),
        "packed mode short-anchor drops the time/hint group: {out:?}"
    );
    assert!(
        UnicodeWidthStr::width(out.as_str()) <= 40,
        "packed row must fit single line at 40 cols, got width {} text={out:?}",
        UnicodeWidthStr::width(out.as_str())
    );
    // /stop has the highest drop priority and must vanish first.
    assert!(
        !out.contains("/stop to close"),
        "packed mode: /stop drops first at 40 cols, got {out:?}"
    );
}
