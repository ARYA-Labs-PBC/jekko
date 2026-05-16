//! Plugin manager dialog.
//!
//! Ports `packages/jekko/src/cli/cmd/tui/feature-plugins/system/plugins.tsx`
//! to a Ratatui widget. The original TS component composed `DialogSelect`
//! with rows for internal (Rust) plugins and external (declarative manifest)
//! plugins, plus a `Shift+I` shortcut to install a new package.
//!
//! This port keeps the same column shape (`id`, kind, version, theme/command
//! counts) and the keyboard model (`j`/`k`/arrows to move the cursor, `Space`
//! or `Enter` to toggle enabled, `Esc`/`q` to exit). The actual install action
//! is delegated to the caller through the [`PluginManager::install_requested`]
//! flag — the dialog itself does not shell out to npm.

mod manager;
mod render;
mod row;

#[cfg(test)]
mod tests;

pub use manager::PluginManager;
pub use row::{PluginRow, PluginRowKind};
