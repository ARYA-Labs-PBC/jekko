//! `jekko import` — import sessions from a backup file.
//!
//! Mirrors `packages/jekko/src/cli/cmd/import.ts`.

use std::path::PathBuf;

use anyhow::Result;
use clap::Args;

use crate::cli::GlobalOpts;

#[derive(Args, Debug)]
pub struct ImportArgs {
    /// Input file (`.json` or `.zst`).
    pub input: PathBuf,
    /// Overwrite existing sessions when ids collide.
    #[arg(long)]
    pub overwrite: bool,
}

pub fn run(_global: &GlobalOpts, args: &ImportArgs) -> Result<()> {
    eprintln!(
        "jekko import {}: pending runtime integration",
        args.input.display()
    );
    Ok(())
}
