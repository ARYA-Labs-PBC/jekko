//! Slash command registry (COWBOY.md I1/I2/I3).
//!
//! Owns the slash-command catalog used by the inline runtime popup. Built-in
//! commands and workspace-defined `.jankurai/commands/*.md` commands are
//! exposed through one [`SlashCatalog`] surface.

mod action;
mod builtins;
mod catalog;
mod command;
mod submenu;

#[cfg(test)]
mod tests;

pub mod user_defined;

pub use action::SlashAction;
pub use builtins::BUILTIN_SLASH;
pub use catalog::SlashCatalog;
pub use command::SlashCommand;
pub use submenu::{SlashSubcommand, SlashSubmenu, SLASH_SUBMENUS};
