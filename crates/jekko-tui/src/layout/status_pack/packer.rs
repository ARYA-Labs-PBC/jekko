use ratatui::text::Span;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use super::{PackOptions, Segment};

/// Pack `segments` into a single line ≤ `width` columns wide.
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
    let mut kept: Vec<bool> = vec![true; n];
    drop_until_fit(segments, &widths, &mut kept, width, sep_w);

    let (final_total, final_count) = total_width(&widths, &kept, sep_w);
    let mut truncated_text: Option<(usize, String)> = None;
    if final_count == 1 && final_total > width {
        let idx = kept.iter().position(|k| *k).expect("count == 1");
        let trimmed = truncate_with_ellipsis(&segments[idx].text, width, opts.ellipsis);
        widths[idx] = UnicodeWidthStr::width(trimmed.as_str());
        truncated_text = Some((idx, trimmed));
    }

    render_spans(segments, &kept, opts, truncated_text)
}

fn drop_until_fit(
    segments: &[Segment],
    widths: &[usize],
    kept: &mut [bool],
    width: usize,
    sep_w: usize,
) {
    let mut order: Vec<usize> = (0..segments.len()).collect();
    order.sort_by(|a, b| {
        segments[*b]
            .priority
            .cmp(&segments[*a].priority)
            .then(b.cmp(a))
    });

    let mut order_idx = 0usize;
    loop {
        let (total, count) = total_width(widths, kept, sep_w);
        if total <= width || count <= 1 {
            break;
        }
        while order_idx < order.len() && !kept[order[order_idx]] {
            order_idx += 1;
        }
        if order_idx >= order.len() {
            break;
        }
        kept[order[order_idx]] = false;
        order_idx += 1;
    }
}

fn total_width(widths: &[usize], kept: &[bool], sep_w: usize) -> (usize, usize) {
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
}

fn render_spans(
    segments: &[Segment],
    kept: &[bool],
    opts: &PackOptions,
    truncated_text: Option<(usize, String)>,
) -> Vec<Span<'static>> {
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
/// content is dropped.
pub(super) fn truncate_with_ellipsis(text: &str, width: usize, ellipsis: &str) -> String {
    if width == 0 {
        return String::new();
    }
    let text_w = UnicodeWidthStr::width(text);
    if text_w <= width {
        return text.to_string();
    }
    let ell_w = UnicodeWidthStr::width(ellipsis);
    if width <= ell_w {
        return ellipsis_prefix(ellipsis, width);
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

fn ellipsis_prefix(ellipsis: &str, width: usize) -> String {
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
    out
}
