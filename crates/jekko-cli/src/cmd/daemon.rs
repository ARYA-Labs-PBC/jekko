//! `jekko daemon` — background daemon management.
//!
//! Mirrors `packages/jekko/src/cli/cmd/daemon.ts`.

use anyhow::Result;
use clap::{Args, Subcommand};

use crate::cli::GlobalOpts;

#[derive(Args, Debug)]
pub struct DaemonArgs {
    #[command(subcommand)]
    pub command: DaemonCommand,
}

#[derive(Subcommand, Debug)]
pub enum DaemonCommand {
    /// Start the daemon.
    Start(DaemonStartArgs),
    /// Stop the daemon.
    Stop,
    /// Print daemon status.
    Status,
    /// Tail daemon logs.
    Logs(DaemonLogsArgs),
}

#[derive(Args, Debug, Default)]
pub struct DaemonStartArgs {
    /// Detach into the background.
    #[arg(long)]
    pub detach: bool,
}

#[derive(Args, Debug, Default)]
pub struct DaemonLogsArgs {
    /// Follow new log lines as they are appended.
    #[arg(long, short = 'f')]
    pub follow: bool,
    /// Number of trailing lines to print.
    #[arg(long, short = 'n', default_value_t = 80)]
    pub lines: usize,
}

pub fn run(_global: &GlobalOpts, args: &DaemonArgs) -> Result<()> {
    match &args.command {
        DaemonCommand::Start(opts) => start(opts),
        DaemonCommand::Stop => stop(),
        DaemonCommand::Status => status(),
        DaemonCommand::Logs(opts) => logs(opts),
    }
}

fn start(_args: &DaemonStartArgs) -> Result<()> {
    eprintln!("jekko daemon start: pending runtime integration");
    Ok(())
}

fn stop() -> Result<()> {
    eprintln!("jekko daemon stop: pending runtime integration");
    Ok(())
}

fn status() -> Result<()> {
    eprintln!("jekko daemon status: pending runtime integration");
    Ok(())
}

fn logs(_args: &DaemonLogsArgs) -> Result<()> {
    eprintln!("jekko daemon logs: pending runtime integration");
    Ok(())
}
