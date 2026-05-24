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
///
/// [`layout::status_pack::pack`]: crate::layout::status_pack::pack
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
    let glyph = match elapsed {
        Some(active_elapsed) => pulse_glyph_with_motion(active_elapsed, motion_enabled),
        None => pulse_glyph_with_motion(Duration::ZERO, motion_enabled),
    };
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
    let (anchor_text, anchor_width) = if pack_single_line && area.width >= short_width {
        (short_anchor, short_width)
    } else if area.width >= full_width {
        (full_anchor, full_width)
    } else if area.width >= short_width {
        (short_anchor, short_width)
    } else {
        return false;
    };

    let row = Rect::new(area.x, area.y, area.width, 1);
    buf.set_string(area.x, area.y, anchor_text.clone(), pulse_style);

    let rest_x = area.x + anchor_width;
    let rest_width = area.width - anchor_width;
    if rest_width == 0 {
        return true;
    }

    let segments = background_segments(background_count, dim, pack_single_line);
    if segments.is_empty() {
        return true;
    }

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
    let sep_width = UnicodeWidthStr::width(separator) as u16;
    if rest_width <= sep_width {
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

fn background_segments(
    background_count: usize,
    dim: Style,
    pack_single_line: bool,
) -> Vec<Segment> {
    if background_count == 0 {
        return Vec::new();
    }
    let noun = if background_count == 1 {
        "terminal"
    } else {
        "terminals"
    };
    let (p_bg, p_ps, p_stop) = if pack_single_line {
        (1u8, 3u8, 4u8)
    } else {
        (0u8, 1u8, 2u8)
    };
    vec![
        Segment::new(
            format!("{background_count} background {noun} running"),
            dim,
            p_bg,
        ),
        Segment::new("/ps to view".to_string(), dim, p_ps),
        Segment::new("/stop to close".to_string(), dim, p_stop),
    ]
}
