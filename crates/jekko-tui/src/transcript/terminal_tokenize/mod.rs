//! Terminal-output tokenizer.
//!
//! Ports `packages/jekko/src/cli/cmd/tui/util/terminal-tokenize.ts`. Produces
//! a stable, non-overlapping list of [`TerminalSpan`] entries flagged with a
//! coarse [`TerminalScope`] (string, number, warning, etc.). Used by the
//! tool card renderer to colour shell output.
//!
//! This is presentation-only — the scan is bounded to keep terminal rendering
//! cheap on arbitrary log output. ANSI escape sequences are stripped before
//! tokenising; the stripped string is what callers should render.

mod ansi;
mod matchers;
mod tokenize;
mod types;

#[cfg(test)]
mod tests;

pub use ansi::strip_ansi;
pub use tokenize::tokenize_terminal;
pub use types::{TerminalScope, TerminalSpan};
