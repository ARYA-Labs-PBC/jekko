//! Minimal sandbox enforcement for the engine runners (T-SANDBOX-ENF).
//!
//! Codex's 01:15Z scout flagged that `jekko sandbox` advertised isolation but
//! the runners spawned raw processes with no cwd / env / path policy. This
//! module is the smallest credible enforcement layer:
//!
//! - `cwd`: chdir the child before exec so it cannot accidentally inherit the
//!   parent shell's working directory.
//! - `env`: scrub the environment to a documented set (full clear, or a named
//!   allowlist). Defaults to inherit so callers that don't opt in see no
//!   behavior change.
//! - `allowed_paths`: advisory only in v1. Full filesystem isolation would
//!   require Linux namespaces / the macOS sandbox wrapper, both of which are
//!   out of scope here. Documented as such so docs match reality.
//!
//! Apply via [`SandboxPolicy::apply_to_command`] (for `std::process::Command`
//! / `tokio::process::Command`) or [`SandboxPolicy::apply_to_pty_builder`]
//! (for `portable_pty::CommandBuilder`). The two helpers exist because the
//! PTY builder has a slightly different env API (`env_clear` + `env(k, v)`
//! per-var, no `envs(iter)` shortcut) and is used by the PTY runner.
//!
//! Default (`SandboxPolicy::default()` with `SandboxEnv::Inherit`,
//! `cwd = None`, empty `allowed_paths`) is a no-op: callers that don't pass
//! a policy or pass `None` see the legacy unrestricted runner behavior.

mod command_like;
mod env;
mod policy;

#[cfg(test)]
mod tests;

pub use command_like::CommandLike;
pub use env::{default_allowlist, SandboxEnv};
pub use policy::SandboxPolicy;
