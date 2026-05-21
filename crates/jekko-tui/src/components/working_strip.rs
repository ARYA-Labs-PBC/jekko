//! Working/status strip (COWBOY.md T1-V3, per `tips/fucktui/tip6.txt`).
//!
//! One-row strip painted ABOVE the permission banner — but only when a turn
//! is in flight OR a background terminal is running. When idle and no
//! background work, the strip is hidden entirely (zero rows).
//!
//! ```text
//! ◦ Working (1m 5s • esc to interrupt) · 2 background terminal running · /ps to view · /stop to close
//! ```
//!
//! - The leading `◦` glyph pulses via [`anim::pulse_glyph`] and is painted
//!   in [`codex::CYAN_TAB`]. Reduced-motion users get the static "brightest"
//!   frame (handled inside `pulse_glyph`).
//! - The rest of the row is [`codex::FG_DIM`].
//! - At narrow widths, segments drop by priority via
//!   [`layout::status_pack::pack`]: pulse_label=0 (anchor) → time=1 →
//!   background=2 → ps_hint=3 → stop_hint=4 (drops first).
//!
//! Returns `true` if the strip drew anything (so the caller can decide
//! whether to reserve a row in its `Layout::vertical` constraint set).

use std::time::Duration;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};
use unicode_width::UnicodeWidthStr;

use crate::anim::{self, elapsed_label, pulse_glyph_with_motion};
use crate::glyph_set;
use crate::layout::status_pack::{pack, PackOptions, Segment};
use crate::theme::codex;

/// Render the working strip into `area`. Returns `true` if any cells were
/// painted (work was in-flight or a background terminal was running), or
/// `false` if the strip was hidden because there was nothing to report.
///
/// `elapsed = None` → no in-flight turn.
/// `background_count > 0` → still draw even when idle so the user sees their
/// detached terminals.
/// `pack_single_line` — T-COMPONENT-PLUMBING: when `true`, prefer the short
/// anchor form (just `◦ Working`/`◦ Background`) so the row stays packed on
/// a single line. The trailing segments still flow through
/// [`layout::status_pack::pack`]; `false` preserves the historical render
/// path that prefers the full anchor including the parenthesised time/hint.
///
/// `cfg` — T-GLYPH-WAVE3: when `Some`, the pulsing `◦` anchor honors the
/// resolved [`jekko_core::config::ui::UiConfig.accessibility.reduced_motion`]
/// flag via [`anim::motion_enabled_with_cfg`]. When `None`, behaviour falls
/// back to the legacy env/file-cached [`anim::motion_enabled`] path so
/// unwired call sites keep their existing animation.
pub fn render_working_strip(
    buf: &mut Buffer,
    area: Rect,
    elapsed: Option<Duration>,
    background_count: usize,
    pack_single_line: bool,
    cfg: Option<&jekko_core::config::ui::UiConfig>,
) -> bool {
    let motion_enabled = anim::motion_enabled_with_cfg(cfg);
    if area.width == 0 || area.height == 0 {
        return false;
    }
    let in_flight = elapsed.is_some();
    if !in_flight && background_count == 0 {
        return false;
    }

    let dim = Style::default().fg(codex::FG_DIM);
    let pulse_style = Style::default().fg(codex::CYAN_TAB);

    // Per tip6 spec: `◦ Working (1m 5s • esc to interrupt) · …`. The anchor
    // (pulsing glyph + "Working" + parenthesised time/hint) joins with
    // SPACES, while the later segments (background count, /ps, /stop) join
    // with ` · `. status_pack only supports one separator, so we render the
    // anchor manually first and hand the remainder to the packer.
    let glyph = match elapsed {
        Some(active_elapsed) => pulse_glyph_with_motion(active_elapsed, motion_enabled),
        None => {
            // No pulse needed when only background terminals are alive — render
            // the bright static frame for visibility.
            pulse_glyph_with_motion(Duration::ZERO, motion_enabled)
        }
    };
    // Two anchor flavours: "full" includes the parenthesised time/hint;
    // "short" is just glyph + label. Use full when there's room, fall back
    // to short when the terminal is too narrow.
    let full_anchor = match (in_flight, elapsed) {
        (true, Some(t)) => format!("{glyph} Working ({} • esc to interrupt)", elapsed_label(t)),
        (true, None) => format!("{glyph} Working"),
        (false, _) => format!("{glyph} Background"),
    };
    let short_anchor = if in_flight {
        format!("{glyph} Working")
    } else {
        format!("{glyph} Background")
    };
    let full_width = UnicodeWidthStr::width(full_anchor.as_str()) as u16;
    let short_width = UnicodeWidthStr::width(short_anchor.as_str()) as u16;
    // T-COMPONENT-PLUMBING: in packed single-line mode we always pick the
    // short anchor when it fits (saving cells for the trailing segments and
    // matching the "packed" intent). The non-packed path keeps the historical
    // preference for the full anchor.
    let (anchor_text, anchor_width) = if pack_single_line && area.width >= short_width {
        (short_anchor, short_width)
    } else if area.width >= full_width {
        (full_anchor, full_width)
    } else if area.width >= short_width {
        (short_anchor, short_width)
    } else {
        // Even the short form doesn't fit — skip the strip entirely so the
        // chrome below shifts up cleanly.
        return false;
    };

    let row = Rect::new(area.x, area.y, area.width, 1);
    buf.set_string(area.x, area.y, anchor_text.clone(), pulse_style);

    let rest_x = area.x + anchor_width;
    let rest_width = area.width - anchor_width;
    if rest_width == 0 {
        return true;
    }

    let mut segments: Vec<Segment> = Vec::new();
    if background_count > 0 {
        let noun = if background_count == 1 {
            "terminal"
        } else {
            "terminals"
        };
        // T-COMPONENT-PLUMBING: in packed single-line mode the trailing
        // segments must drop sharper so the anchor + count survive longest.
        // Without the flag we use the historical 0/1/2 ordering.
        let (p_bg, p_ps, p_stop) = if pack_single_line {
            (1u8, 3u8, 4u8)
        } else {
            (0u8, 1u8, 2u8)
        };
        segments.push(Segment::new(
            format!("{background_count} background {noun} running"),
            dim,
            p_bg,
        ));
        segments.push(Segment::new("/ps to view".to_string(), dim, p_ps));
        segments.push(Segment::new("/stop to close".to_string(), dim, p_stop));
    }
    if segments.is_empty() {
        return true;
    }

    // T-GLYPH-WAVE3: separator dot defers to GlyphMode (`-` in ASCII), and
    // ellipsis already honors GlyphMode for narrow-width truncation.
    let g = glyph_set::current();
    let separator: &'static str = if g.separator_dot == "·" {
        " · "
    } else {
        " - "
    };
    let opts = PackOptions {
        separator,
        separator_style: Style::default().fg(codex::FG_VERY_DIM),
        ellipsis: g.ellipsis,
    };
    // Reserve `separator` worth of cells for the join between the anchor and
    // the packed remainder; the packer plans its own internal joins inside
    // the remaining budget.
    let sep_width = UnicodeWidthStr::width(separator) as u16;
    if rest_width <= sep_width {
        // Not enough room for both the join and any content — leave the
        // anchor alone.
        return true;
    }
    let pack_budget = rest_width - sep_width;
    let spans = pack(&segments, pack_budget, &opts);
    if spans.is_empty() {
        let _ = row;
        return true;
    }
    let mut full = vec![Span::styled(separator.to_string(), opts.separator_style)];
    full.extend(spans);
    let rest_area = Rect::new(rest_x, area.y, rest_width, 1);
    Paragraph::new(Line::from(full)).render(rest_area, buf);
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;
    use unicode_width::UnicodeWidthStr;

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
}
