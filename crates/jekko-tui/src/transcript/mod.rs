//! Transcript rendering primitives for the Claude/Codex-style chat surface.
//!
//! The legacy session-route cards (`cards/`, `permission`, `question`,
//! `route`, `sidebar`, and the wrapper `Transcript` container) were removed
//! in the R3 legacy purge. The inline renderer in [`inline_cards`] is the
//! only public card grammar now; supporting diff/syntax/terminal/yaml
//! tokenizers remain because the inline renderer leans on them.

pub mod diff;
/// Claude/Codex-style inline-viewport renderer (used by both the fullscreen
/// alt-screen runtime and the `--no-alt-screen` compatibility path).
pub mod inline_cards;
pub mod markup;
pub mod runtime;
pub mod syntax;
pub mod terminal_tokenize;
pub mod yaml_tokenize;

pub use diff::{parse_unified_diff, DiffFile, DiffHunk, DiffLine, DiffLineKind};
pub use runtime::{IntoTranscriptEvent, Transcript, TranscriptEvent};
pub use terminal_tokenize::{tokenize_terminal, TerminalScope, TerminalSpan};
pub use yaml_tokenize::{tokenize_yaml, YamlScope, YamlSpan};
