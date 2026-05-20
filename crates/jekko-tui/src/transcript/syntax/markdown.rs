pub fn render_markdown_lines(text: &str, mode: ThemeMode, muted: bool) -> Vec<Line<'static>> {
    let styles = RenderStyles::for_mode(mode, muted);
    let mut renderer = MarkdownRenderer::new(styles);
    let parser = Parser::new_ext(text, Options::empty());

    let mut in_code_block: Option<CodeBlockKind<'static>> = None;
    let mut code_block = String::new();

    for event in parser {
        match event {
            Event::Start(Tag::Paragraph) => {
                renderer.current_prefix = renderer.build_prefix();
            }
            Event::End(TagEnd::Paragraph) => {
                renderer.newline_after_block(!renderer.is_block_context_active());
            }
            Event::Start(Tag::Heading { level, .. }) => {
                renderer.heading_marker = Some(MarkdownRenderer::heading_marker(level));
                renderer.current_prefix = renderer.build_prefix();
            }
            Event::End(TagEnd::Heading(_)) => {
                renderer.newline_after_block(true);
                renderer.heading_marker = None;
            }
            Event::Start(Tag::BlockQuote(_)) => {
                renderer.quote_depth = renderer.quote_depth.saturating_add(1);
                renderer.current_prefix = renderer.build_prefix();
            }
            Event::End(TagEnd::BlockQuote(_)) => {
                renderer.newline_after_block(false);
                renderer.quote_depth = renderer.quote_depth.saturating_sub(1);
            }
            Event::Start(Tag::List(start)) => {
                renderer.list_stack.push(ListState {
                    ordered: start.is_some(),
                    next_index: start.unwrap_or(1) as usize,
                });
            }
            Event::End(TagEnd::List(_)) => {
                renderer.newline_after_block(false);
                renderer.list_stack.pop();
            }
            Event::Start(Tag::Item) => {
                renderer.current_item_marker = Some(renderer.list_marker());
                renderer.current_prefix = renderer.build_prefix();
            }
            Event::End(TagEnd::Item) => {
                renderer.newline_after_block(false);
                renderer.current_item_marker = None;
            }
            Event::Start(Tag::Emphasis) => renderer.inline_stack.push(InlineKind::Emphasis),
            Event::End(TagEnd::Emphasis) => {
                let _ = pop_inline(&mut renderer.inline_stack, InlineKind::Emphasis);
            }
            Event::Start(Tag::Strong) => renderer.inline_stack.push(InlineKind::Strong),
            Event::End(TagEnd::Strong) => {
                let _ = pop_inline(&mut renderer.inline_stack, InlineKind::Strong);
            }
            Event::Start(Tag::Link { .. }) => renderer.inline_stack.push(InlineKind::Link),
            Event::End(TagEnd::Link) => {
                let _ = pop_inline(&mut renderer.inline_stack, InlineKind::Link);
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                in_code_block = Some(kind.into_static());
                code_block.clear();
                renderer.flush_current_line(false);
            }
            Event::End(TagEnd::CodeBlock) => {
                renderer
                    .lines
                    .extend(render_code_block(&code_block, in_code_block.as_ref(), mode));
                code_block.clear();
                in_code_block = None;
            }
            Event::Text(text) => {
                if in_code_block.is_some() {
                    code_block.push_str(&text);
                } else {
                    renderer.push_text(&text);
                }
            }
            Event::Code(code) => {
                renderer.push_inline_code(&code);
            }
            Event::SoftBreak => {
                if in_code_block.is_some() {
                    code_block.push('\n');
                } else {
                    renderer.soft_break();
                }
            }
            Event::HardBreak => {
                if in_code_block.is_some() {
                    code_block.push('\n');
                } else {
                    renderer.soft_break();
                }
            }
            Event::Rule => {
                renderer.flush_current_line(false);
                renderer.lines.push(Line::from(vec![Span::styled(
                    "────────────────",
                    renderer.styles.rule,
                )]));
            }
            Event::TaskListMarker(checked) => {
                renderer.refresh_prefix();
                let marker = if checked { "[x] " } else { "[ ] " };
                renderer
                    .current
                    .push(Span::styled(marker, renderer.styles.bullet));
            }
            Event::Html(text) | Event::InlineHtml(text) | Event::FootnoteReference(text) => {
                if in_code_block.is_some() {
                    code_block.push_str(&text);
                } else {
                    renderer.push_text(&text);
                }
            }
            Event::DisplayMath(text) | Event::InlineMath(text) => {
                if in_code_block.is_some() {
                    code_block.push_str(&text);
                } else {
                    renderer.push_inline_code(&text);
                }
            }
            _ => {}
        }
    }

    if in_code_block.is_some() {
        renderer
            .lines
            .extend(render_code_block(&code_block, in_code_block.as_ref(), mode));
    }

    renderer.finish()
}

fn pop_inline(stack: &mut Vec<InlineKind>, kind: InlineKind) -> bool {
    if let Some(pos) = stack
        .iter()
        .rposition(|entry| std::mem::discriminant(entry) == std::mem::discriminant(&kind))
    {
        stack.remove(pos);
        true
    } else {
        false
    }
}
