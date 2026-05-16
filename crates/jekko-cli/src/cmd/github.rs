//! `jekko github` — GitHub event helpers (passthrough to `xtask github-event`).
//!
//! Mirrors `packages/jekko/src/cli/cmd/github.ts`.

use std::process::Command;

use anyhow::{Context, Result};
use clap::Args;

use crate::cli::GlobalOpts;

#[derive(Args, Debug, Default)]
pub struct GithubArgs {
    /// Args forwarded verbatim to `cargo xtask github-event`.
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub forwarded: Vec<String>,
}

pub fn run(_global: &GlobalOpts, args: &GithubArgs) -> Result<()> {
    let mut cmd = Command::new("cargo");
    cmd.arg("xtask").arg("github-event");
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
