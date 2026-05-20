//! Jekko TUI library — Claude/Codex-style chat surface.
//!
//! The legacy alt-screen 3-pane shell (`app.rs`, `feature_plugins/`,
//! `dialog/`, etc.) was removed in the R3 legacy purge. The user-facing
//! runtime now lives in [`inline_runtime`] (despite the name, it owns the
//! fullscreen alt-screen agent UI as well as the `--no-alt-screen`
//! compatibility inline viewport).

pub mod action;
pub mod activity;
pub mod agents;
pub mod anim;
pub mod background;
pub mod chat_bridge;
pub mod chat_bridge_backend;
pub mod components;
pub mod engine;
pub mod format;
pub mod glyph_set;
/// Claude/Codex-style TUI runtime. Top-level entry point for chat sessions.
pub mod inline_runtime;
pub mod layout;
pub mod lifecycle;
pub mod osc52;
pub mod prompt;
pub mod slash;
pub mod theme;
pub mod transcript;

pub use action::{
    default_initial_theme, Action, Route, RuntimeEvent, ToolEvent, FIRST_FRAME_WATCHDOG, FRAME_TICK,
};
pub use activity::{ActiveOperation, ActivityKind, ActivityTracker};
pub use lifecycle::{
    enter_agent_terminal, leave_agent_terminal, print_fatal_startup_error, restore_for_fatal,
    AgentTerminalOptions, TerminalRestoreGuard, Tty, FATAL_RESTORE_BYTES,
};
pub use prompt::{
    builtin_commands, display_width, grapheme_count, grapheme_offsets, truncate_to_width, Frecency,
    FrecencyRank, MentionCandidate, MentionPopup, PasteBuffer, PasteRecord, Prompt, PromptHistory,
    PromptOutcome, PromptSnapshot, PromptStash, RouteKey, SlashCommand, SlashPopup,
    PASTE_BYTE_THRESHOLD, PASTE_LINE_THRESHOLD,
};
pub use transcript::{
    parse_unified_diff, tokenize_terminal, tokenize_yaml, DiffFile, DiffHunk, DiffLine,
    DiffLineKind, TerminalScope, TerminalSpan, YamlScope, YamlSpan,
};

pub mod mouse;
