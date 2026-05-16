//! Jankurai audit-live panel.
//!
//! Ports `packages/jekko/src/cli/cmd/tui/feature-plugins/jankurai/` —
//! `index.tsx`, `panel-audit-live.tsx`, `delta.ts`, `sparkline.ts` — to a
//! Ratatui widget.
//!
//! The panel surfaces:
//! * the current Jankurai score with a unicode sparkline history;
//! * delta-vs-baseline columns for caps, hard findings, soft findings, score;
//! * a worker roster pulled from the ZYAL runner;
//! * a last-updated timestamp ("Audit · 23s · pass · v3.1").
//!
//! Live data is intentionally a static snapshot ([`JankuraiSnapshot`]) until the
//! orchestrator wires `crates/jankurai-runner` into the App. The shape mirrors
//! the score document the JS panel reads from `~/.jankurai/score.json`.
//!
//! Once integrated, replace the static snapshot with a live read from
//! `jankurai_runner::events::ScoreEvent` (and ditto for `useJankuraiHistory`
//! and `useZyalWorkers`).

mod delta;
mod detect;
mod panel;
mod runner;
mod snapshot;
mod sparkline;
mod style;

mod tests;

pub use delta::{compute_delta, format_delta, DeltaDirection, DeltaMetric};
pub use detect::{is_jankurai_installed, JANKURAI_INSTALL_URL};
pub use panel::JankuraiPanel;
pub use runner::run_audit;
pub use snapshot::{JankuraiSnapshot, JankuraiWorker};
pub use sparkline::sparkline;
