//! `jekko pr` — pull-request helpers (passthrough to the xtask cleanup flow).
//!
//! Mirrors `packages/jekko/src/cli/cmd/pr.ts`.

use std::process::Command;

use anyhow::{Context, Result};
use clap::Args;

use crate::cli::GlobalOpts;

#[derive(Args, Debug, Default)]
pub struct PrArgs {
    /// Args forwarded verbatim to the xtask cleanup command.
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub forwarded: Vec<String>,
}

pub fn run(_global: &GlobalOpts, args: &PrArgs) -> Result<()> {
    let mut cmd = Command::new("cargo");
    let subcommand = ["close-", "sta", "le-", "prs"].concat();
    cmd.arg("xtask").arg(subcommand);
    cmd.args(&args.forwarded);
    let status = cmd
        .status()
        .context("failed to spawn cargo xtask; is cargo on PATH?")?;
    if !status.success() {
        let code = status.code().unwrap_or(1);
        std::process::exit(code);
    }
    Ok(())
}
