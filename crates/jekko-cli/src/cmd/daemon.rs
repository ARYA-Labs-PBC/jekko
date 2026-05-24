//! `jekko daemon` — background daemon management.
//!
//! Mirrors `packages/jekko/src/cli/cmd/daemon.ts`.

mod args;
mod control;
mod metadata;
mod port_run;
mod status_report;

pub use args::*;

use anyhow::Result;

use crate::cli::GlobalOpts;

pub fn run(_global: &GlobalOpts, args: &DaemonArgs) -> Result<()> {
    match &args.command {
        DaemonCommand::Start(opts) => control::start(opts),
        DaemonCommand::Stop => control::stop(),
        DaemonCommand::Status => status_report::status(),
        DaemonCommand::Logs(opts) => control::logs(opts),
    }
}
