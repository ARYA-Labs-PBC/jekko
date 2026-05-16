//! `jekko db` — database tools.
//!
//! Mirrors `packages/jekko/src/cli/cmd/db.ts`. Uses `jekko-store`.

use anyhow::{Context, Result};
use clap::{Args, Subcommand};

use crate::cli::GlobalOpts;

#[derive(Args, Debug)]
pub struct DbArgs {
    #[command(subcommand)]
    pub command: DbCommand,
}

#[derive(Subcommand, Debug)]
pub enum DbCommand {
    /// Run pending migrations.
    Migrate,
    /// Print migration status (applied count + pending count).
    Status,
    /// Print the resolved database path.
    Path,
}

pub fn run(_global: &GlobalOpts, args: &DbArgs) -> Result<()> {
    match &args.command {
        DbCommand::Migrate => migrate(),
        DbCommand::Status => status(),
        DbCommand::Path => path(),
    }
}

fn db_path() -> std::path::PathBuf {
    match std::env::var_os("HOME") {
        Some(home) => std::path::PathBuf::from(home)
            .join(".jekko")
            .join("jekko.db"),
        None => std::path::PathBuf::from("jekko.db"),
    }
}

fn migrate() -> Result<()> {
    let path = db_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("create db dir at {}", parent.display()))?;
    }
    let total = jekko_store::db::embedded_migration_count();
    eprintln!("Migrating database ({total} embedded migrations)...");
    let _db =
        jekko_store::Db::open(&path).with_context(|| format!("open db at {}", path.display()))?;
    println!("ok ({} migrations embedded)", total);
    Ok(())
}

fn status() -> Result<()> {
    let total = jekko_store::db::embedded_migration_count();
    println!("embedded: {total}");
    for (name, hash) in jekko_store::db::embedded_migrations() {
        println!("  {name} {hash}");
    }
    Ok(())
}

fn path() -> Result<()> {
    println!("{}", db_path().display());
    Ok(())
}
