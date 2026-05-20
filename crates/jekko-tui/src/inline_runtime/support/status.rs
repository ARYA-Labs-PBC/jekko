/// Multi-line environment snapshot for `/doctor` — version, cwd, branch, HOME,
/// API-key presence flags (never the values themselves).
fn render_doctor_snapshot(ctx: &BootContext) -> String {
    let mut out = Vec::new();
    out.push(format!("jekko v{}", ctx.version));
    out.push(format!("cwd:    {}", ctx.cwd_display));
    out.push(format!(
        "branch: {}",
        ctx.branch.as_deref().unwrap_or("(not a git repo)")
    ));
    out.push(format!(
        "HOME:   {}",
        match std::env::var("HOME") {
            Ok(value) => value,
            Err(_) => "(unset)".to_string(),
        }
    ));
    out.push(format!(
        "SHELL:  {}",
        match std::env::var("SHELL") {
            Ok(value) => value,
            Err(_) => "(unset)".to_string(),
        }
    ));
    out.push(format!("TERM:   {}", env_or("TERM", "(unset)")));
    out.push(format!(
        "ANTHROPIC_API_KEY: {}",
        if std::env::var("ANTHROPIC_API_KEY").is_ok() {
            "set"
        } else {
            "missing"
        }
    ));
    out.push(format!(
        "OPENAI_API_KEY:    {}",
        if std::env::var("OPENAI_API_KEY").is_ok() {
            "set"
        } else {
            "missing"
        }
    ));
    out.push(format!(
        "JEKKO_GATEWAY_URL: {}",
        env_or("JEKKO_GATEWAY_URL", "(unset)")
    ));
    out.push(format!(
        "JEKKO_MODEL:       {}",
        env_or("JEKKO_MODEL", "(unset)")
    ));
    out.push(format!(
        "jankurai on PATH:  {}",
        if which_in_path("jankurai").is_some() {
            "yes"
        } else {
            "no"
        }
    ));
    out.push(format!(
        "just on PATH:      {}",
        if which_in_path("just").is_some() {
            "yes"
        } else {
            "no"
        }
    ));
    out.join("\n")
}

/// Inline `/status` snapshot — current turn state + transcript metrics.
fn render_status_snapshot(
    ctx: &BootContext,
    in_flight: bool,
    block_count: usize,
    jnoccio_boot: &JnoccioBootRuntime,
) -> String {
    let backend = env_or("JEKKO_BACKEND", "auto");
    let model = env_or("JEKKO_MODEL", "(default)");
    let mut out = Vec::new();
    out.push(format!("model:        {model}"));
    out.push(format!("backend:      {backend}"));
    out.push(format!(
        "turn:         {}",
        if in_flight { "in flight" } else { "idle" }
    ));
    out.push(format!("transcript:   {block_count} block(s)"));
    out.push(format!("cwd:          {}", ctx.cwd_display));
    out.push(format!(
        "branch:       {}",
        ctx.branch.as_deref().unwrap_or("(no git)")
    ));
    out.push("jnoccio:".to_string());
    out.extend(
        jnoccio_boot
            .status_lines()
            .into_iter()
            .map(|line| format!("  {line}")),
    );
    out.push("jankurai:".to_string());
    match render_jankurai_status_lines() {
        Ok(lines) => out.extend(lines.into_iter().map(|line| format!("  {line}"))),
        Err((_, body)) => out.push(format!("  {body}")),
    }
    out.push("zyal:".to_string());
    out.extend(
        render_zyal_status_lines()
            .into_iter()
            .map(|line| format!("  {line}")),
    );
    out.join("\n")
}

fn render_panels_snapshot(
    ctx: &BootContext,
    in_flight: Option<&InFlight>,
    block_count: usize,
    jnoccio_boot: &JnoccioBootRuntime,
    agent_panel: &AgentPanelState,
    toasts: &ToastStack,
    permission_mode_label: &str,
    motion_enabled: bool,
    width: u16,
) -> String {
    let mut out = Vec::new();
    out.push("status".to_string());
    out.extend(
        render_status_snapshot(ctx, in_flight.is_some(), block_count, jnoccio_boot)
            .lines()
            .map(|line| format!("  {line}")),
    );
    out.push(String::new());
    out.push("agents:".to_string());
    let activity = in_flight.map(|state| {
        let active_tool = state.latest_tool().map(|tool| {
            let mut text = tool.name.clone();
            if let Some(input) = &tool.input {
                if !input.trim().is_empty() {
                    text = format!("{text}({input})");
                }
            }
            truncate_to_width(&text, width.saturating_sub(20) as usize)
        });
        crate::agents::panel::PanelStreamStatus {
            spinner: Some(state.spinner_glyph(motion_enabled).to_string()),
            active_tool,
            elapsed: Some(elapsed_label(state.started_at.elapsed())),
        }
    });
    let panel_opts = PanelRenderOptions {
        permission_mode_label: Cow::Owned(permission_mode_label.to_string()),
        max_agents: 8,
        max_visible_rows: 8,
        width,
        activity,
        compact: true,
        motion_enabled,
    };
    let lines = render_agent_panel(agent_panel, Instant::now(), &panel_opts);
    out.extend(
        lines_to_strings(&lines)
            .into_iter()
            .map(|line| format!("  {line}")),
    );
    out.push(String::new());
    out.push("recent notices:".to_string());
    let recent = toasts.recent(3);
    if recent.is_empty() {
        out.push("  (none)".to_string());
    } else {
        for toast in recent {
            out.push(format!("  [{}] {}", toast.kind.label(), toast.message));
        }
    }
    out.join("\n")
}

fn render_zyal_status_lines() -> Vec<String> {
    let cwd = current_dir_or_dot();
    let agent_root = cwd.join("agent").join("zyal");
    let tracked = std::fs::read_dir(&agent_root)
        .ok()
        .into_iter()
        .flat_map(|read_dir| read_dir.filter_map(|entry| entry.ok()))
        .filter(|entry| entry.path().extension().and_then(|ext| ext.to_str()) == Some("zyal"))
        .count();
    let spec_present = cwd.join("docs").join("ZYAL").join("SPEC.md").exists();
    let mut out = Vec::new();
    if tracked == 0 {
        out.push("agent/zyal: no tracked .zyal runbooks".to_string());
    } else {
        out.push(format!("agent/zyal: {tracked} tracked .zyal runbook(s)"));
    }
    out.push(format!(
        "docs/ZYAL/SPEC.md: {}",
        if spec_present { "present" } else { "missing" }
    ));
    out
}

fn lines_to_strings(lines: &[Line<'static>]) -> Vec<String> {
    lines
        .iter()
        .map(|line| {
            line.spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect()
        })
        .collect()
}
