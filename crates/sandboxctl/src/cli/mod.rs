//! Subcommand dispatch. Each subcommand lives in its own file and exposes
//! `Args` + `run()`.

use std::path::PathBuf;

pub mod compile_spec;
pub mod create;
pub mod destroy;
pub mod export;
pub mod list;
pub mod run;
pub mod status;
pub mod validate;

/// Default sandbox root used by status/export/destroy/list when no override
/// is provided. Honors `SANDBOXCTL_ROOT` for tests and CI overrides.
pub(crate) fn status_root() -> PathBuf {
    if let Some(env) = std::env::var_os("SANDBOXCTL_ROOT") {
        return PathBuf::from(env);
    }
    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home).join(".local/share/agent-sandboxes");
    }
    PathBuf::from(".sandbox")
}
