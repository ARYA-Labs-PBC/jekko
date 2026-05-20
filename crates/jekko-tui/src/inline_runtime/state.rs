// ── Slash commands ───────────────────────────────────────────────────────────
//
// Catalog + action types live in `crate::slash::*` (COWBOY I1/I2). This
// runtime keeps only the popup state machine + visibility gating for the
// compatibility `/panels` marker.

#[derive(Default)]
struct SlashState {
    active: bool,
    query: String,
    cursor: usize,
    submenu: Option<SlashSubmenuState>,
    // WHY: store filtered ids as owned strings rather than catalog indices.
    // Indices would conflate builtins + user-defined entries and break when
    // the user-defined set changes; ids are stable across filters.
    filtered: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SlashSubmenuState {
    parent_id: String,
    cursor: usize,
}

impl SlashState {
    fn refresh_filter(&mut self, catalog: &SlashCatalog) {
        let q = self.query.to_lowercase();
        self.filtered.clear();
        for cmd in catalog.all() {
            if !slash_command_visible(cmd) {
                continue;
            }
            if q.is_empty() || cmd.id().starts_with(&q) {
                self.filtered.push(cmd.id().to_string());
            }
        }
        if self.cursor >= self.filtered.len() {
            self.cursor = self.filtered.len().saturating_sub(1);
        }
    }

    fn current_command<'a>(&self, catalog: &'a SlashCatalog) -> Option<&'a SlashCommand> {
        if self.submenu.is_some() {
            return None;
        }
        self.filtered
            .get(self.cursor)
            .and_then(|id| catalog.find(id))
    }

    fn open_submenu(&mut self, catalog: &SlashCatalog, parent_id: &str) -> bool {
        let Some(submenu) = catalog.submenu_for(parent_id) else {
            return false;
        };
        if submenu.items.is_empty() {
            return false;
        }
        self.query = parent_id.to_string();
        self.refresh_filter(catalog);
        self.submenu = Some(SlashSubmenuState {
            parent_id: parent_id.to_string(),
            cursor: 0,
        });
        true
    }

    fn pop_submenu(&mut self) -> bool {
        self.submenu.take().is_some()
    }

    fn selected_subcommand(
        &self,
        catalog: &SlashCatalog,
    ) -> Option<(&'static str, &'static str, &'static SlashSubcommand)> {
        let state = self.submenu.as_ref()?;
        let submenu = catalog.submenu_for(&state.parent_id)?;
        let item = submenu.item(state.cursor)?;
        Some((submenu.parent_id, submenu.shell_base, item))
    }

    fn selection_len(&self, catalog: &SlashCatalog) -> usize {
        if let Some(state) = &self.submenu {
            return match catalog.submenu_for(&state.parent_id) {
                Some(submenu) => submenu.items.len(),
                None => 0,
            };
        }
        self.filtered.len()
    }

    fn move_prev(&mut self) {
        if let Some(submenu) = self.submenu.as_mut() {
            if submenu.cursor > 0 {
                submenu.cursor -= 1;
            }
        } else if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    fn move_next(&mut self, catalog: &SlashCatalog) {
        let max = self.selection_len(catalog).saturating_sub(1);
        if let Some(submenu) = self.submenu.as_mut() {
            if submenu.cursor < max {
                submenu.cursor += 1;
            }
        } else if self.cursor < max {
            self.cursor += 1;
        }
    }
}

fn slash_command_visible(cmd: &SlashCommand) -> bool {
    if cmd.id() != "panels" {
        return true;
    }
    std::env::var(LEGACY_PANELS_ENV)
        .ok()
        .map(|v| matches!(v.trim(), "1" | "true" | "on"))
        .unwrap_or(false)
}

fn slash_command_visible_count(catalog: &SlashCatalog) -> usize {
    catalog
        .all()
        .filter(|cmd| slash_command_visible(cmd))
        .count()
}

// ── Mentions ─────────────────────────────────────────────────────────────────

#[derive(Default)]
struct MentionState {
    active: bool,
    trigger_byte_offset: usize,
    query: String,
    cursor: usize,
    filtered: Vec<PathBuf>,
}

impl MentionState {
    fn current_path(&self) -> Option<&PathBuf> {
        self.filtered.get(self.cursor)
    }
}

/// Find the byte offset of the active `@` trigger in `text`, if any.
///
/// The trigger is the last `@` that is either at the start of the text or
/// preceded by a non-alphanumeric character (so emails like `a@b.com` don't
/// fire). Everything after the `@` up to the cursor must be non-whitespace.
fn detect_mention_trigger(text: &str) -> Option<(usize, String)> {
    let at_pos = text.rfind('@')?;
    let prefix = &text[..at_pos];
    let preceded_by_alnum = prefix
        .chars()
        .next_back()
        .map(|c| c.is_alphanumeric() || c == '_')
        .unwrap_or(false);
    if preceded_by_alnum {
        return None;
    }
    let after = &text[at_pos + 1..];
    if after.chars().any(|c| c.is_whitespace()) {
        return None;
    }
    Some((at_pos, after.to_string()))
}

/// One owned diff body line carried by [`ChatEvent::Diff`]. Mirrors the
/// borrowed [`crate::transcript::inline_cards::DiffLine`] but stores `text` as
/// `String` so the payload can be moved across channel boundaries without a
/// lifetime parameter. The runtime materializes a borrowed view onto these
/// values at render time (`render_diff` / `render_diff_into`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiffBlockLine {
    /// Add / Del / Ctx.
    pub kind: crate::transcript::inline_cards::DiffLineKind,
    /// Original-file lineno (when known).
    pub old_lineno: Option<usize>,
    /// New-file lineno (when known).
    pub new_lineno: Option<usize>,
    /// Body text without the leading sigil.
    pub text: String,
}

/// Events streamed back from the chat backend to the inline runtime.
#[derive(Clone, Debug)]
pub enum ChatEvent {
    /// An incremental text delta from the assistant.
    AssistantDelta(String),
    /// A completed reasoning/thinking card.
    Reasoning { reasoning_id: String, text: String },
    /// Tool lifecycle / output event from the backend.
    Tool(ToolEvent),
    /// A parsed unified-diff block ready to render as a transcript card.
    /// Translators (e.g. [`crate::chat_bridge_backend::translate_action`])
    /// emit this once they have a complete unified diff in hand — usually
    /// after a `ToolEvent::Complete` whose accumulated stdout parsed as a diff.
    Diff {
        /// Display path for the card header (stripped of `a/`/`b/` prefixes).
        path: String,
        /// Body lines in original document order.
        hunks: Vec<DiffBlockLine>,
    },
    /// The assistant turn finished cleanly.
    TurnComplete,
    /// The assistant turn failed; render an error notice.
    TurnFailed(String),
    /// Informational system notice (e.g. mode change, login state).
    Notice(NoticeKind, String),
    /// Structured runtime lifecycle or service event forwarded from the bus
    /// or transport adapters.
    Runtime(crate::action::RuntimeEvent),
}

/// Trait for any backend the inline runtime can drive — chat bridge,
/// local echo, future protocol clients. Implementations are responsible for
/// spawning their own worker thread / task and forwarding events back through
/// the channel returned by `start_turn`.
pub trait ChatBackend: Send + 'static {
    /// Submit a user prompt. Returns a receiver of streaming events for this
    /// turn. The runtime will keep reading until it observes either
    /// `TurnComplete` or `TurnFailed`.
    fn start_turn(&mut self, prompt: String, cancel: CancellationToken) -> Receiver<ChatEvent>;
}

/// T-PERMISSIONS-PLUMB: Claude-compatible permission modes surfaced by
/// `/permissions` cycling and the chrome rail label. The variants mirror the
/// strings accepted by the `--permission-mode` CLI flag so
/// [`PermissionState::from_opts`] can round-trip the raw value back into a
/// typed mode.
///
/// `BypassPermissions` is the default — this preserves the runtime's
/// pre-T-PERMISSIONS-PLUMB behaviour (the chrome rail hardcoded
/// "bypass permissions" and there was no MCP gate at all). Stricter modes
/// require explicit opt-in via the CLI flag or `/permissions` cycling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PermissionMode {
    /// No gating — every tool / write call proceeds. Default (backward compat).
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

    /// Advance to the next mode in cycle order. Drives `/permissions`'s
    /// repeated-invocation cycling behaviour until full modal infra lands.
    pub fn cycle(&self) -> Self {
        match self {
            Self::BypassPermissions => Self::AskBeforeWrite,
            Self::AskBeforeWrite => Self::ReadOnly,
            Self::ReadOnly => Self::BypassPermissions,
        }
    }
}

/// T-PERMISSIONS-PLUMB: snapshot of the permission/sandbox/approval state
/// owned by the runtime. Mutated by `/permissions` cycling, displayed by
/// `/sandbox`, and surfaced to the chrome via `permission_state.mode.label()`.
#[derive(Debug, Clone, Default)]
pub struct PermissionState {
    pub mode: PermissionMode,
    pub sandbox_profile: Option<String>,
    pub approval_mode: Option<String>,
}

impl PermissionState {
    /// Derive a fresh state from the CLI/loader-resolved [`InlineRuntimeOptions`].
    ///
    /// Accepts both the Claude-shorthand (`bypass`, `ask`, `read-only`) and
    /// the fully-spelled (`bypass-permissions`, `ask-before-write`, `readonly`)
    /// forms. Any unrecognised value fails closed so the runtime does not
    /// boot with an unintended permission state.
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

/// Inline runtime options. Most callers can use the defaults.
pub struct InlineRuntimeOptions {
    pub boot_visible: bool,
    pub initial_notice: Option<String>,
    pub no_alt_screen: bool,
    /// Initial Jnoccio status snapshot when the runtime starts.
    pub jnoccio_boot_status: JnoccioBootStatus,
    /// Live boot-status receiver owned by the runtime.
    pub jnoccio_boot_rx: Option<Receiver<JnoccioBootStatus>>,
    /// Resolved UI configuration (TOML + env + CLI overlay). When `None`,
    /// downstream consumers (e.g. [`crate::anim`]) fall back to env/file
    /// auto-detect. Set by the CLI boot path (`jekko-cli`) so the runtime
    /// receives a single source of truth instead of re-reading the filesystem.
    pub ui_config: Option<jekko_core::config::ui::UiConfig>,
    /// T-INLINE-CLUSTER #11: sandbox profile selector (raw CLI value).
    /// Threaded from `jekko chat --sandbox <POLICY>` so the agent rail / /sandbox
    /// modal can display the current policy. The runner-level `SandboxPolicy` is
    /// constructed elsewhere (T-SANDBOX-ENF); the runtime only displays this.
    pub sandbox_profile: Option<String>,
    /// T-INLINE-CLUSTER #11: approval policy selector (raw CLI value).
    /// Threaded from `jekko chat --ask-for-approval <POLICY>`.
    pub approval_mode: Option<String>,
    /// T-INLINE-CLUSTER #11: Claude-compatible permission mode (raw CLI value).
    /// Threaded from `jekko chat --permission-mode <MODE>`. Used by the
    /// permission banner + /permissions slash display.
    pub permission_mode: Option<String>,
    /// T-INLINE-CLUSTER #1: profile label sourced from `--profile`. Surfaced in
    /// the footer chrome alongside cwd/branch.
    pub profile: Option<String>,
    /// T-INLINE-WAVE3 #3: number of detached background terminals currently
    /// alive. Threaded into [`render_working_strip`] so the status row can show
    /// `N background terminal(s) running · /ps to view · /stop to close`.
    /// Defaults to `0` because the background-terminal manager doesn't exist
    /// yet (T-BG-COUNT-MANAGER follow-up); flip this field once the runtime
    /// has a live source of truth.
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

#[derive(Debug)]
struct JnoccioBootRuntime {
    status: JnoccioBootStatus,
}

impl JnoccioBootRuntime {
    fn new(status: JnoccioBootStatus) -> Self {
        Self { status }
    }

    fn drain_updates(&mut self, rx: &mut Option<Receiver<JnoccioBootStatus>>) -> bool {
        let Some(rx) = rx.as_ref() else {
            return false;
        };
        let mut dirty = false;
        while let Ok(next) = rx.try_recv() {
            if self.status != next {
                self.status = next;
                dirty = true;
            }
        }
        dirty
    }

    fn footer_label(&self) -> Option<String> {
        match self.status {
            JnoccioBootStatus::Idle => None,
            _ => Some(format!("jnoccio {}", self.status.label())),
        }
    }

    fn status_lines(&self) -> Vec<String> {
        match self.detail() {
            Some(detail) => detail.lines().map(|line| line.to_string()).collect(),
            None => vec![format!("jnoccio: {}", self.status.label())],
        }
    }

    fn detail(&self) -> Option<String> {
        self.status.detail()
    }
}
