/// Pick the focus-aware hint string for the permission banner.
fn permission_hint_for(focus: FocusArea) -> &'static str {
    match focus {
        FocusArea::Composer => HINT_CHAT_FOCUS,
        FocusArea::Agents => HINT_AGENT_PANEL_FOCUS,
    }
}

/// Build a [`FooterInfo`] snapshot for the current frame. Reads the model /
/// effort from env, the cwd + branch from [`BootContext`], and accepts the
/// profile from the caller (T-INLINE-CLUSTER #1 — sourced from
/// [`InlineRuntimeOptions::profile`], which the CLI populates from
/// `--profile`).
fn footer_info_for(
    ctx: &BootContext,
    profile: Option<&str>,
    jnoccio_boot_label: Option<&str>,
) -> FooterInfo {
    let model = env_or("JEKKO_MODEL", "(default)");
    let effort = match std::env::var("JEKKO_EFFORT") {
        Ok(effort) => effort,
        Err(_) => String::new(),
    };
    FooterInfo {
        model,
        effort,
        cwd: ctx.cwd_display.clone(),
        branch: ctx.branch.clone(),
        profile: profile.map(|s| s.to_string()),
        jnoccio: jnoccio_boot_label.map(|s| s.to_string()),
    }
}

fn render_popups(
    buf: &mut Buffer,
    area: Rect,
    composer: &ComposerState,
    index_len: usize,
    catalog: &SlashCatalog,
) {
    if area.height == 0 || area.width == 0 {
        return;
    }
    if composer.slash.active {
        render_slash_popup(buf, area, &composer.slash, catalog);
    } else if composer.mention.active {
        render_mention_popup(buf, area, &composer.mention, index_len);
    }
}

fn render_agent_panel_lines(buf: &mut Buffer, area: Rect, lines: &[Line<'static>]) {
    if area.height == 0 || area.width == 0 {
        return;
    }
    let height = area.height.min(lines.len() as u16);
    let start = lines.len().saturating_sub(height as usize);
    let start_y = area.y + area.height.saturating_sub(height);
    for (i, line) in lines[start..].iter().enumerate() {
        let row_area = Rect::new(area.x, start_y + i as u16, area.width, 1);
        Paragraph::new(line.clone()).render(row_area, buf);
    }
}

fn render_transcript_viewport(
    buf: &mut Buffer,
    area: Rect,
    transcript: &Transcript,
    scroll: &ScrollState,
    in_flight: Option<&InFlight>,
    motion_enabled: bool,
) {
    if area.height == 0 || area.width == 0 {
        return;
    }

    let mut rows: Vec<Line<'static>> =
        transcript.visible_rows(area.width, area.height, scroll.offset_from_bottom);
    if scroll.sticky_bottom() {
        if let Some(state) = in_flight {
            if !state.buffer.is_empty() {
                rows.extend(render_assistant(&state.buffer));
            }
            // T-SEMANTIC-TRANSCRIPT-A: stream every in-flight tool card so
            // multi-tool turns don't visually collapse to a single chip.
            // Insertion order is preserved by `IndexMap` so the order mirrors
            // the order tools started.
            for tool in state.active_tools.values() {
                rows.extend(render_active_tool_card(tool, motion_enabled));
            }
        }
    }

    let max_rows = area.height as usize;
    if rows.len() > max_rows {
        let keep_from = rows.len() - max_rows;
        rows.drain(0..keep_from);
    }
    let start_y = area
        .y
        .saturating_add(area.height.saturating_sub(rows.len() as u16));
    for (i, line) in rows.into_iter().enumerate() {
        let row_area = Rect::new(area.x, start_y + i as u16, area.width, 1);
        Paragraph::new(line).render(row_area, buf);
    }
}

fn finalize_turn(
    terminal: &mut Tty,
    mode: RuntimeMode,
    transcript: &mut Transcript,
    scroll: &mut ScrollState,
    state: &InFlight,
    ok: bool,
) -> Result<()> {
    // T-SEMANTIC-TRANSCRIPT-A: drain any tools that never received a
    // Complete/Fail before the turn finalized. Emit them as Cancelled
    // (turn ok) or Failure (turn failed) so the user has a per-tool record,
    // and register their OutputBuffers on the sidecar so the pager can still
    // surface their captured output.
    for tool in state.active_tools.values() {
        let lines = render_completed_tool_card(tool, ok);
        let buffer = Arc::new(tool.build_output_buffer());
        transcript.record_tool_buffer(tool._id.clone(), buffer);
        emit_lines(
            terminal,
            mode,
            transcript,
            scroll,
            TranscriptEvent::tool(
                tool._id.clone(),
                tool.name.clone(),
                tool.input.clone(),
                lines,
            ),
        )?;
    }
    let text = if state.buffer.trim().is_empty() {
        if ok {
            "(empty response)".to_string()
        } else {
            "(no output before failure)".to_string()
        }
    } else {
        state.buffer.clone()
    };
    let lines = render_assistant(&text);
    emit_lines(
        terminal,
        mode,
        transcript,
        scroll,
        TranscriptEvent::assistant(text, lines),
    )?;
    Ok(())
}

fn render_active_tool_card(tool: &ActiveToolChip, motion_enabled: bool) -> Vec<Line<'static>> {
    let output: Vec<String> = tool.output.lines().map(|line| line.to_string()).collect();
    let call = ToolCall {
        verb: &tool.name,
        args: tool.input.as_deref().unwrap_or(""),
        status: ActionStatus::Running,
        output: &output,
        max_output_lines: 8,
    };
    render_tool_call_live(&call, tool.started_at, Instant::now(), motion_enabled)
}

/// Materialize a [`ChatEvent::Diff`] payload into a transcript-ready vector of
/// `Line<'static>` by delegating to the existing
/// [`crate::transcript::inline_cards::render_diff`] helper.
///
/// `render_diff` borrows from `DiffLine<'_>`, so we lift each owned
/// [`DiffBlockLine`] into a temporary borrowed view that lives only for the
/// duration of the call. This keeps the diff renderer untouched (T1-V5 ships
/// `render_diff` / `render_diff_into` purely for tests; T1-V5b only adds the
/// live dispatch arm calling them).
fn render_diff_lines_from_payload(path: &str, hunks: &[DiffBlockLine]) -> Vec<Line<'static>> {
    let borrowed: Vec<DiffLine<'_>> = hunks
        .iter()
        .map(|line| DiffLine {
            kind: line.kind,
            old_lineno: line.old_lineno,
            new_lineno: line.new_lineno,
            text: line.text.as_str(),
        })
        .collect();
    render_diff(path, &borrowed)
}

fn render_completed_tool_card(tool: &ActiveToolChip, ok: bool) -> Vec<Line<'static>> {
    // Strip blank lines from rendered output so the visible-tail window
    // doesn't collapse to whitespace + a `+N lines` marker (which renders as
    // a giant gap between the tool header and the collapse marker). The raw
    // `tool.output` buffer is preserved for Ctrl+O pager + `/copy` /
    // `/export` paths; only the in-card render is filtered.
    let output: Vec<String> = tool
        .output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.to_string())
        .collect();
    let status = if ok {
        ActionStatus::Success
    } else {
        match tool.status {
            ToolChipStatus::Failure => ActionStatus::Failure,
            _ => ActionStatus::Cancelled,
        }
    };
    let call = ToolCall {
        verb: &tool.name,
        args: tool.input.as_deref().unwrap_or(""),
        status,
        output: &output,
        max_output_lines: 12,
    };
    render_tool_call(&call)
}

fn emit_lines<E>(
    terminal: &mut Tty,
    mode: RuntimeMode,
    transcript: &mut Transcript,
    scroll: &mut ScrollState,
    event: E,
) -> Result<()>
where
    E: crate::transcript::IntoTranscriptEvent,
{
    let event = event.into_transcript_event();
    let lines = event.lines().to_vec();
    match mode {
        RuntimeMode::NoAltScreen => push_to_scrollback(terminal, &lines),
        RuntimeMode::Fullscreen => {
            let width = terminal.size()?.width;
            let was_sticky = scroll.sticky_bottom();
            transcript.push_event(event);
            if was_sticky {
                scroll.to_bottom();
            } else {
                scroll.clamp(transcript, width);
            }
            Ok(())
        }
    }
}

fn push_to_scrollback(terminal: &mut Tty, lines: &[Line<'static>]) -> Result<()> {
    if lines.is_empty() {
        return Ok(());
    }
    let height = lines.len() as u16;
    let cloned: Vec<Line<'static>> = lines.to_vec();
    terminal.insert_before(height, move |buf: &mut Buffer| {
        for (i, line) in cloned.iter().enumerate() {
            let area = Rect::new(0, i as u16, buf.area.width, 1);
            Paragraph::new(line.clone()).render(area, buf);
        }
    })?;
    Ok(())
}
