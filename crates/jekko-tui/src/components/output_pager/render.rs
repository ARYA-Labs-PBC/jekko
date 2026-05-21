use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

use crate::glyph_set;
use crate::theme::codex;

use super::state::{MatchRef, PagerMode, PagerState};

/// Render the pager into `area`. Top row is the status line, last row is the
/// optional search prompt when in `Search` mode, the rest is body.
pub fn render_pager(buf: &mut Buffer, area: Rect, state: &PagerState) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let show_search_prompt = state.mode == PagerMode::Search && area.height >= 3;
    let header_h = 1u16;
    let prompt_h = if show_search_prompt { 1 } else { 0 };
    let body_h = area.height.saturating_sub(header_h + prompt_h);

    render_status_row(buf, Rect::new(area.x, area.y, area.width, header_h), state);

    if body_h > 0 {
        render_body(
            buf,
            Rect::new(area.x, area.y + header_h, area.width, body_h),
            state,
        );
    }

    if show_search_prompt {
        render_search_prompt(
            buf,
            Rect::new(
                area.x,
                area.y + area.height.saturating_sub(1),
                area.width,
                1,
            ),
            state,
        );
    }
}

fn render_status_row(buf: &mut Buffer, area: Rect, state: &PagerState) {
    let dim = Style::default().fg(codex::FG_DIM);
    let very_dim = Style::default().fg(codex::FG_VERY_DIM);
    let label = format!(
        " Pager · {}/{} lines · {} matches ",
        state.scroll,
        state.lines.len(),
        state.matches.len()
    );
    let usable = area.width as usize;
    let label_max = usable.saturating_sub(8);
    let g = glyph_set::current();
    let trimmed = if label.chars().count() > label_max {
        let mut s = String::new();
        for ch in label.chars().take(label_max.saturating_sub(1)) {
            s.push(ch);
        }
        s.push_str(g.ellipsis);
        s
    } else {
        label
    };

    let total = usable;
    let lead_n: usize = 2;
    let tail_n = total.saturating_sub(lead_n + trimmed.chars().count());
    let lead: String = g.divider.repeat(lead_n);
    let tail: String = g.divider.repeat(tail_n);

    let spans: Vec<Span> = vec![
        Span::styled(lead, very_dim),
        Span::styled(trimmed, dim),
        Span::styled(tail, very_dim),
    ];
    Paragraph::new(Line::from(spans)).render(area, buf);
}

fn render_body(buf: &mut Buffer, area: Rect, state: &PagerState) {
    let body_style = Style::default().fg(codex::FG);
    let other_match = Style::default()
        .fg(codex::CYAN_TAB)
        .add_modifier(Modifier::UNDERLINED);
    let current_match = Style::default().bg(codex::YELLOW).fg(codex::BG_OVERLAY);

    let viewport_h = area.height as usize;
    let end = state
        .scroll
        .saturating_add(viewport_h)
        .min(state.lines.len());
    let visible = &state.lines[state.scroll..end];

    for (row_idx, line) in visible.iter().enumerate() {
        let absolute_line = state.scroll + row_idx;
        let row = Rect::new(area.x, area.y + row_idx as u16, area.width, 1);
        let spans = build_line_spans(
            line,
            absolute_line,
            &state.matches,
            state.current_match,
            body_style,
            other_match,
            current_match,
        );
        Paragraph::new(Line::from(spans)).render(row, buf);
    }
}

fn build_line_spans<'a>(
    line: &'a str,
    line_idx: usize,
    matches: &'a [MatchRef],
    current: Option<usize>,
    base: Style,
    other: Style,
    cur: Style,
) -> Vec<Span<'a>> {
    let mut hits: Vec<(usize, &MatchRef)> = matches
        .iter()
        .enumerate()
        .filter(|(_, m)| m.line == line_idx)
        .collect();
    hits.sort_by_key(|(_, m)| m.byte_start);
    if hits.is_empty() {
        return vec![Span::styled(line, base)];
    }

    let mut out: Vec<Span<'a>> = Vec::with_capacity(hits.len() * 2 + 1);
    let mut cursor = 0usize;
    for (idx, m) in hits {
        if m.byte_start > cursor {
            out.push(Span::styled(&line[cursor..m.byte_start], base));
        }
        let span_style = if Some(idx) == current { cur } else { other };
        out.push(Span::styled(&line[m.byte_start..m.byte_end], span_style));
        cursor = m.byte_end;
    }
    if cursor < line.len() {
        out.push(Span::styled(&line[cursor..], base));
    }
    out
}

fn render_search_prompt(buf: &mut Buffer, area: Rect, state: &PagerState) {
    let dim = Style::default().fg(codex::FG_DIM);
    let strong = Style::default().fg(codex::FG_STRONG);
    let spans: Vec<Span> = vec![
        Span::styled("/ ", dim),
        Span::styled(state.search_query.clone(), strong),
        Span::styled(glyph_set::current().cursor_block, strong),
    ];
    Paragraph::new(Line::from(spans)).render(area, buf);
}
