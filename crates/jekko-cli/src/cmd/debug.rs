//! `jekko debug` — diagnostic utilities.
//!
//! Mirrors `packages/jekko/src/cli/cmd/debug/*.ts`.

use anyhow::Result;
use clap::{Args, Subcommand};

use crate::cli::GlobalOpts;

#[derive(Args, Debug)]
pub struct DebugArgs {
    #[command(subcommand)]
    pub command: DebugCommand,
}

#[derive(Subcommand, Debug)]
pub enum DebugCommand {
    /// Print an environment snapshot (versions, paths, feature flags).
    Snapshot,
    /// Print environment variables Jekko consumes.
    Env,
    /// Print resolved data / config / cache paths.
    Paths,
}

pub fn run(_global: &GlobalOpts, args: &DebugArgs) -> Result<()> {
    match &args.command {
        DebugCommand::Snapshot => snapshot(),
        DebugCommand::Env => env(),
        DebugCommand::Paths => paths(),
    }
}

fn snapshot() -> Result<()> {
    println!("jekko: {}", env!("CARGO_PKG_VERSION"));
    println!("os: {}", std::env::consts::OS);
    println!("arch: {}", std::env::consts::ARCH);
    let cwd = match std::env::current_dir() {
        Ok(path) => path.display().to_string(),
        Err(_) => "<unknown>".to_string(),
    };
    println!("cwd: {cwd}");
    Ok(())
}

fn env() -> Result<()> {
    for key in ["JEKKO", "JEKKO_PURE", "JEKKO_PID", "AGENT", "HOME"] {
        match std::env::var(key) {
            Ok(v) => println!("{key}={v}"),
            Err(_) => println!("{key}=<unset>"),
        }
    }
    Ok(())
}

fn paths() -> Result<()> {
    match std::env::var_os("HOME").map(std::path::PathBuf::from) {
        Some(home) => {
            println!("data: {}", home.join(".jekko").display());
            println!("config: {}", home.join(".config").join("jekko").display());
        }
        None => {
            println!("data: <no HOME>");
            println!("config: <no HOME>");
        }
    }
    Ok(())
}
