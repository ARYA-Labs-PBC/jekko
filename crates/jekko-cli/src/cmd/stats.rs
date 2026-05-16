//! `jekko stats` — usage statistics.
//!
//! Mirrors `packages/jekko/src/cli/cmd/stats.ts`.

use anyhow::Result;
use clap::Args;

use crate::cli::GlobalOpts;

#[derive(Args, Debug, Default)]
pub struct StatsArgs {
    /// Output format. Either `table` or `json`.
    #[arg(long, default_value = "table")]
    pub format: String,
    /// Number of days to include. Defaults to 30.
    #[arg(long, short = 'd', default_value_t = 30)]
    pub days: u32,
}

pub fn run(_global: &GlobalOpts, args: &StatsArgs) -> Result<()> {
    eprintln!(
        "jekko stats: pending runtime integration (days={}, format={})",
        args.days, args.format
    );
    Ok(())
}
