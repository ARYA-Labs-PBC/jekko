use std::time::Duration;

use crossterm::event::{KeyEvent, MouseEvent};
use jekko_core::keybind::Chord;
use jekko_core::session::SessionId;
use jekko_core::theme::ThemeMode;

use crate::feature_plugins::ShellTab;

/// Live status of the Jnoccio Fusion server, forwarded from the boot thread.
///
/// Mirrors `JnoccioBootStatus` from `jnoccio-boot.ts`. Kept here (not in
/// `jekko-jnoccio-boot`) so the TUI crate can pattern-match without a dep
/// cycle. The boot crate's `BootStatus` is mapped 1-to-1 by the bridge in
/// `lib.rs`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum JnoccioBootStatus {
    #[default]
    Idle,
    Checking,
    Starting,
    /// Server is reachable. `model_count` is the number of routable models.
    Ready {
        enabled_models: u32,
        total_models: u32,
    },
    Unavailable,
    Failed,
}

/// Top-level route discriminator for the TUI app.
///
/// Initial route for the TUI app.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum Route {
    #[default]
    Home,
    Shell,
    Session {
        session_id: SessionId,
    },
}

/// Events emitted by the runtime layer (jekko-runtime) and forwarded into the
/// TUI's action stream. Intentionally minimal — expanded as more runtime
/// event kinds get plumbed through the bus.
#[derive(Clone, Debug)]
pub enum RuntimeEvent {
    SessionStarted {
        session_id: SessionId,
    },
    SessionEnded {
        session_id: SessionId,
    },
    DaemonStatus {
        online: bool,
    },
    Tick,
    /// Streaming assistant text delta. The first delta after a `PromptSubmit`
    /// opens a fresh `AssistantCard`; subsequent deltas append to that card.
    AssistantTextDelta {
        text: String,
    },
    /// Streaming assistant response finished cleanly.
    AssistantCompleted,
    /// Streaming assistant response failed; carries a human-readable reason.
    AssistantFailed {
        error: String,
    },
    /// Reasoning stream started (model is "thinking" before responding).
    ReasoningStarted {
        reasoning_id: String,
    },
    /// Incremental reasoning delta — append to the live `ReasoningCard`.
    ReasoningDelta {
        text: String,
    },
    /// Reasoning stream ended — finalize the `ReasoningCard`.
    ReasoningEnded {
        reasoning_id: String,
        text: String,
    },
}

/// A single actionable finding from a jankurai audit.
#[derive(Clone, Debug)]
pub struct AuditFinding {
    /// Severity: "critical", "high", "medium", "low".
    pub severity: String,
    /// Human-readable problem statement.
    pub problem: String,
    /// Agent-targeted fix suggestion from the auditor.
    pub agent_fix: String,
    /// File path where the finding was detected.
    pub path: String,
    /// Jankurai rule identifier (e.g. "HLT-001-DEAD-MARKER").
    pub rule_id: String,
    /// Optional line number.
    pub line: Option<u64>,
}

/// Parsed summary of a jankurai audit run, extracted from `agent/repo-score.json`.
#[derive(Clone, Debug)]
pub struct AuditSummary {
    /// Final score after caps (0-100).
    pub score: u64,
    /// Raw score before caps.
    pub raw_score: u64,
    /// Number of score-capping rules that fired.
    pub caps_count: usize,
    /// Names of the caps that fired.
    pub caps: Vec<String>,
    /// Number of hard findings.
    pub hard_findings: u64,
    /// Number of soft findings.
    pub soft_findings: u64,
    /// Conformance blockers.
    pub blockers: Vec<String>,
    /// The most impactful findings (with `agent_fix` hints).
    pub actionable_findings: Vec<AuditFinding>,
}

/// The action enum dispatched by the TUI loop.
///
/// Components must not mutate app state directly; they emit `Action`s.
#[derive(Clone, Debug)]
pub enum Action {
    Quit,
    Navigate(Route),
    ToggleTheme,
    Key(KeyEvent),
    Chord(Chord),
    Mouse(MouseEvent),
    Paste(String),
    Resize {
        cols: u16,
        rows: u16,
    },
    Tick,
    Runtime(RuntimeEvent),
    /// The prompt widget emitted a submit. Carries the expanded buffer text.
    PromptSubmit(String),
    /// The user pressed `Ctrl+C` in the prompt with a non-empty buffer; the
    /// host should clear local state in response.
    PromptCancel,
    /// Cycle the Shell route's LEFT tab cluster. `forward = true` for `Tab`,
    /// `false` for `Shift+Tab`.
    ShellTabCycle {
        forward: bool,
    },
    /// Jump the Shell route to a specific tab (`1` / `2` / `3`).
    ShellTabSet(ShellTab),
    /// Toggle the Shell/Session sidebar (`Ctrl+B`).
    SidebarToggle,
    /// Begin the empty-state engagement animation. Fired by the Enter key on
    /// the Shell route when the prompt buffer is empty (and the engagement
    /// state is still `Idle`). Idempotent — the slide does not restart once
    /// it has begun.
    EngageSession,
    /// Jnoccio boot thread reported a status change.
    JnoccioBootUpdate(JnoccioBootStatus),
    /// User requested a jankurai audit (via `/audit` slash command or chat intercept).
    RunJankuraiAudit,
    /// User requested a full jankurai cycle: audit → analyze → fix → verify → reaudit.
    RunJankuraiCycle,
    /// User explicitly confirmed the mutating jankurai cycle.
    RunJankuraiCycleConfirmed,
    /// A single progress line from the running jankurai audit subprocess.
    JankuraiAuditLine(String),
    /// A single progress line from the jankurai-runner subprocess during a cycle.
    JankuraiRunnerLine(String),
    /// Background audit thread finished. `success` is false on non-zero exit.
    /// When successful, `summary` carries the parsed audit results so the app
    /// can auto-propose fixes for actionable findings.
    JankuraiScoreUpdate {
        success: bool,
        summary: Option<AuditSummary>,
    },
    /// Full jankurai cycle completed. `improved` is true if the re-audit showed
    /// a better score than the initial audit.
    JankuraiCycleComplete {
        improved: bool,
    },
}

/// Frame cadence target for the Ratatui draw loop (60 fps).
pub const FRAME_TICK: Duration = Duration::from_millis(16);

/// First-frame watchdog timeout.
pub const FIRST_FRAME_WATCHDOG: Duration = Duration::from_secs(5);

/// Default initial theme mode when the terminal palette query times out.
pub fn default_initial_theme() -> ThemeMode {
    ThemeMode::Dark
}
