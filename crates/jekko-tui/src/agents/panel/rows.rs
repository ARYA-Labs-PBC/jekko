fn render_bullet_row(
    agent: &AgentRun,
    now: Instant,
    selected: bool,
    width: u16,
    compact: bool,
    motion_enabled: bool,
) -> Line<'static> {
    let bullet = status_glyph(
        agent.status,
        now.duration_since(agent.started_at),
        motion_enabled,
    );
    let name_style = if selected {
        Style::default()
            .fg(codex_fg_strong())
            .bg(codex_bg_overlay())
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(codex_fg())
    };
    let runtime = elapsed_label(agent.runtime(now));
    let status_color = status_color(agent.status);
    let suffix_mode = match width {
        0..=43 => 0,
        44..=57 => 1,
        58..=75 => 2,
        _ => 3,
    };
    let status_text = agent.status.label();
    // WHY: T1-V8 — show both directions when both sides have non-zero token
    // counts. The shared helper drops a side that's still 0, and returns
    // empty when both are 0, so we only have to gate on `is_empty()` once.
    // T-COMPONENT-PLUMBING: when compact, skip the token columns entirely and
    // truncate runtime to the leading unit (`5m` rather than `5m 42s`) so the
    // narrow renderer keeps the agent name visible.
    let tokens_text = format_tokens_with_direction(&agent.tokens);
    let suffix = if compact {
        let short_runtime = short_runtime_label(&runtime);
        format!("{status_text} · {short_runtime}")
    } else {
        match suffix_mode {
            0 => String::new(),
            1 => runtime.clone(),
            2 => format!("{status_text} · {runtime}"),
            _ => {
                if !tokens_text.is_empty() {
                    format!("{status_text} · {runtime} · {tokens_text}")
                } else {
                    format!("{status_text} · {runtime}")
                }
            }
        }
    };
    let mut name = agent.name.clone();
    let suffix_width = suffix.chars().count().saturating_add(3);
    let name_budget = width as usize;
    let available_name = name_budget.saturating_sub(suffix_width).max(8);
    name = truncate_to_width(&name, available_name);
    let mut spans = vec![
        bullet,
        Span::raw(" "),
        Span::styled(
            name,
            if selected {
                name_style.add_modifier(Modifier::BOLD)
            } else {
                name_style
            },
        ),
    ];
    if !suffix.is_empty() {
        spans.push(Span::raw(" "));
        spans.push(Span::styled(
            suffix,
            Style::default().fg(status_color).add_modifier(if selected {
                Modifier::BOLD
            } else {
                Modifier::empty()
            }),
        ));
    } else if agent.locality == AgentLocality::Remote {
        spans.push(Span::styled(
            " remote",
            Style::default().fg(codex_fg_very_dim()),
        ));
    }
    Line::from(spans)
}

fn render_summary_row(agent: &AgentRun, width: u16) -> Line<'static> {
    let summary = truncate_to_width(&agent.summary, width.saturating_sub(4) as usize);
    Line::from(vec![
        Span::raw("    "),
        Span::styled(summary, Style::default().fg(codex_fg_dim())),
    ])
}

/// Compact-mode helper: keep only the leading unit from an `elapsed_label`
/// (`"5m 42s"` → `"5m"`, `"42s"` → `"42s"`, `"1h 5m"` → `"1h"`, `"1d 1h"` →
/// `"1d"`). Used by the agent rail when compact rendering is enabled so the
/// status row stays short enough to keep the agent name visible.
fn short_runtime_label(runtime: &str) -> String {
    match runtime.split_whitespace().next() {
        Some(value) => value.to_string(),
        None => String::new(),
    }
}

fn status_glyph(
    status: AgentStatus,
    elapsed: std::time::Duration,
    motion_enabled: bool,
) -> Span<'static> {
    // T-GLYPH-WAVE2: static bullets route through GlyphMode so `JEKKO_ASCII=1`
    // swaps `●`/`○` for `*`/`o`. The Running variant keeps `pulse_glyph` since
    // anim.rs is excluded from this migration wave.
    let g = glyph_set::current();
    match status {
        AgentStatus::Running => Span::styled(
            pulse_glyph_with_motion(elapsed, motion_enabled),
            Style::default().fg(codex_orange_agent()),
        ),
        AgentStatus::Idle => Span::styled(g.bullet_pending, Style::default().fg(codex_fg_dim())),
        AgentStatus::Queued => {
            Span::styled(g.bullet_pending, Style::default().fg(codex_fg_very_dim()))
        }
        AgentStatus::Waiting => Span::styled(g.bullet_pending, Style::default().fg(codex_fg_dim())),
        AgentStatus::Done => Span::styled(g.bullet_success, Style::default().fg(codex_green_ok())),
        AgentStatus::Failed => {
            Span::styled(g.bullet_success, Style::default().fg(codex_salmon_fail()))
        }
        AgentStatus::Cancelled => {
            Span::styled(g.bullet_success, Style::default().fg(codex_fg_very_dim()))
        }
    }
}

fn status_color(status: AgentStatus) -> ratatui::style::Color {
    match status {
        AgentStatus::Running => codex_orange_agent(),
        AgentStatus::Idle | AgentStatus::Queued | AgentStatus::Waiting => codex_fg_dim(),
        AgentStatus::Done => codex_green_ok(),
        AgentStatus::Failed => codex_salmon_fail(),
        AgentStatus::Cancelled => codex_fg_very_dim(),
    }
}
