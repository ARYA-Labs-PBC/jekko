//! Time-derived animation primitives (COWBOY.md D1).
//!
//! All animation is computed from elapsed time, NOT state mutation. This keeps
//! the render path pure: given the same instant + same inputs, you always get
//! the same output. The chat runtime ticks a render interval (~33ms) and these
//! helpers produce the right glyph/color/text for the current frame.

use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::Duration;
use std::{env, fs};

use jekko_core::config::ui::UiConfig;
use ratatui::style::Color;

const PULSE_PERIOD: Duration = Duration::from_millis(160);
const OSCILLATE_DEFAULT_HZ: f32 = 1.0;
const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
const PULSE_FRAMES: &[&str] = &["●", "◉", "◌"];

include!("anim/motion.rs");
include!("anim/glyphs.rs");
include!("anim/color.rs");
include!("anim/text.rs");

#[cfg(test)]
include!("anim/tests.rs");
