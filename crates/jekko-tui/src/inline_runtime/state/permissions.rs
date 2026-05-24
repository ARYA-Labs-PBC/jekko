/// Claude-compatible permission modes surfaced by `/permissions` cycling and
/// the chrome rail label.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PermissionMode {
    /// No gating; every tool / write call proceeds. Default for back-compat.
    #[default]
    BypassPermissions,
    /// Writes + run_command prompt the operator on a TTY; deny on non-TTY.
    AskBeforeWrite,
    /// Hard-deny writes + run_command at the MCP boundary.
    ReadOnly,
}

impl PermissionMode {
    /// Display label rendered in the chrome rail + permission notices.
    pub fn label(&self) -> &'static str {
        match self {
            Self::BypassPermissions => "bypass permissions",
            Self::AskBeforeWrite => "ask before write",
            Self::ReadOnly => "read-only",
        }
    }

    /// Advance to the next mode in cycle order.
    pub fn cycle(&self) -> Self {
        match self {
            Self::BypassPermissions => Self::AskBeforeWrite,
            Self::AskBeforeWrite => Self::ReadOnly,
            Self::ReadOnly => Self::BypassPermissions,
        }
    }
}

/// Snapshot of the permission/sandbox/approval state owned by the runtime.
#[derive(Debug, Clone, Default)]
pub struct PermissionState {
    pub mode: PermissionMode,
    pub sandbox_profile: Option<String>,
    pub approval_mode: Option<String>,
}

impl PermissionState {
    /// Derive a fresh state from the CLI/loader-resolved [`InlineRuntimeOptions`].
    pub fn from_opts(opts: &InlineRuntimeOptions) -> anyhow::Result<Self> {
        let mode = match opts.permission_mode.as_deref() {
            Some(raw) => match raw.trim().to_ascii_lowercase().as_str() {
                "bypass" | "bypass-permissions" | "bypasspermissions" => {
                    PermissionMode::BypassPermissions
                }
                "ask" | "ask-before-write" | "askbeforewrite" => PermissionMode::AskBeforeWrite,
                "read-only" | "readonly" => PermissionMode::ReadOnly,
                other => {
                    return Err(anyhow::anyhow!("invalid permission mode: {other}"));
                }
            },
            None => PermissionMode::default(),
        };
        Ok(Self {
            mode,
            sandbox_profile: opts.sandbox_profile.clone(),
            approval_mode: opts.approval_mode.clone(),
        })
    }
}
