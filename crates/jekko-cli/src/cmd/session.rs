//! `jekko session` — session management subcommands.
//!
//! Mirrors `packages/jekko/src/cli/cmd/session.ts`. The TS surface only
//! exposes `list` and `delete`; we add `show`, `export`, and `import` here so
//! the help-parity tooling can collapse the standalone `jekko export` /
//! `jekko import` commands into a single tree later.

use anyhow::Result;
use clap::{Args, Subcommand};

use crate::cli::GlobalOpts;

/// `jekko session` top-level args.
#[derive(Args, Debug)]
pub struct SessionArgs {
    #[command(subcommand)]
    pub command: SessionCommand,
}

/// Session subcommands.
///
/// Examples:
/// ```text
/// jekko session list
/// jekko session show ses_abc
/// jekko session delete ses_abc
/// ```
#[derive(Subcommand, Debug)]
pub enum SessionCommand {
    /// List sessions.
    List(SessionListArgs),
    /// Show a single session.
    Show(SessionShowArgs),
    /// Delete a session.
    Delete(SessionDeleteArgs),
    /// Export a session to a file.
    Export(SessionExportArgs),
    /// Import a session from a file.
    Import(SessionImportArgs),
}

#[derive(Args, Debug, Default)]
pub struct SessionListArgs {
    /// Limit to the N most recent sessions.
    #[arg(short = 'n', long = "max-count", value_name = "N")]
    pub max_count: Option<usize>,
    /// Output format. Defaults to a table.
    #[arg(long, value_name = "FORMAT", default_value = "table")]
    pub format: String,
}

#[derive(Args, Debug)]
pub struct SessionShowArgs {
    /// Session id to show.
    pub session_id: String,
}

#[derive(Args, Debug)]
pub struct SessionDeleteArgs {
    /// Session id to delete.
    pub session_id: String,
}

#[derive(Args, Debug)]
pub struct SessionExportArgs {
    /// Session id to export.
    pub session_id: String,
    /// Output path. Defaults to `./<id>.json`.
    #[arg(short = 'o', long = "out", value_name = "PATH")]
    pub out: Option<std::path::PathBuf>,
}

#[derive(Args, Debug)]
pub struct SessionImportArgs {
    /// Input path.
    pub input: std::path::PathBuf,
}

/// Entry point for the `session` subtree.
pub fn run(_global: &GlobalOpts, args: &SessionArgs) -> Result<()> {
    match &args.command {
        SessionCommand::List(opts) => list(opts),
        SessionCommand::Show(opts) => show(opts),
        SessionCommand::Delete(opts) => delete(opts),
        SessionCommand::Export(opts) => export(opts),
        SessionCommand::Import(opts) => import(opts),
    }
}

fn list(_args: &SessionListArgs) -> Result<()> {
    eprintln!("jekko session list: pending runtime integration");
    Ok(())
}

fn show(args: &SessionShowArgs) -> Result<()> {
    eprintln!(
        "jekko session show {}: pending runtime integration",
        args.session_id
    );
    Ok(())
}

fn delete(args: &SessionDeleteArgs) -> Result<()> {
    eprintln!(
        "jekko session delete {}: pending runtime integration",
        args.session_id
    );
    Ok(())
}

fn export(args: &SessionExportArgs) -> Result<()> {
    eprintln!(
        "jekko session export {}: pending runtime integration",
        args.session_id
    );
    Ok(())
}

fn import(args: &SessionImportArgs) -> Result<()> {
    eprintln!(
        "jekko session import {}: pending runtime integration",
        args.input.display()
    );
    Ok(())
}
