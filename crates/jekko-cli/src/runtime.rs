//! Process bootstrapping helpers used by `main.rs`.
//!
//! Roughly mirrors `initializeCliRuntime` in `packages/jekko/src/index.ts`:
//! - parse log level
//! - install a `tracing_subscriber` env-filter
//! - set the `JEKKO_PURE` env marker
//! - switch the process CWD when `--cwd` is supplied
//! - surface the migration banner before opening the store
//!
//! Deep integrations (`Heap.start()`, the jnoccio heartbeat, the live db
//! progress bar) land with the C/D packets.

use std::env;
use std::path::Path;

use anyhow::{Context, Result};
use tracing_subscriber::EnvFilter;

use crate::cli::GlobalOpts;
use crate::migration_ui;

/// Configure tracing + env markers. Idempotent within a single process.
pub fn bootstrap(opts: &GlobalOpts) -> Result<()> {
    let level = match opts.log_level.as_deref() {
        Some(level) => canonicalize_level(level),
        None => default_level(),
    };

    let filter = match EnvFilter::try_from_default_env() {
        Ok(filter) => filter,
        Err(_) => EnvFilter::new(level.clone()),
    };

    let builder = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr);

    if opts.print_logs {
        let _ = builder.try_init();
    } else {
        // Still initialize, but route to stderr at WARN+ so unexpected errors
        // surface in CI logs without flooding the user's terminal.
        let quiet_filter = match EnvFilter::try_from_default_env() {
            Ok(filter) => filter,
            Err(_) => EnvFilter::new("warn"),
        };
        let _ = tracing_subscriber::fmt()
            .with_env_filter(quiet_filter)
            .with_writer(std::io::stderr)
            .try_init();
    }

    if opts.pure {
        // Match the TS marker so subprocesses can detect the flag.
        env::set_var("JEKKO_PURE", "1");
    }

    if let Some(cwd) = opts.cwd.as_deref() {
        switch_cwd(cwd)?;
    }

    Ok(())
}

fn switch_cwd(path: &Path) -> Result<()> {
    env::set_current_dir(path)
        .with_context(|| format!("failed to change working dir to {}", path.display()))?;
    Ok(())
}

fn default_level() -> String {
    "info".to_string()
}

fn canonicalize_level(input: &str) -> String {
    input.trim().to_ascii_lowercase()
}

/// Surface the migration progress banner described in `index.ts`. The store
/// crate currently runs migrations inline inside `Db::open` so we only have
/// "before" and "after" hooks to work with — see [`migration_ui`].
pub fn surface_migration_banner() {
    migration_ui::print_pre_migration();
    migration_ui::print_post_migration();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonicalize_lowercases() {
        assert_eq!(canonicalize_level("DEBUG"), "debug");
        assert_eq!(canonicalize_level("  Info  "), "info");
    }

    #[test]
    fn default_level_is_info() {
        assert_eq!(default_level(), "info");
    }

    #[test]
    fn bootstrap_accepts_empty_opts() {
        let opts = GlobalOpts::default();
        let _ = bootstrap(&opts);
    }
}
