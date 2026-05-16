//! `jekko acp` — Agent Client Protocol integration.
//!
//! Mirrors `packages/jekko/src/cli/cmd/acp.ts`.

use anyhow::Result;
use clap::{Args, Subcommand};

use crate::cli::GlobalOpts;

#[derive(Args, Debug)]
pub struct AcpArgs {
    #[command(subcommand)]
    pub command: AcpCommand,
}

#[derive(Subcommand, Debug)]
pub enum AcpCommand {
    /// Initialise an ACP session (stdio transport).
    Init(AcpInitArgs),
    /// Show ACP status.
    Status,
}

#[derive(Args, Debug, Default)]
pub struct AcpInitArgs {
    /// Working directory for the ACP session.
    #[arg(long)]
    pub cwd: Option<std::path::PathBuf>,
}

pub fn run(_global: &GlobalOpts, args: &AcpArgs) -> Result<()> {
    match &args.command {
        AcpCommand::Init(opts) => init(opts),
        AcpCommand::Status => status(),
    }
}

fn init(_args: &AcpInitArgs) -> Result<()> {
    eprintln!("jekko acp init: pending ACP integration");
    Ok(())
}

fn status() -> Result<()> {
    eprintln!("jekko acp status: pending ACP integration");
    Ok(())
}
