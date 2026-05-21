/// 3-row composer shell rendered into the footer.
struct ComposerShell<'a> {
    input: &'a str,
    streaming: bool,
    streaming_preview: Option<&'a str>,
    empty_hint: Option<&'a str>,
    branch: Option<&'a str>,
}

impl Widget for ComposerShell<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // 3-row shell: top rule, prompt row, bottom rule.
        let layout = Layout::vertical([
            Constraint::Length(1), // top rule
            Constraint::Length(1), // composer row
            Constraint::Length(1), // bottom rule
        ])
        .split(area);

        render_full_width_rule(buf, layout[0]);

        render_composer_row(
            buf,
            layout[1],
            self.input,
            self.streaming,
            self.streaming_preview,
            self.empty_hint,
        );

        render_bottom_rule_with_branch(buf, layout[2], self.branch);
    }
}

/// Paint the composer prompt row(s) — the live wiring for T1-V1b.
///
/// Layout:
/// - Column 0 of row 0: the blue `›` glyph ([`PROMPT_GLYPH`] in [`BLUE_PATH`]).
/// - Column 1 of row 0: a blank space (so body text starts at column 2).
/// - Continuation rows (1..N): two blank columns then body text — no second
///   chevron, ensuring wrapped/Shift+Enter rows align under the first body
///   character of row 0.
///
/// The body is rendered into the area shifted right by [`PROMPT_PREFIX_WIDTH`]
/// columns. The single source of truth for the glyph + gutter width lives in
/// `crate::prompt::widget` — this helper just paints them into the chrome the
/// inline runtime owns.
///
/// Exposed `#[doc(hidden)] pub` so the snapshot tests in
/// `tests/inline_snapshots.rs` can drive the exact same render path the
/// runtime uses.
#[doc(hidden)]
pub fn render_composer_row(
    buf: &mut Buffer,
    area: Rect,
    input: &str,
    streaming: bool,
    streaming_preview: Option<&str>,
    empty_hint: Option<&str>,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    // Paint the prefix gutter on every row. Row 0 gets the blue `›` + blank;
    // continuation rows get two blanks. We only carve out the gutter when the
    // area is wide enough to leave at least one body column; below that, fall
    // back to a gutter-less paint so we don't lose all editable space.
    let gutter_fits = area.width > PROMPT_PREFIX_WIDTH;
    if gutter_fits {
        let glyph_style = Style::default().fg(theme::codex_blue_path());
        let blank_style = Style::default();
        for row_offset in 0..area.height {
            let y = area.y + row_offset;
            if row_offset == 0 {
                buf.set_string(area.x, y, PROMPT_GLYPH, glyph_style);
                buf.set_string(area.x + 1, y, " ", blank_style);
            } else {
                buf.set_string(area.x, y, "  ", blank_style);
            }
        }
    }

    let body_area = if gutter_fits {
        Rect {
            x: area.x + PROMPT_PREFIX_WIDTH,
            y: area.y,
            width: area.width - PROMPT_PREFIX_WIDTH,
            height: area.height,
        }
    } else {
        area
    };

    let composer_line = if streaming {
        #[allow(clippy::manual_unwrap_or_default)]
        let preview = match streaming_preview.map(|s| {
            #[allow(clippy::manual_unwrap_or)]
            let last_line = match s.lines().last() {
                Some(line) => line,
                None => "",
            };
                let max = (body_area.width as usize).saturating_sub(12);
                if last_line.len() > max && max > 1 {
                    // T-GLYPH-WAVE2: ellipsis chrome glyph defers to GlyphMode.
                    format!(
                        "{}{}",
                        glyph_set::current().ellipsis,
                        &last_line[last_line.len() - (max - 1)..]
                    )
                } else {
                    last_line.to_string()
                }
        }) {
            Some(preview) => preview,
            None => String::new(),
        };
        Line::from(vec![
            Span::styled("⋯ ", Style::default().fg(theme::codex_orange_agent())),
            Span::styled("streaming  ", Style::default().fg(theme::codex_fg_dim())),
            Span::styled(preview, Style::default().fg(theme::codex_fg())),
        ])
    } else if input.is_empty() {
        let hint = match empty_hint {
            Some(text) => truncate_to_width(text, body_area.width as usize),
            None => String::new(),
        };
        Line::from(vec![Span::styled(
            hint,
            Style::default().fg(theme::codex_fg_dim()),
        )])
    } else {
        // Render last line of composer (for multi-line input, earlier lines
        // flow above eventually; v1 keeps this simple).
        let display = input.lines().last().unwrap_or("").to_string();
        Line::from(vec![
            Span::styled(display, Style::default().fg(theme::codex_fg_strong())),
            Span::styled(
                " ",
                Style::default().bg(theme::codex_fg()).fg(theme::codex_bg()),
            ),
        ])
    };
    Paragraph::new(composer_line).render(body_area, buf);
}
