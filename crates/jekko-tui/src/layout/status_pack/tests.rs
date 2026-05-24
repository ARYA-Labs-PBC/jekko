use ratatui::style::Style;
use ratatui::text::Span;
use unicode_width::UnicodeWidthStr;

use super::packer::truncate_with_ellipsis;
use super::*;

fn segs() -> Vec<Segment> {
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
    if spans.is_empty() {
        return 0;
    }
    let seps = spans.iter().filter(|s| s.content.as_ref() == sep).count();
    spans.len() - seps
}

#[test]
fn pack_width_matrix() {
    let opts = PackOptions::default();
    let fixture = segs();
    let cases: &[(u16, usize)] = &[(120, 5), (100, 5), (80, 4), (60, 3), (40, 2)];
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
    let segs = vec![
        Segment::new("first", Style::default(), 5),
        Segment::new("second", Style::default(), 5),
    ];
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
    let s = "▸▸";
    let w = UnicodeWidthStr::width(s);
    assert_eq!(w, 2, "▸▸ should occupy {w} cells per unicode-width");

    let segs = vec![
        Segment::new("▸▸ banner", Style::default(), 0),
        Segment::new("extra", Style::default(), 9),
    ];
    let out = pack(&segs, 9, &PackOptions::default());
    assert_eq!(rendered_text(&out), "▸▸ banner");
}

#[test]
fn pack_keeps_one_segment_when_all_drop_candidates_consumed() {
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
    let segs = vec![
        Segment::new("a", Style::default(), 0),
        Segment::new("b", Style::default(), 0),
    ];
    let out = pack(&segs, 5, &PackOptions::default());
    assert_eq!(rendered_text(&out), "a · b");
}

#[test]
fn truncate_with_ellipsis_keeps_budget() {
    assert_eq!(truncate_with_ellipsis("hello world", 8, "…"), "hello w…");
    assert_eq!(truncate_with_ellipsis("hi", 8, "…"), "hi");
    assert_eq!(truncate_with_ellipsis("hello", 0, "…"), "");
}
