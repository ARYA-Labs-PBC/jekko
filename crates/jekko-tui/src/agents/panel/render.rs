pub fn render(
    panel: &AgentPanelState,
    now: Instant,
    opts: &PanelRenderOptions,
) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    if !panel.visible {
        return lines;
    }
    lines.push(render_strip(panel, opts, panel.focused));
    let mut visible_slots = panel.visible_agent_slots().min(opts.max_agents);
    let max_rows = opts.max_visible_rows.max(1);
    visible_slots = visible_slots.min(max_rows.saturating_sub(1) / 2);
    if visible_slots == 0 {
        return lines;
    }

    let start = panel
        .scroll_offset
        .min(panel.agents.len().saturating_sub(visible_slots));
    let end = (start + visible_slots).min(panel.agents.len());
    let width = opts.width.max(20);
    for (idx, agent) in panel.agents[start..end].iter().enumerate() {
        let selected = start + idx == panel.selected_index;
        lines.push(render_bullet_row(
            agent,
            now,
            selected,
            width,
            opts.compact,
            opts.motion_enabled,
        ));
        if !agent.summary.is_empty() {
            lines.push(render_summary_row(agent, width));
        }
    }
    lines
}

fn render_strip(
    panel: &AgentPanelState,
    opts: &PanelRenderOptions,
    focused: bool,
) -> Line<'static> {
    let local = panel.local_running_count();
    let pluralize = if local == 1 { "agent" } else { "agents" };
    // T-GLYPH-WAVE2: banner prefix glyph defers to GlyphMode (`>>` in ASCII).
    let banner = format!(" {} ", glyph_set::current().banner_prefix);
    let mut spans = vec![
        Span::styled(banner, Style::default().fg(codex_orange_agent())),
        Span::styled(
            opts.permission_mode_label.to_string(),
            Style::default().fg(codex_pink_agent()),
        ),
        Span::styled(" · ", Style::default().fg(codex_fg_very_dim())),
        Span::styled(
            format!("{local} local {pluralize}"),
            Style::default().fg(codex_cyan_tab()),
        ),
        Span::styled(" · ", Style::default().fg(codex_fg_very_dim())),
    ];
    if focused {
        spans.push(Span::styled(
            "↑/↓ select · Enter view · Esc back",
            Style::default().fg(codex_fg_dim()),
        ));
    } else {
        spans.push(Span::styled(
            "↓ to manage",
            Style::default().fg(codex_fg_dim()),
        ));
    }
    if let Some(activity) = &opts.activity {
        spans.push(Span::styled(
            " · ",
            Style::default().fg(codex_fg_very_dim()),
        ));
        // T-GLYPH-WAVE3: spinner fallback glyph defers to GlyphMode (`...` in
        // ASCII) when the activity stream doesn't supply a spinner frame.
        spans.push(Span::styled(
            activity
                .spinner
                .as_deref()
                .unwrap_or(glyph_set::current().spinner_placeholder)
                .to_string(),
            Style::default().fg(codex_orange_agent()),
        ));
        spans.push(Span::raw(" "));
        spans.push(Span::styled(
            "streaming",
            Style::default().fg(codex_fg_dim()),
        ));
        if let Some(tool) = &activity.active_tool {
            spans.push(Span::styled(
                " · ",
                Style::default().fg(codex_fg_very_dim()),
            ));
            // T-GLYPH-WAVE2: success bullet glyph defers to GlyphMode.
            spans.push(Span::styled(
                glyph_set::current().bullet_success,
                Style::default().fg(codex_orange_agent()),
            ));
            spans.push(Span::raw(" "));
            spans.push(Span::styled(
                tool.clone(),
                Style::default().fg(codex_fg_dim()),
            ));
        }
        if let Some(elapsed) = &activity.elapsed {
            spans.push(Span::styled(
                " · ",
                Style::default().fg(codex_fg_very_dim()),
            ));
            spans.push(Span::styled(
                elapsed.clone(),
                Style::default().fg(codex_fg_dim()),
            ));
        }
    }
    Line::from(spans)
}
