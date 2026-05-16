//! Inline question card.
//!
//! Ports `packages/jekko/src/cli/cmd/tui/routes/session/question-view.tsx`
//! plus the controller in `question-controller.ts`. The widget supports
//! single-choice, multi-choice, and custom-answer flows.

mod input;
mod model;
mod render;

#[cfg(test)]
mod tests;

pub use model::{QuestionCard, QuestionChoice, QuestionEvent, QuestionMode};
