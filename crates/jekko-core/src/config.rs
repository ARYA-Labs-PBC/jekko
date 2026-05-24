//! `jekko.json` / `.jekko/tui.json` configuration shapes.

mod enums;
mod root;
mod sections;

#[cfg(test)]
mod tests;

/// UI-specific TOML configuration schema (pure overlay types, no I/O).
pub mod ui;

pub use enums::{AutoUpdate, Layout, LogLevel, NotifyLiteral, SharePolicy};
pub use root::Config;
pub use sections::{
    CompactionConfig, EnterpriseConfig, ExperimentalConfig, ToolOutputConfig, WatcherConfig,
};
