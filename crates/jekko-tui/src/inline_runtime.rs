//! Agent TUI runtime — the user-facing replacement for `run_with_jnoccio`.
//!
//! Renders the Claude/Codex-style chat experience: a fullscreen app-owned
//! alternate-screen session with internal scrollback, a pinned bottom composer,
//! slash/file popups, and live streaming/tool feedback. `--no-alt-screen`
//! keeps the compatibility inline viewport and native scrollback path.
//!
//! Streaming: while a backend turn is in flight, the inline viewport grows to
//! hold the streaming assistant card. When the turn completes, the final card
//! is pushed into scrollback and the viewport snaps back to the composer.
//!
//! Backend wiring is closure-based so this module stays free of any specific
//! chat-bridge dependency. Callers (e.g. `jekko-cli::cmd::chat`) implement
//! `ChatBackend` to send user prompts and yield streaming events.

use std::borrow::Cow;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use crossterm::event::{Event, EventStream, KeyCode, KeyEvent, KeyModifiers};
use futures_util::StreamExt;
use indexmap::IndexMap;
use jekko_core::jankurai::parse_jankurai_score_json;
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

use crate::action::{JnoccioBootStatus, ToolEvent};
use crate::agents::panel::{render as render_agent_panel, PanelRenderOptions};
use crate::agents::{AgentKind, AgentPanelState, AgentRun, AgentStatus};
use crate::anim::{elapsed_label, motion_enabled_with_cfg};
use crate::background::{BackgroundJobManager, JobId, JobStatus};
use crate::components::boot_inline::{render_inline_boot_block, BootContext};
use crate::components::footer_status::{render_footer_status, FooterInfo};
use crate::components::output_pager::{
    handle_key as pager_handle_key, render_pager, PagerAction, PagerState,
};
use crate::components::permission_banner::{
    render_permission_banner, HINT_AGENT_PANEL_FOCUS, HINT_CHAT_FOCUS,
};
use crate::components::splash::{
    render_splash, snapshot_lines as splash_snapshot_lines, SplashContext, SPLASH_ROW_COUNT,
};
use crate::components::toast::{Toast, ToastStack};
use crate::components::working_strip::render_working_strip;
use crate::engine::cancel::{CancelLevel, CancellationToken, Escalator};
use crate::engine::output_collapse::OutputBuffer;
use crate::glyph_set;
use crate::lifecycle::{
    enter_agent_terminal, leave_agent_terminal, AgentTerminalOptions, TerminalRestoreGuard, Tty,
};
use crate::mouse::{map_mouse_event, MouseAction};
use crate::osc52;
use crate::prompt::file_index::FileIndex;
use crate::prompt::{paste::PasteBuffer, truncate_to_width, PROMPT_GLYPH, PROMPT_PREFIX_WIDTH};
use crate::slash::{SlashAction, SlashCatalog, SlashCommand, SlashSubcommand};
use crate::theme;
#[cfg(test)]
use crate::theme::codex::BLUE_PATH;
use crate::transcript::{
    inline_cards::{
        render_assistant, render_diff, render_permission_chip, render_question_chip,
        render_reasoning, render_session_header, render_system_notice, render_tool_call,
        render_tool_call_live, render_user, ActionStatus, DiffLine, NoticeKind, ToolCall,
    },
    Transcript, TranscriptEvent,
};

const MENTION_POPUP_LIMIT: usize = 8;
const FILE_INDEX_MAX_ENTRIES: usize = 5000;
const STREAM_FRAME_BUDGET: Duration = Duration::from_millis(33);
const LEGACY_PANELS_ENV: &str = "JEKKO_INLINE_SHOW_PANELS";
// T-BG-COUNT-MANAGER: sweep cadence for the background-job manager. Trims
// finished entries every ~5s so the `/ps` view doesn't accumulate noise from
// short-lived jobs, while keeping a 30s grace window so users have time to
// notice that a backgrounded command exited.
const BG_SWEEP_INTERVAL: Duration = Duration::from_secs(5);
const BG_RETAIN_AFTER_FINISH: Duration = Duration::from_secs(30);

include!("inline_runtime/state.rs");
include!("inline_runtime/runtime.rs");
include!("inline_runtime/commands.rs");
include!("inline_runtime/layout.rs");
include!("inline_runtime/render.rs");
include!("inline_runtime/composer.rs");
include!("inline_runtime/popups.rs");
include!("inline_runtime/support.rs");
#[cfg(test)]
include!("inline_runtime/tests.rs");
#[cfg(test)]
include!("inline_runtime/layout_and_runtime_tests.rs");

// Silence unused warning when the panic-hook helper is not referenced by the CLI binary build path.
#[allow(dead_code)]
fn _unused_sender_lifetime_marker(_tx: &Sender<ChatEvent>) {}
