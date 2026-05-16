//! Phase 9 / Packet H — Rust port of the JS prompt component.
//!
//! Ports `packages/jekko/src/cli/cmd/tui/component/prompt/` into a single
//! `Prompt` widget plus supporting state types. The widget owns:
//!
//! * a `tui_textarea::TextArea` for multi-line editing,
//! * a slash-command popup (built-ins until Packet B fills the catalog),
//! * an `@`-mention popup over a caller-supplied file list,
//! * an in-memory paste buffer that replaces large pastes with summary chips,
//! * in-memory history / frecency / per-route stash state.
//!
//! No persistence happens in this module — JSONL load/save lives in the host
//! crate.

pub mod frecency;
pub mod history;
pub mod mentions;
pub mod paste;
pub mod slash;
pub mod stash;
pub mod unicode;
mod widget;

pub use frecency::{Frecency, FrecencyRank};
pub use history::PromptHistory;
pub use mentions::{MentionCandidate, MentionPopup};
pub use paste::{PasteBuffer, PasteRecord, PASTE_BYTE_THRESHOLD, PASTE_LINE_THRESHOLD};
pub use slash::{builtin_commands, SlashCommand, SlashPopup};
pub use stash::{PromptStash, RouteKey};
pub use unicode::{display_width, grapheme_count, grapheme_offsets, truncate_to_width};
pub use widget::{Prompt, PromptOutcome, PromptSnapshot};
