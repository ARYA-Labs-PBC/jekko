/// T-INLINE-CLUSTER #7 / T-PERMISSIONS-PLUMB: multi-line body for the
/// /permissions "modal" (rendered as a system notice until full modal
/// infrastructure lands). `/permissions` cycles through
/// [`PermissionMode::cycle`] on each invocation, so the footer hint advertises
/// the cycle behaviour.
fn permissions_modal_body(
    effective_label: &str,
    raw_mode: Option<&str>,
    running_agents: usize,
) -> String {
    let mut out = Vec::new();
    out.push("/permissions".to_string());
    out.push(format!("  mode (effective):    {effective_label}"));
    out.push(format!(
        "  mode (--permission-mode): {}",
        raw_mode.unwrap_or("(unset)")
    ));
    out.push(format!("  running agents:       {running_agents}"));
    out.push("  Run /permissions again to cycle modes (bypass → ask → read-only).".to_string());
    out.join("\n")
}

/// T-INLINE-CLUSTER #8 / T-PERMISSIONS-PLUMB: multi-line body for the
/// /sandbox modal (rendered as a system notice until full modal infrastructure
/// lands). Shows the resolved sandbox/approval/cwd plus deferred fields for
/// allow_net + allowed_paths until the runner exposes its resolved
/// [`SandboxPolicy`] back to the runtime.
fn sandbox_modal_body(
    sandbox_profile: Option<&str>,
    approval_mode: Option<&str>,
    ctx: &BootContext,
) -> String {
    let mut out = Vec::new();
    out.push("/sandbox".to_string());
    out.push(format!(
        "  --sandbox:             {}",
        sandbox_profile.unwrap_or("none")
    ));
    out.push(format!(
        "  --ask-for-approval:    {}",
        approval_mode.unwrap_or("none")
    ));
    out.push(format!("  cwd:                   {}", ctx.cwd_display));
    out.push(format!(
        "  branch:                {}",
        ctx.branch.as_deref().unwrap_or("(no git)")
    ));
    // Env scrubbing + allow-net + allowed paths are owned by the runner config
    // (built in chat_bridge_backend.rs from `SandboxPolicy`). The runner
    // doesn't expose its resolved state back through `InlineRuntimeOptions`
    // yet — surface the raw flags + cwd here and let T-SANDBOX-ENF wire the
    // rest. Until then, document the defaults so the operator knows what the
    // runner will pick up.
    out.push("  allow_net:             default deny".to_string());
    out.push("  allowed_paths:         default = cwd-only".to_string());
    out.push("  env scrubbing:         (resolved at runner construction)".to_string());
    out.join("\n")
}

/// Write a starter `CLAUDE.md` to `<cwd>/CLAUDE.md` when missing.
fn init_claude_md() -> (NoticeKind, String) {
    let cwd = current_dir_or_dot();
    let path = cwd.join("CLAUDE.md");
    if Path::new(&path).exists() {
        return (
            NoticeKind::Warn,
            format!("{} already exists — skipping", path.display()),
        );
    }
    let template = include_str!("../../../resources/claude_md_starter.md");
    match std::fs::write(&path, template) {
        Ok(_) => (NoticeKind::Info, format!("wrote {}", path.display())),
        Err(err) => (NoticeKind::Error, format!("init failed: {err}")),
    }
}

fn submenu_child_notice_for(shell_base: &str, item: &SlashSubcommand) -> String {
    format!(
        "Run `{shell_base} {}` from your shell; in-TUI sub-action execution is a follow-up.",
        item.id
    )
}
