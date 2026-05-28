use std::time::Duration;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;

use super::*;

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

fn wordmark_colors(buf: &Buffer) -> Vec<Color> {
    let area = buf.area();
    let mut colors = Vec::new();
    for y in 0..area.height {
        for x in 0..area.width {
            if buf[(x, y)].symbol().trim().is_empty() {
                continue;
            }
            if let Some(color) = buf[(x, y)].style().fg {
                colors.push(color);
            }
        }
    }
    colors
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
fn render_splash_stays_static_across_elapsed_time() {
    let area = Rect::new(0, 0, 80, 24);
    let ctx = fixture_ctx();

    let mut buf_a = fresh_buffer(80, 24);
    render_splash(&mut buf_a, area, Duration::ZERO, &ctx, None);

    let mut buf_b = fresh_buffer(80, 24);
    render_splash(&mut buf_b, area, Duration::from_millis(500), &ctx, None);

    for y in 0..area.height {
        for x in 0..area.width {
            assert_eq!(
                buf_a[(x, y)].style().fg,
                buf_b[(x, y)].style().fg,
                "splash logo must not animate (cell {x},{y})"
            );
        }
    }
}

#[test]
fn render_splash_static_test_seam_matches_public_renderer() {
    let area = Rect::new(0, 0, 80, 24);
    let ctx = fixture_ctx();

    let mut buf_a = fresh_buffer(80, 24);
    render_splash_static_for_tests(&mut buf_a, area, &ctx);

    let mut buf_b = fresh_buffer(80, 24);
    render_splash(&mut buf_b, area, Duration::from_millis(1_000), &ctx, None);

    for y in 0..area.height {
        for x in 0..area.width {
            assert_eq!(
                buf_a[(x, y)].style().fg,
                buf_b[(x, y)].style().fg,
                "expected static seam to match public renderer (cell {x},{y})"
            );
        }
    }
}

#[test]
fn render_splash_uses_multi_color_gradient() {
    let area = Rect::new(0, 0, 80, 24);
    let ctx = fixture_ctx();
    let mut buf = fresh_buffer(80, 24);
    render_splash(&mut buf, area, Duration::ZERO, &ctx, None);

    let colors = wordmark_colors(&buf);
    let mut unique = Vec::new();
    for color in colors {
        if !unique.contains(&color) {
            unique.push(color);
        }
    }
    assert!(
        unique.len() >= 5,
        "expected a rich static gradient, saw {unique:?}"
    );
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
