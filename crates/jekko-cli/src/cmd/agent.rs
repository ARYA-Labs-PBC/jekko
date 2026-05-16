//! `jekko agent` — agent management.
//!
//! Mirrors `packages/jekko/src/cli/cmd/agent.ts`.

use anyhow::Result;
use clap::{Args, Subcommand};

use crate::cli::GlobalOpts;

#[derive(Args, Debug)]
pub struct AgentArgs {
    #[command(subcommand)]
    pub command: AgentCommand,
}

#[derive(Subcommand, Debug)]
pub enum AgentCommand {
    /// Create a new agent.
    Create(AgentCreateArgs),
    /// List installed agents.
    List,
    /// Show details for an agent.
    Show(AgentShowArgs),
    /// Remove an agent.
    Remove(AgentShowArgs),
}

#[derive(Args, Debug, Default)]
pub struct AgentCreateArgs {
    /// Description (passed to the generator).
    #[arg(long)]
    pub description: Option<String>,
    /// Mode: `all`, `primary`, or `subagent`.
    #[arg(long, value_name = "MODE")]
    pub mode: Option<String>,
    /// Comma-separated list of permissions to allow.
    #[arg(long)]
    pub permissions: Option<String>,
    /// Model (provider/model).
    #[arg(long, short = 'm')]
    pub model: Option<String>,
    /// Output directory.
    #[arg(long)]
    pub path: Option<std::path::PathBuf>,
}

#[derive(Args, Debug)]
pub struct AgentShowArgs {
    /// Agent identifier.
    pub agent: String,
}

pub fn run(_global: &GlobalOpts, args: &AgentArgs) -> Result<()> {
    match &args.command {
        AgentCommand::Create(opts) => create(opts),
        AgentCommand::List => list(),
        AgentCommand::Show(opts) => show(opts),
        AgentCommand::Remove(opts) => remove(opts),
    }
}

fn create(_args: &AgentCreateArgs) -> Result<()> {
    eprintln!("jekko agent create: pending runtime integration");
    Ok(())
}

fn list() -> Result<()> {
    eprintln!("jekko agent list: pending runtime integration");
    Ok(())
}

fn show(args: &AgentShowArgs) -> Result<()> {
    eprintln!(
        "jekko agent show {}: pending runtime integration",
        args.agent
    );
    Ok(())
}

fn remove(args: &AgentShowArgs) -> Result<()> {
    eprintln!(
        "jekko agent remove {}: pending runtime integration",
        args.agent
    );
    Ok(())
}
