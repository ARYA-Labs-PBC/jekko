//! Pager state machine + renderer for expanded tool output (COWBOY T2-P3).
//!
//! When a user `Ctrl+O`-expands a collapsed
//! [`OutputBuffer`](crate::engine::output_collapse::OutputBuffer) the contents
//! get handed off to this pager. The pager owns three responsibilities:
//!
//! 1. Vertical scroll within a `Vec<String>` of pre-split lines, clamped to
//!    bounds so `End` and oversize `PageDown` never run past the buffer.
//! 2. A `/` search prompt with `n`/`N` navigation across matches. Matches are
//!    case-sensitive substring hits collected via [`str::match_indices`].
//! 3. A yank path (`y` for current line, `Y` for all visible lines) that
//!    returns the payload as [`PagerAction::Yank`] -- the caller decides
//!    how to write the clipboard (typically [`crate::osc52`]).
//!
//! Visual contract:
//!
//! ```text
//! -- Pager · 0/200 lines · 3 matches -------------------------------
//! line 0
//! line 1 [matchA]
//! ...
//! line 9
//! / matchA█
//! ```

mod input;
mod render;
mod state;

#[cfg(test)]
mod tests;

pub use input::{handle_key, PagerAction};
pub use render::render_pager;
pub use state::{MatchRef, PagerMode, PagerState};
