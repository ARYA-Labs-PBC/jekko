fn estimate_input_tokens(text: &str) -> u64 {
    ((text.len() as u64).saturating_add(3)) / 4
}

fn update_main_agent_turn(
    panel: &mut AgentPanelState,
    status: AgentStatus,
    summary: &str,
    input_tokens: Option<u64>,
    label_hint: Option<&str>,
) {
    let now = Instant::now();
    let mut current = match panel
        .agents
        .iter()
        .find(|agent| agent.id.as_str() == "main")
        .cloned()
    {
        Some(agent) => agent,
        None => AgentRun::new_main("main", summary),
    };
    current.id = crate::agents::AgentId::new("main");
    current.name = "main".to_string();
    current.kind = AgentKind::Main;
    current.status = status;
    current.summary = if summary.trim().is_empty() {
        match label_hint {
            Some(label) => label.to_string(),
            None => "session active".to_string(),
        }
    } else {
        summary.to_string()
    };
    current.last_active_at = now;
    if let Some(tokens) = input_tokens {
        current.tokens.add_input(tokens);
    }
    if status == AgentStatus::Running {
        current.tokens.add_output(0);
    }
    panel.upsert(current);
}

fn update_main_agent_output(panel: &mut AgentPanelState, delta: &str) {
    let mut current = match panel
        .agents
        .iter()
        .find(|agent| agent.id.as_str() == "main")
        .cloned()
    {
        Some(agent) => agent,
        None => AgentRun::new_main("main", "streaming"),
    };
    current.id = crate::agents::AgentId::new("main");
    current.name = "main".to_string();
    current.kind = AgentKind::Main;
    current.status = AgentStatus::Running;
    current.summary = "streaming".to_string();
    current.last_active_at = Instant::now();
    current.tokens.add_output(delta.len() as u64);
    panel.upsert(current);
}
