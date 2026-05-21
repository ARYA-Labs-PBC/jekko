//! Jekko CLI library surface.
//!
//! This crate ships a `jekko` binary (see `src/main.rs`) and a small library
//! that exposes the parsed `Cli` shape so xtask/help-parity tooling can render
//! the command tree without re-parsing argv.
//!
//! The TypeScript predecessor lives at `packages/jekko/src/index.ts` and
//! `packages/jekko/src/cli/cmd/**`. Subcommand modules under [`cmd`] mirror
//! that layout one-to-one.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod cli;
pub mod cmd;
pub mod config_loader;
pub mod migration_ui;
pub mod runtime;

pub use cli::{Cli, Command, GlobalOpts};
