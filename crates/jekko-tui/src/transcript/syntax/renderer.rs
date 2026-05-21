#[derive(Clone, Copy)]
struct RenderStyles {
    body: Style,
    quote: Style,
    heading: Style,
    bullet: Style,
    link: Style,
    inline_code: Style,
    rule: Style,
    code_plain: Style,
}

impl RenderStyles {
    fn for_mode(mode: ThemeMode, muted: bool) -> Self {
        let pal = theme::palette(mode);
        let body = if muted {
            Style::default()
                .fg(pal.text_muted)
                .add_modifier(Modifier::ITALIC)
        } else {
            Style::default().fg(pal.text)
        };
        Self {
            body,
            quote: Style::default()
                .fg(pal.text_muted)
                .add_modifier(Modifier::ITALIC),
            heading: Style::default().fg(pal.accent).add_modifier(Modifier::BOLD),
            bullet: Style::default().fg(pal.text_muted),
            link: Style::default()
                .fg(theme::INFO)
                .add_modifier(Modifier::UNDERLINED),
            inline_code: Style::default()
                .fg(theme::ACCENT)
                .add_modifier(Modifier::BOLD),
            rule: Style::default().fg(pal.border),
            code_plain: Style::default().fg(pal.text).add_modifier(Modifier::DIM),
        }
    }
}

#[derive(Clone, Copy)]
enum InlineKind {
    Emphasis,
    Strong,
    Link,
}

#[derive(Clone, Debug)]
struct ListState {
    ordered: bool,
    next_index: usize,
}

struct MarkdownRenderer {
    styles: RenderStyles,
    lines: Vec<Line<'static>>,
    current_prefix: Vec<Span<'static>>,
    current: Vec<Span<'static>>,
    quote_depth: usize,
    list_stack: Vec<ListState>,
    current_item_marker: Option<String>,
    heading_marker: Option<String>,
    inline_stack: Vec<InlineKind>,
}

impl MarkdownRenderer {
    fn new(styles: RenderStyles) -> Self {
        Self {
            styles,
            lines: Vec::new(),
            current_prefix: Vec::new(),
            current: Vec::new(),
            quote_depth: 0,
            list_stack: Vec::new(),
            current_item_marker: None,
            heading_marker: None,
            inline_stack: Vec::new(),
        }
    }

    fn finish(mut self) -> Vec<Line<'static>> {
        self.flush_current_line(false);
        while self.lines.first().is_some_and(is_blank_line) {
            self.lines.remove(0);
        }
        while self.lines.last().is_some_and(is_blank_line) {
            self.lines.pop();
        }
        self.lines
    }

    fn is_block_context_active(&self) -> bool {
        self.quote_depth > 0 || self.current_item_marker.is_some() || self.heading_marker.is_some()
    }

    fn refresh_prefix(&mut self) {
        if self.current.is_empty() {
            self.current_prefix = self.build_prefix();
        }
    }

    fn build_prefix(&self) -> Vec<Span<'static>> {
        let mut spans = Vec::new();
        if self.quote_depth > 0 {
            let mut quote = String::new();
            for _ in 0..self.quote_depth {
                quote.push_str("> ");
            }
            spans.push(Span::styled(quote, self.styles.quote));
        }
        if let Some(marker) = &self.heading_marker {
            spans.push(Span::styled(marker.clone(), self.styles.heading));
            spans.push(Span::styled(" ", self.styles.heading));
        }
        if let Some(marker) = &self.current_item_marker {
            spans.push(Span::styled(marker.clone(), self.styles.bullet));
            spans.push(Span::styled(" ", self.styles.bullet));
        }
        spans
    }

    fn current_text_style(&self) -> Style {
        let mut style = if self.heading_marker.is_some() {
            self.styles.heading
        } else if self.quote_depth > 0 {
            self.styles.quote
        } else {
            self.styles.body
        };
        for kind in &self.inline_stack {
            match kind {
                InlineKind::Emphasis => {
                    style = style.add_modifier(Modifier::ITALIC);
                }
                InlineKind::Strong => {
                    style = style.add_modifier(Modifier::BOLD);
                }
                InlineKind::Link => {
                    style = self.styles.link;
                }
            }
        }
        style
    }

    fn push_text(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        self.refresh_prefix();
        let style = self.current_text_style();
        for piece in text.split_inclusive('\n') {
            if let Some(stripped) = piece.strip_suffix('\n') {
                if !stripped.is_empty() {
                    self.current.push(Span::styled(stripped.to_string(), style));
                }
                self.flush_current_line(true);
                self.refresh_prefix();
            } else {
                self.current.push(Span::styled(piece.to_string(), style));
            }
        }
    }

    fn push_inline_code(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        self.refresh_prefix();
        self.current
            .push(Span::styled(text.to_string(), self.styles.inline_code));
    }

    fn flush_current_line(&mut self, keep_blank_if_empty: bool) {
        if self.current.is_empty() {
            if keep_blank_if_empty && !self.is_block_context_active() {
                self.lines.push(Line::from(Vec::<Span<'static>>::new()));
            }
            return;
        }
        let mut spans = self.current_prefix.clone();
        spans.append(&mut self.current);
        self.lines.push(Line::from(spans));
        self.current.clear();
        self.current_prefix.clear();
    }

    fn soft_break(&mut self) {
        self.flush_current_line(false);
        self.refresh_prefix();
    }

    fn newline_after_block(&mut self, blank_after: bool) {
        self.flush_current_line(false);
        if blank_after {
            self.lines.push(Line::from(Vec::<Span<'static>>::new()));
        }
        self.current.clear();
        self.current_prefix = self.build_prefix();
    }

    fn heading_marker(level: HeadingLevel) -> String {
        let count = match level {
            HeadingLevel::H1 => 1,
            HeadingLevel::H2 => 2,
            HeadingLevel::H3 => 3,
            HeadingLevel::H4 => 4,
            HeadingLevel::H5 => 5,
            HeadingLevel::H6 => 6,
        };
        "#".repeat(count)
    }

    fn list_marker(&mut self) -> String {
        if let Some(last) = self.list_stack.last_mut() {
            if last.ordered {
                let marker = format!("{}.", last.next_index);
                last.next_index = last.next_index.saturating_add(1);
                marker
            } else {
                "-".to_string()
            }
        } else {
            "-".to_string()
        }
    }
}
