fn render_slash_popup(buf: &mut Buffer, area: Rect, slash: &SlashState, catalog: &SlashCatalog) {
    if area.height == 0 || area.width == 0 {
        return;
    }
    // Title row + N command rows; cap visible rows to area height.
    let max_rows = area.height as usize;
    if max_rows == 0 {
        return;
    }
    if let Some(state) = &slash.submenu {
        if let Some(submenu) = catalog.submenu_for(&state.parent_id) {
            render_slash_submenu_popup(buf, area, state, submenu);
            return;
        }
    }
    let title_area = Rect::new(area.x, area.y, area.width, 1);
    let title = Line::from(vec![
        Span::styled(
            " / commands  ",
            Style::default().fg(theme::codex_cyan_tab()),
        ),
        Span::styled(
            format!(
                "({}/{})",
                slash.filtered.len(),
                slash_command_visible_count(catalog)
            ),
            Style::default().fg(theme::codex_fg_very_dim()),
        ),
    ]);
    Paragraph::new(title).render(title_area, buf);

    let body_rows = max_rows.saturating_sub(1);
    if body_rows == 0 || slash.filtered.is_empty() {
        if slash.filtered.is_empty() && body_rows > 0 {
            let empty_area = Rect::new(area.x, area.y + 1, area.width, 1);
            let line = Line::from(vec![Span::styled(
                "   (no matches)",
                Style::default().fg(theme::codex_fg_very_dim()),
            )]);
            Paragraph::new(line).render(empty_area, buf);
        }
        return;
    }

    // Scroll window so cursor is visible.
    let cursor = slash.cursor.min(slash.filtered.len() - 1);
    let start = cursor.saturating_sub(body_rows - 1);
    let end = (start + body_rows).min(slash.filtered.len());

    for (offset, idx_pos) in (start..end).enumerate() {
        let cmd_id = &slash.filtered[idx_pos];
        let Some(cmd) = catalog.find(cmd_id) else {
            continue;
        };
        let selected = idx_pos == cursor;
        let row_area = Rect::new(area.x, area.y + 1 + offset as u16, area.width, 1);
        let marker = if selected {
            Span::styled(" › ", Style::default().fg(theme::codex_orange_agent()))
        } else {
            Span::raw("   ")
        };
        let id_style = if selected {
            Style::default()
                .fg(theme::codex_fg_strong())
                .bg(theme::codex_bg_overlay())
        } else {
            Style::default().fg(theme::codex_fg())
        };
        let desc_style = if selected {
            Style::default()
                .fg(theme::codex_fg_dim())
                .bg(theme::codex_bg_overlay())
        } else {
            Style::default().fg(theme::codex_fg_dim())
        };
        let row = Line::from(vec![
            marker,
            Span::styled(format!("/{:<10}", cmd.id()), id_style),
            Span::styled(" ", desc_style),
            Span::styled(cmd.description().to_string(), desc_style),
        ]);
        Paragraph::new(row).render(row_area, buf);
    }
}

fn render_slash_submenu_popup(
    buf: &mut Buffer,
    area: Rect,
    state: &SlashSubmenuState,
    submenu: &crate::slash::SlashSubmenu,
) {
    let max_rows = area.height as usize;
    let title_area = Rect::new(area.x, area.y, area.width, 1);
    let title = Line::from(vec![
        Span::styled(
            " / commands > ",
            Style::default().fg(theme::codex_cyan_tab()),
        ),
        Span::styled(
            format!("/{}  ", submenu.parent_id),
            Style::default().fg(theme::codex_fg()),
        ),
        Span::styled(
            format!("({})", submenu.items.len()),
            Style::default().fg(theme::codex_fg_very_dim()),
        ),
    ]);
    Paragraph::new(title).render(title_area, buf);

    let body_rows = max_rows.saturating_sub(1);
    if body_rows == 0 || submenu.items.is_empty() {
        return;
    }

    let cursor = state.cursor.min(submenu.items.len() - 1);
    let start = cursor.saturating_sub(body_rows - 1);
    let end = (start + body_rows).min(submenu.items.len());

    for (offset, idx_pos) in (start..end).enumerate() {
        let item = &submenu.items[idx_pos];
        let selected = idx_pos == cursor;
        let row_area = Rect::new(area.x, area.y + 1 + offset as u16, area.width, 1);
        let marker = if selected {
            Span::styled(" › ", Style::default().fg(theme::codex_orange_agent()))
        } else {
            Span::raw("   ")
        };
        let id_style = if selected {
            Style::default()
                .fg(theme::codex_fg_strong())
                .bg(theme::codex_bg_overlay())
        } else {
            Style::default().fg(theme::codex_fg())
        };
        let desc_style = if selected {
            Style::default()
                .fg(theme::codex_fg_dim())
                .bg(theme::codex_bg_overlay())
        } else {
            Style::default().fg(theme::codex_fg_dim())
        };
        let row = Line::from(vec![
            marker,
            Span::styled(format!("{:<18}", item.id), id_style),
            Span::styled(" ", desc_style),
            Span::styled(item.description.to_string(), desc_style),
        ]);
        Paragraph::new(row).render(row_area, buf);
    }
}

fn render_mention_popup(buf: &mut Buffer, area: Rect, mention: &MentionState, index_len: usize) {
    if area.height == 0 || area.width == 0 {
        return;
    }
    let max_rows = area.height as usize;
    if max_rows == 0 {
        return;
    }
    let title_area = Rect::new(area.x, area.y, area.width, 1);
    let title = Line::from(vec![
        Span::styled(" @ files  ", Style::default().fg(theme::codex_cyan_tab())),
        Span::styled(
            format!("({}/{})", mention.filtered.len(), index_len),
            Style::default().fg(theme::codex_fg_very_dim()),
        ),
    ]);
    Paragraph::new(title).render(title_area, buf);

    let body_rows = max_rows.saturating_sub(1);
    if body_rows == 0 {
        return;
    }
    if mention.filtered.is_empty() {
        let empty_area = Rect::new(area.x, area.y + 1, area.width, 1);
        let line = Line::from(vec![Span::styled(
            "   (no matches)",
            Style::default().fg(theme::codex_fg_very_dim()),
        )]);
        Paragraph::new(line).render(empty_area, buf);
        return;
    }

    let cursor = mention.cursor.min(mention.filtered.len() - 1);
    let start = cursor.saturating_sub(body_rows - 1);
    let end = (start + body_rows).min(mention.filtered.len());

    for (offset, idx_pos) in (start..end).enumerate() {
        let path = &mention.filtered[idx_pos];
        let selected = idx_pos == cursor;
        let row_area = Rect::new(area.x, area.y + 1 + offset as u16, area.width, 1);
        let marker = if selected {
            Span::styled(" › ", Style::default().fg(theme::codex_orange_agent()))
        } else {
            Span::raw("   ")
        };

        let (dir_part, base_part) = split_dir_base(path);
        let base_style = if selected {
            Style::default()
                .fg(theme::codex_fg_strong())
                .bg(theme::codex_bg_overlay())
        } else {
            Style::default().fg(theme::codex_fg())
        };
        let dir_style = if selected {
            Style::default()
                .fg(theme::codex_fg_dim())
                .bg(theme::codex_bg_overlay())
        } else {
            Style::default().fg(theme::codex_fg_very_dim())
        };

        let row = Line::from(vec![
            marker,
            Span::styled(dir_part, dir_style),
            Span::styled(base_part, base_style),
        ]);
        Paragraph::new(row).render(row_area, buf);
    }
}

fn split_dir_base(path: &std::path::Path) -> (String, String) {
    let s = path.to_string_lossy().to_string();
    match path.file_name() {
        Some(base) => {
            let base_str = base.to_string_lossy().to_string();
            if s.len() > base_str.len() {
                let dir_len = s.len() - base_str.len();
                (s[..dir_len].to_string(), base_str)
            } else {
                (String::new(), base_str)
            }
        }
        None => (s, String::new()),
    }
}

/// Bottom rule of the composer chrome. When a branch is supplied, overlay it
/// on the right edge as a cyan tab `─── branch ───`-style — matches the Codex
/// CLI screenshots in the user brief. Falls back to a plain full-width rule
/// when there is no branch, the rule is too narrow, or the branch is too long
/// to fit alongside at least 8 cells of leading rule.
fn render_bottom_rule_with_branch(buf: &mut Buffer, area: Rect, branch: Option<&str>) {
    if area.height == 0 || area.width == 0 {
        return;
    }
    let width = area.width as usize;
    // T-GLYPH-WAVE2: chrome glyphs (rule + ellipsis truncation) honor
    // GlyphMode so the composer chrome reads as plain ASCII when
    // `JEKKO_ASCII=1`.
    let g = glyph_set::current();
    let rule_char = g.divider;
    let full_rule = rule_char.repeat(width);

    let Some(branch) = branch.filter(|b| !b.is_empty()) else {
        Paragraph::new(Line::from(Span::styled(
            full_rule,
            Style::default().fg(theme::codex_rule()),
        )))
        .render(area, buf);
        return;
    };

    // Tab label: ` <branch> `. Truncate the branch from the left if it would
    // leave fewer than 8 cells of rule before it.
    let min_rule_before_tab = 8usize;
    let padding = 2usize;
    let max_tab_inner = width.saturating_sub(min_rule_before_tab + padding);
    if max_tab_inner == 0 {
        Paragraph::new(Line::from(Span::styled(
            full_rule,
            Style::default().fg(theme::codex_rule()),
        )))
        .render(area, buf);
        return;
    }

    let label: String = if branch.chars().count() <= max_tab_inner {
        branch.to_string()
    } else {
        let take = max_tab_inner.saturating_sub(1).max(1);
        let suffix: String = branch
            .chars()
            .rev()
            .take(take)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        // T-GLYPH-WAVE2: branch-truncation ellipsis honors GlyphMode.
        format!("{}{suffix}", g.ellipsis)
    };
    let tab = format!(" {label} ");
    let tab_cells = tab.chars().count();
    let rule_cells = width.saturating_sub(tab_cells);
    let rule_left = rule_char.repeat(rule_cells);

    // Branch tab: explicit Color::Black for fg so the label is legible against
    // the cyan tab background. `codex_bg()` returns `Color::Reset` (terminal
    // default) which renders as the foreground default on most dark themes —
    // visually white-on-cyan, hard to read. Black-on-cyan matches Codex CLI.
    let line = Line::from(vec![
        Span::styled(rule_left, Style::default().fg(theme::codex_rule())),
        Span::styled(
            tab,
            Style::default()
                .fg(Color::Black)
                .bg(theme::codex_cyan_tab()),
        ),
    ]);
    Paragraph::new(line).render(area, buf);
}

fn render_full_width_rule(buf: &mut Buffer, area: Rect) {
    if area.height == 0 || area.width == 0 {
        return;
    }
    let line = "─".repeat(area.width as usize);
    Paragraph::new(Line::from(Span::styled(
        line,
        Style::default().fg(theme::codex_rule()),
    )))
    .render(area, buf);
}
