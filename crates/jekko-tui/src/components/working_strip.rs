//! Working/status strip (COWBOY.md T1-V3, per `tips/fucktui/tip6.txt`).
//!
//! One-row strip painted ABOVE the permission banner — but only when a turn
//! is in flight OR a background terminal is running. When idle and no
//! background work, the strip is hidden entirely (zero rows).
//!
//! ```text
//! ◦ Working (1m 5s • esc to interrupt) · 2 background terminal running · /ps to view · /stop to close
//! ```
//!
//! - The leading `◦` glyph pulses via [`anim::pulse_glyph`] and is painted
//!   in [`codex::CYAN_TAB`]. Reduced-motion users get the static "brightest"
//!   frame (handled inside `pulse_glyph`).
//! - The rest of the row is [`codex::FG_DIM`].
//! - At narrow widths, segments drop by priority via
//!   [`layout::status_pack::pack`]: pulse_label=0 (anchor) → time=1 →
//!   background=2 → ps_hint=3 → stop_hint=4 (drops first).
//!
//! Returns `true` if the strip drew anything (so the caller can decide
//! whether to reserve a row in its `Layout::vertical` constraint set).
//!
//! [`anim::pulse_glyph`]: crate::anim::pulse_glyph
//! [`codex::CYAN_TAB`]: crate::theme::codex::CYAN_TAB
//! [`codex::FG_DIM`]: crate::theme::codex::FG_DIM
//! [`layout::status_pack::pack`]: crate::layout::status_pack::pack

mod render;

#[cfg(test)]
mod tests;

pub use render::render_working_strip;
