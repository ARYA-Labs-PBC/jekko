//! Transcript state container.
//!
//! Mirrors the scroll/append behavior of `session-body-core.tsx` plus
//! `session-view.tsx`'s `<scrollbox stickyScroll stickyStart="bottom" />`.
//! The widget itself is dumb: it does not render the cards (that lives in
//! [`crate::transcript::cards`] / [`crate::transcript::route`]). It owns
//! ordered entries, scroll state, sticky-bottom logic, and a small
//! acceleration model for held-down arrows.

mod container;
mod entry;
mod scroll;

#[cfg(test)]
mod tests;

pub use container::Transcript;
pub use entry::TranscriptEntry;
pub use scroll::{ScrollAcceleration, ScrollIntent, ScrollState};
