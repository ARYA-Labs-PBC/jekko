//! Priority-based single-line status packer (COWBOY.md T1-V9).
//!
//! Status rows in the FUCKTUI design MUST NOT wrap. When the available
//! width can't fit every segment, we drop segments by `priority` (highest
//! number drops first) instead of soft-wrapping or chopping arbitrary
//! suffixes off the right edge. This mirrors the way the reference
//! permission banner / working strip / footer behave in tip4–tip6.
//!
//! Algorithm:
//! 1. Build a drop-order list sorted by `(priority desc, original index
//!    desc)`. Higher-priority segments evict first; equal priorities
//!    evict the *later* segment so leading anchors (like the `▸▸`
//!    glyph) survive longer than trailing decorations.
//! 2. Start with every segment kept. Compute total rendered width =
//!    Σ(segment width) + (kept-1) · separator width.
//! 3. While total > available, mark the next drop-order index as dropped
//!    and recompute.
//! 4. If only one segment survives and is still too wide, ellipsise it
//!    so the row fits exactly.
//! 5. Render: interleave kept segments with separator spans, returning
//!    `Vec<Span<'static>>` so callers can wrap into a `Line` directly.
//!
//! Widths use `unicode-width` (terminal display columns), not byte
//! length, so `▸▸` correctly counts as 2 cells per arrow glyph.

use ratatui::style::Style;
use ratatui::text::Span;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::glyph_set;

/// A single status-row segment with drop priority.
#[derive(Clone, Debug)]
pub struct Segment {
    /// Visible text for the segment. Owned so the result `Vec<Span<'static>>`
    /// can outlive the caller's borrow.
    pub text: String,
    /// Style applied to the segment span (foreground/background/modifiers).
    pub style: Style,
    /// Drop priority: `0` survives forever, `255` drops first.
    pub priority: u8,
}

impl Segment {
    pub fn new(text: impl Into<String>, style: Style, priority: u8) -> Self {
        Self {
            text: text.into(),
            style,
            priority,
        }
    }
}

/// Tunables for the packer.
#[derive(Clone, Copy, Debug)]
pub struct PackOptions {
    /// Separator string inserted between kept segments (`" · "` etc.).
    pub separator: &'static str,
    /// Style applied to separator spans.
    pub separator_style: Style,
    /// Ellipsis appended when a sole survivor is truncated (`"…"` etc.).
    pub ellipsis: &'static str,
}

impl Default for PackOptions {
    fn default() -> Self {
        // T-A11Y-MIGRATION: ellipsis honors the active `GlyphMode` (`"…"`
        // Unicode vs `"..."` ASCII). Separator stays Unicode because the
        // mid-dot is part of jekko's visual grammar and has no clean ASCII
        // equivalent at the same width.
        Self {
            separator: " · ",
            separator_style: Style::default(),
            ellipsis: glyph_set::current().ellipsis,
        }
    }
}

/// Pack `segments` into a single line ≤ `width` columns wide.
///
/// Returns an empty vec when `segments` is empty or `width == 0`. When at
/// least one segment is keepable, the result alternates segment spans and
/// separator spans (segment, sep, segment, sep, ..., segment).
pub fn pack(segments: &[Segment], width: u16, opts: &PackOptions) -> Vec<Span<'static>> {
    let width = width as usize;
    if segments.is_empty() || width == 0 {
        return Vec::new();
    }

    let n = segments.len();
    let sep_w = UnicodeWidthStr::width(opts.separator);
    let mut widths: Vec<usize> = segments
        .iter()
        .map(|s| UnicodeWidthStr::width(s.text.as_str()))
        .collect();

    // Drop order: higher priority first; ties broken by *later* position
    // so a high-priority trailing decoration evicts before its earlier
    // neighbour with the same priority.
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|a, b| {
        segments[*b]
            .priority
            .cmp(&segments[*a].priority)
            .then(b.cmp(a))
    });

    let mut kept: Vec<bool> = vec![true; n];

    // total_width helper closure: sum of segment widths + (kept-1) * sep.
    let total_width = |widths: &[usize], kept: &[bool]| -> (usize, usize) {
        let mut total = 0usize;
        let mut count = 0usize;
        for (i, &k) in kept.iter().enumerate() {
            if k {
                total = total.saturating_add(widths[i]);
                count += 1;
            }
        }
        if count > 1 {
            total = total.saturating_add(sep_w.saturating_mul(count - 1));
        }
        (total, count)
    };

    // Phase 1: drop in priority order until we fit or only one survives.
    let mut order_idx = 0usize;
    loop {
        let (total, count) = total_width(&widths, &kept);
        if total <= width || count <= 1 {
            break;
        }
        // Find next drop candidate (skip already-dropped).
        while order_idx < order.len() && !kept[order[order_idx]] {
            order_idx += 1;
        }
        if order_idx >= order.len() {
            break;
        }
        kept[order[order_idx]] = false;
        order_idx += 1;
    }

    // Phase 2: if a single survivor is still too wide, ellipsise it.
    let (final_total, final_count) = total_width(&widths, &kept);
    let mut truncated_text: Option<(usize, String)> = None;
    if final_count == 1 && final_total > width {
        let idx = kept.iter().position(|k| *k).expect("count == 1");
        let trimmed = truncate_with_ellipsis(&segments[idx].text, width, opts.ellipsis);
        widths[idx] = UnicodeWidthStr::width(trimmed.as_str());
        truncated_text = Some((idx, trimmed));
    }

    // Render: interleave kept segments + separator spans.
    let mut out: Vec<Span<'static>> = Vec::new();
    let mut first = true;
    for (i, seg) in segments.iter().enumerate() {
        if !kept[i] {
            continue;
        }
        if !first {
            out.push(Span::styled(
                opts.separator.to_string(),
                opts.separator_style,
            ));
        }
        first = false;
        let text = match &truncated_text {
            Some((ti, t)) if *ti == i => t.clone(),
            _ => seg.text.clone(),
        };
        out.push(Span::styled(text, seg.style));
    }
    out
}

/// Truncate `text` to fit in `width` columns, suffixing `ellipsis` when
/// any content is dropped. Always keeps at least one character of content
/// (plus the ellipsis) unless `width` is smaller than the ellipsis itself,
/// in which case we return a prefix of the ellipsis trimmed to `width`.
fn truncate_with_ellipsis(text: &str, width: usize, ellipsis: &str) -> String {
    if width == 0 {
        return String::new();
    }
    let text_w = UnicodeWidthStr::width(text);
    if text_w <= width {
        return text.to_string();
    }
    let ell_w = UnicodeWidthStr::width(ellipsis);
    if width <= ell_w {
        // Caller asked for less room than the ellipsis itself — return a
        // best-effort prefix of the ellipsis so we still fit the budget.
        let mut out = String::new();
        let mut cols = 0usize;
        for g in ellipsis.graphemes(true) {
            let w = UnicodeWidthStr::width(g);
            if cols + w > width {
                break;
            }
            out.push_str(g);
            cols += w;
        }
        return out;
    }
    let budget = width - ell_w;
    let mut out = String::with_capacity(text.len());
    let mut cols = 0usize;
    for g in text.graphemes(true) {
        let w = UnicodeWidthStr::width(g);
        if cols + w > budget {
            break;
        }
        out.push_str(g);
        cols += w;
    }
    out.push_str(ellipsis);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn segs() -> Vec<Segment> {
        // Five-segment fixture from spec — priorities [0, 0, 2, 3, 4],
        // last-listed drops first.
        vec![
            Segment::new("▸▸ bypass permissions", Style::default(), 0),
            Segment::new("2 local agents", Style::default(), 0),
            Segment::new("workspace-write", Style::default(), 2),
            Segment::new("on-request", Style::default(), 3),
            Segment::new("ctrl+t to hide", Style::default(), 4),
        ]
    }

    fn rendered_text(spans: &[Span<'_>]) -> String {
        spans.iter().map(|s| s.content.as_ref()).collect()
    }

    fn rendered_width(spans: &[Span<'_>]) -> usize {
        spans
            .iter()
            .map(|s| UnicodeWidthStr::width(s.content.as_ref()))
            .sum()
    }

    fn kept_segment_count(spans: &[Span<'_>], sep: &str) -> usize {
        // Segments and separators alternate; count = (len + 1) / 2 when at
        // least one non-separator survives.
        if spans.is_empty() {
            return 0;
        }
        let seps = spans.iter().filter(|s| s.content.as_ref() == sep).count();
        spans.len() - seps
    }

    #[test]
    fn pack_width_matrix() {
        // Verify drop-by-priority across a width sweep. Higher widths
        // keep more segments; lower widths shed them in priority order.
        // Fixture widths (cols):
        //   s0 "▸▸ bypass permissions"   → 21 (▸▸=2 + space + 18 ascii)
        //   s1 "2 local agents"          → 14
        //   s2 "workspace-write"         → 15
        //   s3 "on-request"              → 10
        //   s4 "ctrl+t to hide"          → 14
        // separators (" · ") = 3 cols apiece.
        let opts = PackOptions::default();
        let fixture = segs();
        // Widths (cols):
        //   s0 "▸▸ bypass permissions"   → 21 (▸=1 ×2 + space + 18 ascii)
        //   s1 "2 local agents"          → 14
        //   s2 "workspace-write"         → 15
        //   s3 "on-request"              → 10
        //   s4 "ctrl+t to hide"          → 14
        // Separator " · " = 3 cols each.
        // Full row total: 74 + 4*3 = 86.
        let cases: &[(u16, usize)] = &[
            // 120 cols → everything fits (86 ≤ 120).
            (120, 5),
            // 100 cols → still fits (86 ≤ 100).
            (100, 5),
            // 80 cols → 86 > 80; drop pri=4 → 21+14+15+10 + 3*3 = 69 ≤ 80. count = 4.
            (80, 4),
            // 60 cols → continue: 69 > 60; drop pri=3 → 21+14+15 + 2*3 = 56 ≤ 60. count = 3.
            (60, 3),
            // 40 cols → continue: 56 > 40; drop pri=2 → 21+14 + 3 = 38 ≤ 40. count = 2.
            (40, 2),
        ];
        for (width, expected_count) in cases {
            let out = pack(&fixture, *width, &opts);
            let count = kept_segment_count(&out, opts.separator);
            assert_eq!(
                count,
                *expected_count,
                "at width {width}: expected {expected_count} kept, got {count}, text={:?}",
                rendered_text(&out)
            );
            assert!(
                rendered_width(&out) <= *width as usize,
                "at width {width}: rendered width {} exceeds budget; text={:?}",
                rendered_width(&out),
                rendered_text(&out)
            );
        }
    }

    #[test]
    fn pack_drop_order_respects_priority_ties_by_position() {
        // Two pri=0 anchors: leading one survives, trailing-most pri=0
        // would never drop (we only drop higher priorities first). To
        // exercise tie-breaking, give them pri=5,5 and force one drop.
        let segs = vec![
            Segment::new("first", Style::default(), 5),
            Segment::new("second", Style::default(), 5),
        ];
        // width = 5 → can fit "first" (5) but not "first · second" (16).
        let out = pack(&segs, 5, &PackOptions::default());
        let text = rendered_text(&out);
        assert_eq!(text, "first", "later pri-tied segment should drop first");
    }

    #[test]
    fn pack_empty_input_returns_empty() {
        let out = pack(&[], 80, &PackOptions::default());
        assert!(out.is_empty());
    }

    #[test]
    fn pack_zero_width_returns_empty() {
        let out = pack(&segs(), 0, &PackOptions::default());
        assert!(out.is_empty());
    }

    #[test]
    fn pack_single_segment_too_wide_truncates_with_ellipsis() {
        let segs = vec![Segment::new(
            "this status text is way too long to fit",
            Style::default(),
            0,
        )];
        let out = pack(&segs, 10, &PackOptions::default());
        assert_eq!(out.len(), 1, "no separator, single span expected");
        let text = rendered_text(&out);
        assert!(
            text.ends_with('…'),
            "expected ellipsis suffix, got {text:?}"
        );
        assert_eq!(
            UnicodeWidthStr::width(text.as_str()),
            10,
            "truncated span should fill the exact budget"
        );
    }

    #[test]
    fn pack_unicode_width_counts_wide_glyphs() {
        // "▸▸" is 2 cells (each arrow glyph is 1 col in BMP; verify our
        // accounting matches ratatui's display width assumption).
        let s = "▸▸";
        let w = UnicodeWidthStr::width(s);
        // Two narrow arrows take 2 cells (each is 1-col per unicode-width
        // tables — keep this assertion explicit so future toolchain
        // upgrades that change the table will trip a clear failure).
        assert_eq!(w, 2, "▸▸ should occupy {w} cells per unicode-width");

        // Now feed it through pack and ensure width is respected.
        let segs = vec![
            Segment::new("▸▸ banner", Style::default(), 0),
            Segment::new("extra", Style::default(), 9),
        ];
        // width = 9 → "▸▸ banner" alone is 9 cols, "▸▸ banner · extra"
        // would be 9+3+5 = 17. So pri=9 must drop.
        let out = pack(&segs, 9, &PackOptions::default());
        assert_eq!(rendered_text(&out), "▸▸ banner");
    }

    #[test]
    fn pack_keeps_one_segment_when_all_drop_candidates_consumed() {
        // All segments have priority>0 except one anchor; verify the
        // anchor survives when budget collapses.
        let segs = vec![
            Segment::new("anchor", Style::default(), 0),
            Segment::new("opt1", Style::default(), 1),
            Segment::new("opt2", Style::default(), 2),
        ];
        let out = pack(&segs, 6, &PackOptions::default());
        assert_eq!(rendered_text(&out), "anchor");
    }

    #[test]
    fn pack_width_just_fits_keeps_all() {
        // Exact boundary: total == width → nothing drops.
        let segs = vec![
            Segment::new("a", Style::default(), 0),
            Segment::new("b", Style::default(), 0),
        ];
        // "a" (1) + " · " (3) + "b" (1) = 5.
        let out = pack(&segs, 5, &PackOptions::default());
        assert_eq!(rendered_text(&out), "a · b");
    }

    #[test]
    fn truncate_with_ellipsis_keeps_budget() {
        assert_eq!(truncate_with_ellipsis("hello world", 8, "…"), "hello w…");
        assert_eq!(truncate_with_ellipsis("hi", 8, "…"), "hi");
        // Width smaller than ellipsis → empty (or partial ellipsis prefix).
        assert_eq!(truncate_with_ellipsis("hello", 0, "…"), "");
    }
}
