//! ZYAL research-loop runbook panel.
//!
//! Ports `packages/jekko/src/cli/cmd/tui/feature-plugins/sidebar/zyal/` —
//! `view.tsx`, `constants.ts` — and the related ZYAL context (`zyal-flash`,
//! `zyal-runner`) to a Ratatui widget.
//!
//! The panel shows:
//! * a `∞ ZYAL MODE` sigil at the top with the current status dot;
//! * a runbook preview / paste-detector summary;
//! * loop / token / worker counters in the neon palette;
//! * a `✓ ZYAL` confirmation row that lights up when an exit is recorded;
//! * an optional auto-research sub-panel.
//!
//! Live data is wired through [`ZyalSnapshot`] until the orchestrator brings
//! in `crates/zyalc`. The dispatcher consumes the `Esc` and `q` keys to exit;
//! everything else is read-only.
//!
//! Integrate `crates/zyalc::parse_runbook` for live paste detection once the
//! parser becomes available.

mod palette;
mod panel;
mod snapshot;

#[cfg(test)]
mod tests;

pub use panel::ZyalPanel;
pub use snapshot::{ZyalExitRecord, ZyalExitTone, ZyalRunbookLine, ZyalSnapshot};
