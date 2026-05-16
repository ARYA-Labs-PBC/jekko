//! `jekko plugin` — manage plugins (via `jekko-plugin-api`).
//!
//! Mirrors `packages/jekko/src/cli/cmd/plug.ts`.

use anyhow::Result;
use clap::{Args, Subcommand};

use crate::cli::GlobalOpts;

#[derive(Args, Debug)]
pub struct PluginArgs {
    #[command(subcommand)]
    pub command: PluginCommand,
}

#[derive(Subcommand, Debug)]
pub enum PluginCommand {
    /// List installed plugins.
    List,
    /// Install a plugin package.
    Install(PluginInstallArgs),
    /// Enable a plugin.
    Enable(PluginNameArgs),
    /// Disable a plugin.
    Disable(PluginNameArgs),
}

#[derive(Args, Debug, Default)]
pub struct PluginInstallArgs {
    /// Plugin package name (npm spec or local path).
    pub module: String,
    /// Install in the global config.
    #[arg(long, short = 'g')]
    pub global: bool,
    /// Replace an existing plugin of the same name.
    #[arg(long, short = 'f')]
    pub force: bool,
}

#[derive(Args, Debug)]
pub struct PluginNameArgs {
    /// Plugin name.
    pub name: String,
}

pub fn run(_global: &GlobalOpts, args: &PluginArgs) -> Result<()> {
    match &args.command {
        PluginCommand::List => list(),
        PluginCommand::Install(opts) => install(opts),
        PluginCommand::Enable(opts) => enable(opts),
        PluginCommand::Disable(opts) => disable(opts),
    }
}

fn list() -> Result<()> {
    // The registry surface exists in jekko-plugin-api; this keeps the
    // command grounded in the live registry shape while the loader lands.
    let registry = jekko_plugin_api::PluginRegistry::default();
    eprintln!(
        "jekko plugin list: {} themes, {} commands, {} model presets registered",
        registry.themes().len(),
        registry.commands().len(),
        registry.model_presets().len(),
    );
    Ok(())
}

fn install(args: &PluginInstallArgs) -> Result<()> {
    eprintln!(
        "jekko plugin install {}: pending Packet C plugin manager",
        args.module
    );
    Ok(())
}

fn enable(args: &PluginNameArgs) -> Result<()> {
    eprintln!(
        "jekko plugin enable {}: pending Packet C plugin manager",
        args.name
    );
    Ok(())
}

fn disable(args: &PluginNameArgs) -> Result<()> {
    eprintln!(
        "jekko plugin disable {}: pending Packet C plugin manager",
        args.name
    );
    Ok(())
}
