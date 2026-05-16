//! `jekko upgrade` — check / apply a Jekko release.
//!
//! Mirrors `packages/jekko/src/cli/cmd/upgrade.ts`.

use anyhow::Result;
use clap::Args;

use crate::cli::GlobalOpts;

#[derive(Args, Debug, Default)]
pub struct UpgradeArgs {
    /// Target version (e.g. `0.1.48`). Defaults to the latest release.
    pub target: Option<String>,
    /// Installation method override (`curl`, `npm`, `brew`, ...).
    #[arg(long, short = 'm')]
    pub method: Option<String>,
    /// Repair an existing installation instead of skipping when up to date.
    #[arg(long)]
    pub repair: bool,
    /// Only check for a new version, do not install.
    #[arg(long)]
    pub check: bool,
}

pub fn run(_global: &GlobalOpts, args: &UpgradeArgs) -> Result<()> {
    if args.check {
        eprintln!("jekko upgrade --check: pending installer integration");
        return Ok(());
    }
    eprintln!(
        "jekko upgrade target={:?} method={:?}: pending installer integration",
        args.target, args.method
    );
    Ok(())
}
