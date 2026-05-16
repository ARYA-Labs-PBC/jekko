//! ZYAL snapshot types fed to the panel.

use ratatui::style::Color;

/// Exit-tone discriminant. Mirrors the `EXIT_TONE` map from `zyal-palette.ts`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ZyalExitTone {
    /// Successful satisfaction of the runbook.
    Success,
    /// Paused / warning state.
    Warning,
    /// Crashed / hard-exited.
    Error,
}

impl ZyalExitTone {
    pub(super) fn color(self) -> Color {
        match self {
            ZyalExitTone::Success => Color::Rgb(0x00, 0xff, 0x87),
            ZyalExitTone::Warning => Color::Rgb(0xff, 0xd0, 0x00),
            ZyalExitTone::Error => Color::Rgb(0xff, 0x40, 0x60),
        }
    }

    pub(super) fn label(self) -> &'static str {
        match self {
            ZyalExitTone::Success => "ZYAL SATISFIED",
            ZyalExitTone::Warning => "ZYAL PAUSED",
            ZyalExitTone::Error => "ZYAL EXITED",
        }
    }
}

/// Latest exit record displayed at the top of the panel.
#[derive(Clone, Debug)]
pub struct ZyalExitRecord {
    /// Exit tone (color band).
    pub tone: ZyalExitTone,
    /// Short status label (e.g. `satisfied`, `eviction`).
    pub status: String,
    /// Reason text (one line).
    pub reason: String,
}

/// Runbook preview entry — one line of the user-pasted runbook the executor
/// will run next.
#[derive(Clone, Debug)]
pub struct ZyalRunbookLine {
    /// Step number (1-indexed).
    pub step: u32,
    /// Verbatim runbook line.
    pub text: String,
}

/// Live ZYAL snapshot. Static until `crates/zyalc` is wired in as a path
/// dep; the shape mirrors `useZyalMetrics()` + `useZyalExit()` from the JS
/// sidebar.
#[derive(Clone, Debug, Default)]
pub struct ZyalSnapshot {
    /// Short run id (post-trim).
    pub run_id: Option<String>,
    /// Status label (`active`, `paused`).
    pub status: Option<String>,
    /// Loop counter.
    pub loops_completed: u64,
    /// Tasks completed counter.
    pub tasks_completed: u64,
    /// Tasks incubated counter.
    pub tasks_incubated: u64,
    /// Total token spend across both input and output.
    pub total_tokens: u64,
    /// Input tokens (TS: `inputTokens`).
    pub input_tokens: u64,
    /// Output tokens.
    pub output_tokens: u64,
    /// Cache tokens.
    pub cache_tokens: u64,
    /// Active workers.
    pub workers_active: u32,
    /// Capacity ceiling.
    pub workers_max: u32,
    /// Total USD cost.
    pub cost_usd: f64,
    /// Wall-clock runtime label.
    pub uptime: Option<String>,
    /// Open jankurai findings (cross-feature integration).
    pub jankurai_findings: Option<u32>,
    /// Last paste signature, if a recent paste was detected.
    pub paste_signature: Option<String>,
    /// Total bytes pasted in the current run.
    pub paste_bytes: u64,
    /// Optional runbook preview (first N lines of the active runbook).
    pub runbook_preview: Vec<ZyalRunbookLine>,
    /// Exit record. `None` while the executor is live.
    pub exit: Option<ZyalExitRecord>,
}
