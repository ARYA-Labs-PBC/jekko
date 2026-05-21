pub struct PanelRenderOptions {
    /// Label printed next to the `▸▸` banner glyph at the top of the rail.
    ///
    /// T-CLI-OVERRIDES-PROVENANCE widens this from a bare `&'static str` to
    /// [`Cow<'static, str>`] so the inline runtime can pass a
    /// `Cow::Owned(format!(...))` for the dynamic permission-mode label fed
    /// from `InlineRuntimeOptions::permission_mode` (the static fallback
    /// `Cow::Borrowed("bypass permissions")` remains zero-cost).
    pub permission_mode_label: Cow<'static, str>,
    pub max_agents: usize,
    pub max_visible_rows: usize,
    pub width: u16,
    pub activity: Option<PanelStreamStatus>,
    /// T-COMPONENT-PLUMBING: when `true`, drop the token-count column and
    /// emit short metadata (`"running · 5m"`) so narrow widths still keep the
    /// agent name + status bullet visible. Default = `false` preserves the
    /// historical render path for every existing call site.
    pub compact: bool,
    /// Resolved reduced-motion flag for animated status bullets.
    pub motion_enabled: bool,
}

#[derive(Clone, Debug, Default)]
pub struct PanelStreamStatus {
    pub spinner: Option<String>,
    pub active_tool: Option<String>,
    pub elapsed: Option<String>,
}

impl Default for PanelRenderOptions {
    fn default() -> Self {
        Self {
            permission_mode_label: Cow::Borrowed("bypass permissions"),
            max_agents: 8,
            max_visible_rows: 8,
            width: 80,
            activity: None,
            compact: false,
            motion_enabled: true,
        }
    }
}
