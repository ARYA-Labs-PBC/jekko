use std::time::Duration;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use super::*;
use crate::theme::codex;

fn fixture_ctx() -> SplashContext {
    SplashContext {
        version: "1.2.3".into(),
        cwd: "~/code/jekko".into(),
        branch: Some("main".into()),
    }
}

fn fresh_buffer(width: u16, height: u16) -> Buffer {
    Buffer::empty(Rect::new(0, 0, width, height))
}

fn buffer_to_symbol_string(buf: &Buffer) -> String {
    let area = buf.area();
    let mut out = String::new();
    for y in 0..area.height {
        for x in 0..area.width {
            out.push_str(buf[(x, y)].symbol());
        }
        out.push('\n');
    }
    out
}

#[test]
fn render_splash_fits_within_area_bounds() {
    let area = Rect::new(0, 0, 80, 24);
    let mut buf = fresh_buffer(80, 24);
    let ctx = fixture_ctx();
    render_splash(&mut buf, area, Duration::ZERO, &ctx, None);
    let any_glyph =
        (0..area.height).any(|y| (0..area.width).any(|x| !buf[(x, y)].symbol().trim().is_empty()));
    assert!(any_glyph, "expected splash to emit at least one glyph");
}

#[test]
fn render_splash_emits_wordmark() {
    let area = Rect::new(0, 0, 80, 24);
    let mut buf = fresh_buffer(80, 24);
    let ctx = fixture_ctx();
    render_splash(&mut buf, area, Duration::ZERO, &ctx, None);
    let dump = buffer_to_symbol_string(&buf);
    assert!(
        dump.contains('█'),
        "expected wordmark block glyph, got:\n{dump}"
    );
}

#[test]
fn render_splash_emits_subtitle() {
    let area = Rect::new(0, 0, 80, 24);
    let mut buf = fresh_buffer(80, 24);
    let ctx = fixture_ctx();
    render_splash(&mut buf, area, Duration::ZERO, &ctx, None);
    let dump = buffer_to_symbol_string(&buf);
    assert!(dump.contains("v1.2.3"), "expected version, got:\n{dump}");
    assert!(dump.contains("~/code/jekko"), "expected cwd, got:\n{dump}");
    assert!(dump.contains("main"), "expected branch, got:\n{dump}");
}

#[test]
fn render_splash_animates_when_motion_enabled() {
    let area = Rect::new(0, 0, 80, 24);
    let ctx = fixture_ctx();

    let mut buf_a = fresh_buffer(80, 24);
    render_splash_with_motion(&mut buf_a, area, Duration::ZERO, &ctx, true);

    let mut buf_b = fresh_buffer(80, 24);
    render_splash_with_motion(&mut buf_b, area, Duration::from_millis(500), &ctx, true);

    let mut diff_found = false;
    for y in 0..area.height {
        for x in 0..area.width {
            if buf_a[(x, y)].style().fg != buf_b[(x, y)].style().fg {
                diff_found = true;
                break;
            }
        }
        if diff_found {
            break;
        }
    }
    assert!(
        diff_found,
        "expected at least one cell color to change between frames"
    );
}

#[test]
fn render_splash_static_when_motion_disabled() {
    let area = Rect::new(0, 0, 80, 24);
    let ctx = fixture_ctx();

    let mut buf_a = fresh_buffer(80, 24);
    render_splash_with_motion(&mut buf_a, area, Duration::ZERO, &ctx, false);

    let mut buf_b = fresh_buffer(80, 24);
    render_splash_with_motion(&mut buf_b, area, Duration::from_millis(1_000), &ctx, false);

    for y in 0..area.height {
        for x in 0..area.width {
            assert_eq!(
                buf_a[(x, y)].style().fg,
                buf_b[(x, y)].style().fg,
                "expected flat color when reduced-motion is active (cell {x},{y})"
            );
        }
    }
}

#[test]
fn render_splash_static_uses_blue_path_color() {
    let area = Rect::new(0, 0, 80, 24);
    let ctx = fixture_ctx();
    let mut buf = fresh_buffer(80, 24);
    render_splash_with_motion(&mut buf, area, Duration::ZERO, &ctx, false);

    let mut saw_blue = false;
    for y in 0..area.height {
        for x in 0..area.width {
            if buf[(x, y)].symbol() == "█" {
                assert_eq!(
                    buf[(x, y)].style().fg,
                    Some(codex::BLUE_PATH),
                    "wordmark cell at ({x},{y}) must be BLUE_PATH in reduced motion"
                );
                saw_blue = true;
            }
        }
    }
    assert!(saw_blue, "expected at least one block glyph in dump");
}

#[test]
fn render_splash_no_op_when_area_too_short() {
    let area = Rect::new(0, 0, 80, 3);
    let mut buf = fresh_buffer(80, 3);
    let ctx = fixture_ctx();
    render_splash(&mut buf, area, Duration::ZERO, &ctx, None);
    let dump = buffer_to_symbol_string(&buf);
    assert!(
        !dump.contains('█') && !dump.contains("v1.2.3"),
        "expected empty render when area is too short, got:\n{dump}"
    );
}

#[test]
fn splash_context_detect_returns_version() {
    let ctx = SplashContext::detect();
    assert!(!ctx.version.is_empty(), "version must be non-empty");
}

#[test]
fn splash_context_omits_branch_when_unknown() {
    let area = Rect::new(0, 0, 80, 24);
    let mut buf = fresh_buffer(80, 24);
    let ctx = SplashContext {
        version: "9.9.9".into(),
        cwd: "/tmp".into(),
        branch: None,
    };
    render_splash(&mut buf, area, Duration::ZERO, &ctx, None);
    let dump = buffer_to_symbol_string(&buf);
    assert!(dump.contains("v9.9.9"));
    assert!(
        dump.contains("/tmp"),
        "cwd should appear in splash:\n{dump}"
    );
    assert!(
        dump.contains("(no git)"),
        "branch fallback should appear when absent:\n{dump}"
    );
}
