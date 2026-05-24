use std::path::PathBuf;

use portable_pty::CommandBuilder;

use super::command_like::CommandLike;
use super::env::{default_allowlist, SandboxEnv};

/// Minimal sandbox policy applied to a runner-launched child process.
///
/// All fields default to "no restriction": an instance built with
/// `SandboxPolicy::default()` is functionally identical to not passing a
/// policy at all. That keeps the policy field opt-in and avoids regressing
/// runners that never wanted enforcement.
#[derive(Debug, Clone, Default)]
pub struct SandboxPolicy {
    /// Process working directory. Applied via `current_dir` /
    /// `CommandBuilder::cwd` before spawn.
    pub cwd: Option<PathBuf>,
    /// Paths the process is *expected* to read/write. **Advisory only on
    /// Linux** — there is no syscall-level enforcement on Linux yet
    /// (tracked as T-SANDBOX-FS-ISOLATION-LINUX, landlock / namespaces).
    /// **On macOS** these paths are translated into SBPL `(subpath ...)`
    /// allow rules and enforced by the macOS sandbox wrapper — see
    /// [`crate::engine::sandbox_macos`].
    pub allowed_paths: Vec<PathBuf>,
    /// Environment scrubbing mode. See [`SandboxEnv`].
    pub env: SandboxEnv,
    /// Whether the child is allowed to use the network. Defaults to `false`
    /// (deny).
    ///
    /// On macOS (T-SANDBOX-FS-ISOLATION-MACOS) this maps to the generated
    /// SBPL profile's `network-*` rules: `false` → `(deny network-*)`,
    /// `true` → `(allow network-*)`. On Linux this field is advisory only
    /// today; full network isolation is tracked as
    /// T-SANDBOX-FS-ISOLATION-LINUX. Kept last in the struct so existing
    /// literal initializers using positional-ish field order keep
    /// compiling against the new field.
    pub allow_net: bool,
}

impl SandboxPolicy {
    /// Convenience constructor for the "scrub env to a minimal allowlist,
    /// pin cwd to the current directory" profile used by `jekko sandbox
    /// --sandbox`.
    pub fn minimal_allowlist(cwd: PathBuf) -> Self {
        Self {
            cwd: Some(cwd),
            allowed_paths: Vec::new(),
            env: SandboxEnv::Allowlist(default_allowlist()),
            allow_net: false,
        }
    }

    /// Apply this policy to a `std::process::Command` (or
    /// `tokio::process::Command`, which exposes the same API). Returns the
    /// same mutable reference so callers can chain.
    pub fn apply_to_command<'a, C>(&self, cmd: &'a mut C) -> &'a mut C
    where
        C: CommandLike,
    {
        if let Some(dir) = self.cwd.as_ref() {
            cmd.policy_current_dir(dir);
        }
        match &self.env {
            SandboxEnv::Inherit => {}
            SandboxEnv::Empty => {
                cmd.policy_env_clear();
            }
            SandboxEnv::Allowlist(vars) => {
                cmd.policy_env_clear();
                for key in vars {
                    if let Ok(val) = std::env::var(key) {
                        cmd.policy_env(key, &val);
                    }
                }
            }
        }
        cmd
    }

    /// Apply this policy to a `portable_pty::CommandBuilder`. PTY builder
    /// has a per-variable `env(k, v)` API rather than `envs(iter)`, so it
    /// gets its own helper.
    pub fn apply_to_pty_builder(&self, builder: &mut CommandBuilder) {
        if let Some(dir) = self.cwd.as_ref() {
            builder.cwd(dir);
        }
        match &self.env {
            SandboxEnv::Inherit => {}
            SandboxEnv::Empty => {
                builder.env_clear();
            }
            SandboxEnv::Allowlist(vars) => {
                builder.env_clear();
                for key in vars {
                    if let Ok(val) = std::env::var(key) {
                        builder.env(key, val);
                    }
                }
            }
        }
    }
}
