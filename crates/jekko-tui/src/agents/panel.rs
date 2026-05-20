//! AgentPanel widget — renders the multi-agent rail below the composer
//! (COWBOY.md L2, per tips/fucktui/tip9.txt).
//!
//! Pure render → `Vec<Line<'static>>`. Caller decides where to paint.

use std::borrow::Cow;
use std::time::Instant;

use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

use crate::agents::{AgentLocality, AgentPanelState, AgentRun, AgentStatus};
use crate::anim::{elapsed_label, pulse_glyph_with_motion};
use crate::format::format_tokens_with_direction;
use crate::glyph_set;
use crate::prompt::truncate_to_width;
use crate::theme::{
    codex_bg_overlay, codex_cyan_tab, codex_fg, codex_fg_dim, codex_fg_strong, codex_fg_very_dim,
    codex_green_ok, codex_orange_agent, codex_pink_agent, codex_salmon_fail,
};

include!("panel/options.rs");
include!("panel/render.rs");
include!("panel/rows.rs");

#[cfg(test)]
include!("panel/tests.rs");
