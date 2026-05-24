//! Pure UI configuration schema for `~/.config/jekko/ui.toml`.
//!
//! This module intentionally contains no filesystem, environment, clock, or TOML
//! loading logic. Higher crates own side effects and feed parsed values into
//! these serde-friendly types.

mod animation;
mod root;
mod sections;

#[cfg(test)]
mod tests;

pub use animation::AnimationLevel;
pub use root::UiConfig;
pub use sections::{
    AccessibilitySection, ExecutionSection, InputSection, StatusSection, UiSection,
};
