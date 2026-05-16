//! Phase 10 / Packet I — Rust port of the JS session transcript.
//!
//! Ports the following TS sources into a cohesive Ratatui module:
//!
//! * `packages/jekko/src/cli/cmd/tui/routes/session/session-renderers.tsx` and
//!   `session-view.tsx` — the cards that compose a transcript.
//! * `packages/jekko/src/cli/cmd/tui/routes/session/permission.tsx` and
//!   `question-view.tsx` — inline permission and question prompts.
//! * `packages/jekko/src/cli/cmd/tui/util/transcript.ts`,
//!   `util/revert-diff.ts`, `util/terminal-tokenize.ts`,
//!   `util/yaml-tokenize.ts` — supporting utilities.
//! * `packages/jekko/src/cli/cmd/tui/routes/session/sidebar.tsx`,
//!   `subagent-footer.tsx`, `daemon-banner.tsx`, `footer.tsx` —
//!   surrounding chrome.
//!
//! No persistence and no IO live in this module. Callers in `jekko-cli` and
//! `jekko-server` push state in via the public `Transcript` API.
//!
//! ## Dependencies
//!
//! The orchestrator will wire `similar = "2"` (for unified diff parsing) and
//! optionally `vte = "0.13"` (for ANSI escape parsing) into `Cargo.toml` after
//! this packet lands. Until then, this module ships its own minimal parsers
//! (see [`diff`] and [`terminal_tokenize`]).

pub mod cards;
pub mod diff;
pub mod permission;
pub mod question;
pub mod route;
pub mod sidebar;
pub mod terminal_tokenize;
#[allow(clippy::module_inception)]
pub mod transcript;
pub mod yaml_tokenize;

pub use cards::{
    AssistantCard, AssistantPart, AssistantPartKind, ReasoningCard, SystemCard, SystemKind,
    ToolCard, ToolStatus, UserCard,
};
pub use diff::{parse_unified_diff, DiffFile, DiffHunk, DiffLine, DiffLineKind};
pub use permission::{
    PermissionAction, PermissionCard, PermissionChoice, PermissionDecisionEvent, PermissionStage,
};
pub use question::{QuestionCard, QuestionChoice, QuestionEvent, QuestionMode};
pub use route::SessionRoute;
pub use sidebar::{DaemonStatus, SidebarPanel, StickyBottomIndicator, SubagentFooter};
pub use terminal_tokenize::{tokenize_terminal, TerminalScope, TerminalSpan};
pub use transcript::{ScrollIntent, Transcript, TranscriptEntry};
pub use yaml_tokenize::{tokenize_yaml, YamlScope, YamlSpan};
