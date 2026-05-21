//! Permission banner (COWBOY.md T1-V2, per `tips/fucktui/tip9.txt`).
//!
//! One-row strip between the transcript bottom and the composer top. Renders
//! as:
//!
//! ```text
//! ▸▸ {permission_mode} · {agent_count} local agents · {hint}
//! ```
//!
//! - The `▸▸` prefix is bold magenta. Until Q12 lands a dedicated
//!   `BANNER_MAGENTA` const, we re-use [`codex::PINK_AGENT`] (the closest
//!   approved accent in the codex palette).
//! - The rest of the row is [`codex::FG_DIM`].
//! - When the terminal is too narrow to fit every segment, we drop trailing
//!   segments by priority via [`layout::status_pack::pack`]:
//!   `prefix=0` (anchor, never drops) → `mode=1` → `count=2` → `hint=3`
//!   (drops first).
//!
//! Pure render: callers pass a focus-aware hint string so this module stays
//! ignorant of the actual `Focus` enum (which lives next to `inline_runtime`).

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Paragraph, Widget};
use unicode_width::UnicodeWidthStr;

use crate::glyph_set;
use crate::layout::status_pack::{pack, PackOptions, Segment};
use crate::theme::codex;

/// Per-focus hint strings. The inline runtime picks one based on its own
/// `FocusArea` enum and passes the result through.
pub const HINT_CHAT_FOCUS: &str = "↓ to manage";
pub const HINT_AGENT_PANEL_FOCUS: &str = "↑/↓ to select · Enter to view";
pub const HINT_OTHER_FOCUS: &str = "Esc to return";

/// Render the permission banner into a single row at `area`.
///
/// `mode_label` — current permission mode (e.g. `"bypass permissions"`).
/// `agent_count` — number of local agents currently visible in the rail.
/// `focus_hint` — one of the `HINT_*` constants above, chosen by the caller.
/// `pack_single_line` — T-COMPONENT-PLUMBING: when `true`, fold every
/// optional segment into a single packed line via [`layout::status_pack::pack`]
/// using the high-priority drop chain (hint drops first, then count). Default
/// `false` preserves the historical render path that already routes the
/// post-prefix tail through `pack`, but with the legacy priority ordering.
pub fn render_permission_banner(
    buf: &mut Buffer,
    area: Rect,
    mode_label: &str,
    agent_count: usize,
    focus_hint: &str,
    pack_single_line: bool,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let pluralize = if agent_count == 1 { "agent" } else { "agents" };
    let prefix_style = Style::default()
        .fg(codex::PINK_AGENT)
        .add_modifier(Modifier::BOLD);
    let dim = Style::default().fg(codex::FG_DIM);

    // The spec writes the row as `▸▸ {mode} · {count} · {hint}`: a single
    // space between the magenta arrows and the mode label, then ` · ` for
    // every subsequent join. We can't express two separators inside one
    // `pack` call, so we paint the `▸▸ ` prefix manually first and let the
    // packer own everything to the right of it. The prefix is the anchor
    // (priority 0) — its presence is non-negotiable, so we always reserve
    // its width up front. T-GLYPH-WAVE2: defer to the active GlyphMode so
    // `JEKKO_ASCII=1` swaps the arrows out for `>>`.
    let prefix_glyph = glyph_set::current().banner_prefix;
    let prefix_text = format!("{prefix_glyph} ");
    let prefix_width = UnicodeWidthStr::width(prefix_text.as_str()) as u16;
    if area.width < prefix_width {
        // Not enough room for even the anchor — emit nothing rather than
        // truncate the prefix mid-glyph.
        return;
    }

    // Reserve the leftmost cells for the prefix, hand the rest to the packer.
    let row = Rect::new(area.x, area.y, area.width, 1);
    buf.set_string(area.x, area.y, prefix_text, prefix_style);

    let rest_area = Rect::new(area.x + prefix_width, area.y, area.width - prefix_width, 1);
    // T-COMPONENT-PLUMBING: when packed-single-line is requested, bias the
    // drop priorities sharper so the packer sheds the hint first, the
    // agent-count next, and keeps the mode label as long as room allows.
    // Without this flag we keep the historical priorities (1/2/3).
    let segments = if pack_single_line {
        vec![
            Segment::new(mode_label.to_string(), dim, 2),
            Segment::new(format!("{agent_count} local {pluralize}"), dim, 3),
            Segment::new(focus_hint.to_string(), dim, 4),
        ]
    } else {
        vec![
            Segment::new(mode_label.to_string(), dim, 1),
            Segment::new(format!("{agent_count} local {pluralize}"), dim, 2),
            Segment::new(focus_hint.to_string(), dim, 3),
        ]
    };
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
    let spans = pack(&segments, rest_area.width, &opts);
    if spans.is_empty() {
        // Prefix only — still a valid render.
        let _ = row;
        return;
    }
    Paragraph::new(Line::from(spans)).render(rest_area, buf);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;
    use unicode_width::UnicodeWidthStr;

    fn render(width: u16, mode: &str, count: usize, hint: &str) -> String {
        render_with_pack(width, mode, count, hint, false)
    }

    fn render_with_pack(
        width: u16,
        mode: &str,
        count: usize,
        hint: &str,
        pack_single_line: bool,
    ) -> String {
        let backend = TestBackend::new(width, 1);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                render_permission_banner(
                    frame.buffer_mut(),
                    Rect::new(0, 0, width, 1),
                    mode,
                    count,
                    hint,
                    pack_single_line,
                );
            })
            .unwrap();
        let buf = terminal.backend().buffer();
        let mut out = String::new();
        for x in 0..width {
            out.push_str(buf[(x, 0)].symbol());
        }
        out.trim_end().to_string()
    }

    #[test]
    fn happy_path_renders_prefix_mode_count_hint() {
        let out = render(120, "bypass permissions", 2, HINT_CHAT_FOCUS);
        assert!(out.starts_with("▸▸"), "prefix missing: {out:?}");
        assert!(out.contains("bypass permissions"), "mode missing: {out:?}");
        assert!(out.contains("2 local agents"), "count missing: {out:?}");
        assert!(out.contains(HINT_CHAT_FOCUS), "hint missing: {out:?}");
    }

    #[test]
    fn singular_agent_uses_singular_noun() {
        let out = render(120, "bypass", 1, HINT_CHAT_FOCUS);
        assert!(out.contains("1 local agent"), "singular noun: {out:?}");
        assert!(
            !out.contains("local agents"),
            "should NOT pluralise at 1: {out:?}"
        );
    }

    #[test]
    fn zero_agents_uses_plural_noun() {
        let out = render(120, "bypass", 0, HINT_CHAT_FOCUS);
        assert!(out.contains("0 local agents"), "zero is plural: {out:?}");
    }

    #[test]
    fn narrow_width_drops_hint_first() {
        // 40 cols can't fit prefix + mode + count + hint with " · " seps.
        // Drop priority: hint (3) drops first.
        let out = render(40, "bypass permissions", 2, HINT_CHAT_FOCUS);
        assert!(out.starts_with("▸▸"), "prefix preserved");
        assert!(out.contains("bypass permissions"), "mode preserved");
        assert!(
            !out.contains(HINT_CHAT_FOCUS),
            "hint should drop at 40 cols, got: {out:?}"
        );
    }

    #[test]
    fn very_narrow_width_keeps_only_prefix() {
        // Width that fits only the prefix.
        let out = render(6, "bypass permissions", 99, HINT_CHAT_FOCUS);
        assert!(out.starts_with("▸▸"), "prefix anchor survives: {out:?}");
        assert!(!out.contains("bypass permissions"), "mode dropped: {out:?}");
        assert!(!out.contains(HINT_CHAT_FOCUS), "hint dropped: {out:?}");
    }

    #[test]
    fn zero_area_paints_nothing() {
        let backend = TestBackend::new(80, 1);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                render_permission_banner(
                    frame.buffer_mut(),
                    Rect::new(0, 0, 0, 0),
                    "bypass",
                    1,
                    HINT_CHAT_FOCUS,
                    false,
                );
            })
            .unwrap();
        // No cells touched → buffer is all spaces.
        let buf = terminal.backend().buffer();
        for x in 0..80 {
            assert_eq!(buf[(x, 0)].symbol(), " ", "cell at x={x}");
        }
    }

    #[test]
    fn agent_panel_focus_hint_includes_navigation_keys() {
        let out = render(120, "bypass", 1, HINT_AGENT_PANEL_FOCUS);
        assert!(out.contains("↑/↓"), "missing arrows: {out:?}");
        assert!(out.contains("Enter"), "missing Enter: {out:?}");
    }

    #[test]
    fn rendered_width_never_exceeds_terminal_width() {
        // Sweep across widths used by the snapshots so we never spill into
        // composer chrome below.
        for w in [10u16, 20, 30, 40, 60, 80, 120] {
            let out = render(w, "bypass permissions", 3, HINT_CHAT_FOCUS);
            assert!(
                UnicodeWidthStr::width(out.as_str()) <= w as usize,
                "width {w}: rendered {} cells, text={out:?}",
                UnicodeWidthStr::width(out.as_str())
            );
        }
    }

    #[test]
    fn permission_banner_single_line_packs_under_narrow_width() {
        // T-COMPONENT-PLUMBING: at 40 cols with `pack_single_line == true`,
        // the row must still fit on a single line (no wrap) and shed
        // segments in priority order (hint=4 → count=3 → mode=2). The prefix
        // anchor (`▸▸ `) always survives.
        let out = render_with_pack(40, "bypass permissions", 2, HINT_CHAT_FOCUS, true);
        assert!(out.starts_with("▸▸"), "prefix anchor survives: {out:?}");
        assert!(
            UnicodeWidthStr::width(out.as_str()) <= 40,
            "packed row must fit single line at 40 cols, got width {} text={out:?}",
            UnicodeWidthStr::width(out.as_str())
        );
        // At 40 cols we don't have room for everything; the hint (highest
        // drop priority) must vanish before the lower-priority segments.
        assert!(
            !out.contains(HINT_CHAT_FOCUS),
            "packed mode: hint drops first at 40 cols, got {out:?}"
        );
    }
}
