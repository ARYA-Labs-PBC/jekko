/// Inline runtime options. Most callers can use the defaults.
pub struct InlineRuntimeOptions {
    pub boot_visible: bool,
    pub initial_notice: Option<String>,
    pub no_alt_screen: bool,
    /// Initial Jnoccio status snapshot when the runtime starts.
    pub jnoccio_boot_status: JnoccioBootStatus,
    /// Live boot-status receiver owned by the runtime.
    pub jnoccio_boot_rx: Option<Receiver<JnoccioBootStatus>>,
    /// Resolved UI configuration (TOML + env + CLI overlay).
    pub ui_config: Option<jekko_core::config::ui::UiConfig>,
    /// Sandbox profile selector (raw CLI value).
    pub sandbox_profile: Option<String>,
    /// Approval policy selector (raw CLI value).
    pub approval_mode: Option<String>,
    /// Claude-compatible permission mode (raw CLI value).
    pub permission_mode: Option<String>,
    /// Profile label sourced from `--profile`.
    pub profile: Option<String>,
    /// Number of detached background terminals currently alive.
    pub background_count: u32,
}

impl Default for InlineRuntimeOptions {
    fn default() -> Self {
        Self {
            boot_visible: true,
            initial_notice: None,
            no_alt_screen: false,
            jnoccio_boot_status: JnoccioBootStatus::Idle,
            jnoccio_boot_rx: None,
            ui_config: None,
            sandbox_profile: None,
            approval_mode: None,
            permission_mode: None,
            profile: None,
            background_count: 0,
        }
    }
}
