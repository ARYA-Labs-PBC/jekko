//! `jekko uninstall` — remove Jekko data directories.
//!
//! Mirrors `packages/jekko/src/cli/cmd/uninstall.ts`.

use anyhow::Result;
use clap::Args;

use crate::cli::GlobalOpts;

#[derive(Args, Debug, Default)]
pub struct UninstallArgs {
    /// Keep configuration files.
    #[arg(long, short = 'c')]
    pub keep_config: bool,
    /// Keep session data and snapshots.
    #[arg(long, short = 'd')]
    pub keep_data: bool,
    /// Print the removal plan without deleting anything.
    #[arg(long)]
    pub dry_run: bool,
    /// Skip confirmation prompts.
    #[arg(long, short = 'f')]
    pub force: bool,
}

pub fn run(_global: &GlobalOpts, args: &UninstallArgs) -> Result<()> {
    let home = std::env::var_os("HOME").map(std::path::PathBuf::from);
    let data = home.as_ref().map(|h| h.join(".jekko"));
    let config = home.as_ref().map(|h| h.join(".config").join("jekko"));

    eprintln!("Uninstall plan:");
    if !args.keep_data {
        eprintln!("  remove data dir: {}", describe_path(data.as_ref()));
    }
    if !args.keep_config {
        eprintln!("  remove config dir: {}", describe_path(config.as_ref()));
    }
    if args.dry_run {
        eprintln!("dry run: nothing was removed.");
        return Ok(());
    }
    if !args.force {
        eprintln!("re-run with --force to actually delete the listed directories.");
        return Ok(());
    }
    eprintln!("force-uninstall is currently a no-op; integration pending.");
    Ok(())
}

fn describe_path(path: Option<&std::path::PathBuf>) -> String {
    match path {
        Some(path) => path.display().to_string(),
        None => "HOME is unset; export HOME and rerun".to_string(),
    }
}
