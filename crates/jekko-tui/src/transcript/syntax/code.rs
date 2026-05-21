fn render_code_block(
    code: &str,
    kind: Option<&CodeBlockKind<'static>>,
    mode: ThemeMode,
) -> Vec<Line<'static>> {
    let styles = RenderStyles::for_mode(mode, false);
    if code.is_empty() {
        return vec![Line::from(vec![Span::styled("", styles.code_plain)])];
    }

    let fenced_lang = match kind {
        Some(CodeBlockKind::Fenced(info)) => info
            .split_whitespace()
            .next()
            .and_then(|token| token.split(',').next())
            .unwrap_or("")
            .trim(),
        _ => "",
    };
    let use_syntect = kind
        .map(|k| matches!(k, CodeBlockKind::Fenced(_)) && code.len() <= MAX_CODE_BLOCK_BYTES)
        .unwrap_or(false)
        && !fenced_lang.is_empty();

    if !use_syntect {
        return plain_code_lines(code, styles.code_plain);
    }

    let syntax_set = syntax_set();
    let syntax = syntax_for_lang(syntax_set, fenced_lang);
    if syntax.name == "Plain Text" {
        return plain_code_lines(code, styles.code_plain);
    }

    let theme = syntax_theme(mode);
    let mut highlighter = HighlightLines::new(syntax, theme);
    let mut lines = Vec::new();

    for raw_line in code.split_inclusive('\n') {
        let line = if raw_line.ends_with('\n') {
            raw_line.to_string()
        } else {
            let mut owned = raw_line.to_string();
            owned.push('\n');
            owned
        };
        match highlighter.highlight_line(&line, syntax_set) {
            Ok(ranges) => {
                lines.push(Line::from(highlight_ranges(&ranges)));
            }
            Err(_) => {
                return plain_code_lines(code, styles.code_plain);
            }
        }
    }

    if lines.is_empty() {
        plain_code_lines(code, styles.code_plain)
    } else {
        lines
    }
}

fn plain_code_lines(code: &str, style: Style) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    for piece in code.split_inclusive('\n') {
        if let Some(stripped) = piece.strip_suffix('\n') {
            lines.push(Line::from(vec![Span::styled(stripped.to_string(), style)]));
        } else {
            lines.push(Line::from(vec![Span::styled(piece.to_string(), style)]));
        }
    }
    if lines.is_empty() {
        lines.push(Line::from(vec![Span::styled(String::new(), style)]));
    }
    lines
}

fn highlight_ranges(ranges: &[(SynStyle, &str)]) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    for (style, text) in ranges {
        if text.is_empty() {
            continue;
        }
        spans.push(Span::styled((*text).to_string(), syn_to_ratatui(*style)));
    }
    spans
}

fn syn_to_ratatui(style: SynStyle) -> Style {
    let mut out = Style::default().fg(Color::Rgb(
        style.foreground.r,
        style.foreground.g,
        style.foreground.b,
    ));
    if style.font_style.contains(FontStyle::BOLD) {
        out = out.add_modifier(Modifier::BOLD);
    }
    if style.font_style.contains(FontStyle::ITALIC) {
        out = out.add_modifier(Modifier::ITALIC);
    }
    if style.font_style.contains(FontStyle::UNDERLINE) {
        out = out.add_modifier(Modifier::UNDERLINED);
    }
    out
}
