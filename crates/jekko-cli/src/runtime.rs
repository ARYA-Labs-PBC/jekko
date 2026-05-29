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
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use tracing_subscriber::EnvFilter;

use crate::cli::GlobalOpts;
use crate::migration_ui;

/// Configure tracing + env markers. Idempotent within a single process.
///
/// `directory` is the optional positional workspace argument from the CLI; it
/// takes precedence over the global `--cwd` flag when resolving the workspace
/// root the process is pinned to.
pub fn bootstrap(opts: &GlobalOpts, directory: Option<&Path>) -> Result<()> {
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

    // Pin the process cwd to the canonical workspace root so every downstream
    // path (session directory, git discovery, spawned tools) is scoped to the
    // launch folder. Jekko's infrastructure — the jnoccio-fusion gateway, the
    // key pool, and the database — resolves from `$JEKKO_HOME`/`$HOME` and the
    // executable's own location, so it is found regardless of this switch.
    if let Some(root) = resolve_workspace_root(directory, opts.cwd.as_deref())? {
        switch_cwd(&root)?;
    }

    Ok(())
}

/// Resolve the canonical workspace root from the positional `directory`
/// argument (highest precedence), then the global `--cwd` flag. Returns `None`
/// when neither is supplied so the existing process cwd is left untouched
/// (the common `cd <dir> && jekko` launch). The path is canonicalized so
/// symlinks resolve consistently for everything downstream.
fn resolve_workspace_root(
    directory: Option<&Path>,
    cwd_flag: Option<&Path>,
) -> Result<Option<PathBuf>> {
    let Some(path) = directory.or(cwd_flag) else {
        return Ok(None);
    };
    let canonical = std::fs::canonicalize(path)
        .with_context(|| format!("workspace directory does not exist: {}", path.display()))?;
    if !canonical.is_dir() {
        bail!("workspace path is not a directory: {}", canonical.display());
    }
    Ok(Some(canonical))
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
        let _ = bootstrap(&opts, None);
    }

    #[test]
    fn resolve_workspace_root_none_when_unset() {
        assert!(resolve_workspace_root(None, None).unwrap().is_none());
    }

    #[test]
    fn resolve_workspace_root_prefers_directory_over_cwd_flag() {
        let dir = tempfile::TempDir::new().unwrap();
        let other = tempfile::TempDir::new().unwrap();
        let resolved = resolve_workspace_root(Some(dir.path()), Some(other.path()))
            .unwrap()
            .unwrap();
        // Compare against the canonicalized form: macOS /tmp is a symlink.
        let expected = std::fs::canonicalize(dir.path()).unwrap();
        assert_eq!(resolved, expected);
    }

    #[test]
    fn resolve_workspace_root_falls_back_to_cwd_flag() {
        let other = tempfile::TempDir::new().unwrap();
        let resolved = resolve_workspace_root(None, Some(other.path()))
            .unwrap()
            .unwrap();
        let expected = std::fs::canonicalize(other.path()).unwrap();
        assert_eq!(resolved, expected);
    }

    #[test]
    fn resolve_workspace_root_errors_on_missing_dir() {
        let missing = Path::new("/nonexistent/jekko/workspace/path");
        assert!(resolve_workspace_root(Some(missing), None).is_err());
    }
}
