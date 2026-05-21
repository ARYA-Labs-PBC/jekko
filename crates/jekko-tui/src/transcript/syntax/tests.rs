#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::widgets::Paragraph;
    use ratatui::Terminal;

    fn render_to_buffer(text: &str) -> (ratatui::buffer::Buffer, u16) {
        let lines = render_markdown_lines(text, ThemeMode::Dark, false);
        let width = 80;
        let height = lines.len().max(1) as u16 + 1;
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal
            .draw(|frame| frame.render_widget(Paragraph::new(lines.clone()), frame.area()))
            .unwrap();
        (terminal.backend().buffer().clone(), width)
    }

    fn row_text(buf: &ratatui::buffer::Buffer, width: u16, row: u16) -> String {
        (0..width)
            .map(|x| buf.content[(row as usize * width as usize) + x as usize].symbol())
            .collect()
    }

    #[test]
    fn inline_code_is_styled_differently() {
        let (buf, width) = render_to_buffer("Plain `code` sample.");
        let row = row_text(&buf, width, 0);
        assert!(row.contains("Plain"));
        let row_cells = &buf.content[0..width as usize];
        let code_fg = row_cells
            .iter()
            .find(|cell| cell.symbol() == "c")
            .map(|cell| cell.fg);
        let plain_fg = row_cells
            .iter()
            .find(|cell| cell.symbol() == "P")
            .map(|cell| cell.fg);
        assert!(code_fg.is_some() && plain_fg.is_some());
        assert_ne!(code_fg, plain_fg);
    }

    #[test]
    fn fenced_code_uses_multiple_styles() {
        let (buf, width) =
            render_to_buffer("# Title\n\n```rust\nfn demo() {\n    let n = 1;\n}\n```\n");
        let mut fg_colors = Vec::new();
        for row in 0..buf.area.height {
            for x in 0..width {
                let cell = &buf.content[(row as usize * width as usize) + x as usize];
                if cell.symbol().trim().is_empty() {
                    continue;
                }
                fg_colors.push(cell.fg);
            }
        }
        fg_colors.dedup();
        assert!(fg_colors.len() > 2, "expected more than one syntax color");
    }

    #[test]
    fn markdown_headings_and_lists_render_distinctly() {
        let lines = render_markdown_lines(
            "# Heading\n\n- item one\n- item two",
            ThemeMode::Dark,
            false,
        );
        let rendered = lines
            .iter()
            .flat_map(|line| line.spans.iter().map(|span| span.content.as_ref()))
            .collect::<String>();
        assert!(rendered.contains("Heading"));
        assert!(rendered.contains("item one"));
    }
}
