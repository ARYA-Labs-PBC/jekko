use std::time::Duration;

use crossterm::event::{KeyEvent, MouseEvent};
use jekko_core::keybind::Chord;
use jekko_core::session::SessionId;
use jekko_core::theme::ThemeMode;
use serde::{Deserialize, Serialize};

use crate::activity::ActivityKind;

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
    Disabled,
    NotInstalled,
    Checking,
    Starting,
    /// Server is reachable. `model_count` is the number of routable models.
    Ready {
        enabled_models: u32,
        total_models: u32,
    },
    Failed,
}

impl JnoccioBootStatus {
    /// Compact human-readable label for status surfaces.
    pub fn label(&self) -> String {
        match self {
            Self::Idle => "idle".to_string(),
            Self::Disabled => "disabled".to_string(),
            Self::NotInstalled => "not installed".to_string(),
            Self::Checking => "checking".to_string(),
            Self::Starting => "booting".to_string(),
            Self::Ready {
                enabled_models,
                total_models,
            } => format!("ready {enabled_models}/{total_models}"),
            Self::Failed => "failed".to_string(),
        }
    }

    /// Short detail block for `/status` and `/panels` output.
    pub fn detail(&self) -> Option<String> {
        match self {
            Self::Idle => None,
            Self::Disabled => Some("jnoccio boot disabled".to_string()),
            Self::NotInstalled => Some("jnoccio-fusion not installed or not unlocked".to_string()),
            Self::Checking => Some("jnoccio checking local health".to_string()),
            Self::Starting => Some("jnoccio booting local server".to_string()),
            Self::Ready {
                enabled_models,
                total_models,
            } => Some(format!(
                "jnoccio ready\n  enabled models: {enabled_models}\n  total models:    {total_models}"
            )),
            Self::Failed => Some("jnoccio boot failed".to_string()),
        }
    }
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
        title: Option<String>,
    },
    SessionEnded {
        session_id: SessionId,
    },
    DaemonStatus {
        session_id: Option<SessionId>,
        status: String,
        message: Option<String>,
    },
    PermissionAsked {
        request_id: String,
        session_id: SessionId,
        permission: String,
        patterns: Vec<String>,
        always: Vec<String>,
    },
    PermissionReplied {
        request_id: String,
        session_id: SessionId,
        reply: String,
    },
    QuestionAsked {
        question_id: String,
        session_id: SessionId,
        prompt: String,
        choices: Vec<String>,
    },
    QuestionReplied {
        question_id: String,
        session_id: SessionId,
        answer: String,
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
    /// Tool-call lifecycle event (start/stdout/stderr/complete/fail).
    Tool(ToolEvent),
}

/// Streaming tool-call event, surfaced by the chat-bridge SSE worker and
/// rendered as a live status chip / tool card in the inline runtime.
#[derive(Clone, Debug)]
pub enum ToolEvent {
    Start {
        id: String,
        name: String,
        input: Option<String>,
    },
    StdoutChunk {
        id: String,
        chunk: String,
    },
    StderrChunk {
        id: String,
        chunk: String,
    },
    /// Full current terminal render for a PTY-backed tool, emitted by
    /// `engine::pty_runner`. Unlike `StdoutChunk`/`StderrChunk` (which the chip
    /// *appends*), this carries the entire emulated screen and *replaces* the
    /// chip's captured output. It is how in-place progress bars (`\r` + clear
    /// line, cursor moves) collapse onto a single updating line instead of
    /// flooding the transcript with one row per redraw frame.
    ScreenUpdate {
        id: String,
        text: String,
    },
    Complete {
        id: String,
    },
    Fail {
        id: String,
        error: String,
    },
}

/// A single actionable finding from a jankurai audit.
#[derive(Clone, Debug, Serialize, Deserialize)]
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
#[derive(Clone, Debug, Serialize, Deserialize)]
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
    /// Jnoccio boot thread reported a status change.
    JnoccioBootUpdate(JnoccioBootStatus),
    /// Long-running activity began or progressed.
    ActivityUpdated {
        id: String,
        kind: ActivityKind,
        label: Option<String>,
        status: Option<String>,
        progress: Option<(u64, u64)>,
    },
    /// Long-running activity finished.
    ActivityFinished {
        id: String,
        kind: ActivityKind,
        label: Option<String>,
        status: Option<String>,
        success: bool,
    },
    /// User requested a jankurai audit (via `/audit` slash command or chat intercept).
    RunJankuraiAudit,
    /// User requested the compatibility Jankurai action; this runs an external audit.
    RunJankuraiCycle,
    /// User explicitly confirmed the compatibility Jankurai action.
    RunJankuraiCycleConfirmed,
    /// A single progress line from the running jankurai audit subprocess.
    JankuraiAuditLine(String),
    /// A single progress line from a compatibility Jankurai action.
    JankuraiRunnerLine(String),
    /// Background audit thread finished. `success` is false on non-zero exit.
    /// When successful, `summary` carries the parsed audit results so the app
    /// can auto-propose fixes for actionable findings.
    JankuraiScoreUpdate {
        success: bool,
        summary: Option<AuditSummary>,
    },
    /// Compatibility Jankurai action completed. `improved` is retained for old
    /// callers and is always false for read-only external audits.
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
