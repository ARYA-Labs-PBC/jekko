//! `jekko models` — list and inspect available models.
//!
//! Mirrors `packages/jekko/src/cli/cmd/models.ts`.
//!
//! Examples:
//! ```text
//! jekko models list
//! jekko models show anthropic/claude-opus-4-7
//! jekko models switch openai/gpt-5
//! ```

use anyhow::Result;
use clap::{Args, Subcommand};

use crate::cli::GlobalOpts;

/// `jekko models` args.
#[derive(Args, Debug, Default)]
pub struct ModelsArgs {
    #[command(subcommand)]
    pub command: Option<ModelsCommand>,

    /// Provider to filter by when no subcommand is given.
    #[arg(value_name = "PROVIDER")]
    pub provider: Option<String>,

    /// Print full model metadata (cost, context window, ...).
    #[arg(long, short = 'v')]
    pub verbose: bool,

    /// Refresh the cached `models.dev` catalog before listing.
    #[arg(long)]
    pub refresh: bool,
}

/// Subcommands recognised by `jekko models`. The default action (no
/// subcommand) lists all known models.
#[derive(Subcommand, Debug)]
pub enum ModelsCommand {
    /// List models. Optionally filter by provider id.
    List(ModelsListArgs),
    /// Show details for a single model.
    Show(ModelsShowArgs),
    /// Switch the default model for the active workspace.
    Switch(ModelsShowArgs),
}

#[derive(Args, Debug, Default)]
pub struct ModelsListArgs {
    /// Provider to filter by (e.g. `anthropic`).
    pub provider: Option<String>,
    /// Print full model metadata.
    #[arg(long, short = 'v')]
    pub verbose: bool,
}

#[derive(Args, Debug)]
pub struct ModelsShowArgs {
    /// Model identifier (e.g. `anthropic/claude-opus-4-7`).
    pub model: String,
}

pub fn run(_global: &GlobalOpts, args: &ModelsArgs) -> Result<()> {
    match &args.command {
        Some(ModelsCommand::List(opts)) => list(opts),
        Some(ModelsCommand::Show(opts)) => show(opts),
        Some(ModelsCommand::Switch(opts)) => switch(opts),
        None => list(&ModelsListArgs {
            provider: args.provider.clone(),
            verbose: args.verbose,
        }),
    }
}

fn list(_args: &ModelsListArgs) -> Result<()> {
    eprintln!("jekko models list: pending model catalog integration");
    Ok(())
}

fn show(args: &ModelsShowArgs) -> Result<()> {
    eprintln!(
        "jekko models show {}: pending model catalog integration",
        args.model
    );
    Ok(())
}

fn switch(args: &ModelsShowArgs) -> Result<()> {
    eprintln!(
        "jekko models switch {}: pending model catalog integration",
        args.model
    );
    Ok(())
}
