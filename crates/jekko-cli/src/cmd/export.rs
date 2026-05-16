//! `jekko export` — export sessions to a backup file.
//!
//! Mirrors `packages/jekko/src/cli/cmd/export.ts`.

use std::path::PathBuf;

use anyhow::Result;
use clap::Args;

use crate::cli::GlobalOpts;

#[derive(Args, Debug)]
pub struct ExportArgs {
    /// Session id to export. When omitted, all sessions are exported.
    pub session_id: Option<String>,
    /// Output file path.
    #[arg(long, short = 'o', value_name = "PATH")]
    pub out: Option<PathBuf>,
    /// Use ZSTD compression for the output file.
    #[arg(long)]
    pub compress: bool,
}

pub fn run(_global: &GlobalOpts, args: &ExportArgs) -> Result<()> {
    if let Some(id) = args.session_id.as_deref() {
        eprintln!("jekko export {id}: pending runtime integration");
    } else {
        eprintln!("jekko export: pending runtime integration");
    }
    Ok(())
}
