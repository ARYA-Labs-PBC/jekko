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

use std::path::PathBuf;

use portable_pty::CommandBuilder;

/// What to do with the child's environment.
///
/// - `Inherit`: pass through the current process environment (legacy
///   behavior — what runners did before this module existed).
/// - `Allowlist(vars)`: clear the env and re-set only the named vars,
///   pulling their values from the current process. Names that aren't set
///   in the parent are silently skipped.
/// - `Empty`: clear the env entirely. The child sees only what the runner
///   itself sets (today: nothing).
#[derive(Debug, Clone, Default)]
pub enum SandboxEnv {
    #[default]
    Inherit,
    Allowlist(Vec<String>),
    Empty,
}

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

/// Default allowlist for the `minimal_allowlist` profile. Picked so the
/// child still has the basics it needs to find binaries / render output:
///
/// - `PATH`: locate the program.
/// - `HOME`: user-config lookups.
/// - `USER` / `LOGNAME`: identity for tools that respect it.
/// - `TERM`: terminal capability hints for PTY children.
/// - `LANG` / `LC_ALL`: locale for utf-8 output.
pub fn default_allowlist() -> Vec<String> {
    vec![
        "PATH".to_string(),
        "HOME".to_string(),
        "USER".to_string(),
        "LOGNAME".to_string(),
        "TERM".to_string(),
        "LANG".to_string(),
        "LC_ALL".to_string(),
    ]
}

/// Trait so `apply_to_command` works with both `std::process::Command` and
/// `tokio::process::Command` without dragging in conditional generics.
/// Methods are prefixed with `policy_` to avoid clashing with the underlying
/// inherent methods (which `apply_to_command` *also* needs to call via these
/// trait impls).
pub trait CommandLike {
    fn policy_current_dir(&mut self, dir: &std::path::Path);
    fn policy_env_clear(&mut self);
    fn policy_env(&mut self, key: &str, value: &str);
}

impl CommandLike for std::process::Command {
    fn policy_current_dir(&mut self, dir: &std::path::Path) {
        self.current_dir(dir);
    }
    fn policy_env_clear(&mut self) {
        self.env_clear();
    }
    fn policy_env(&mut self, key: &str, value: &str) {
        self.env(key, value);
    }
}

impl CommandLike for tokio::process::Command {
    fn policy_current_dir(&mut self, dir: &std::path::Path) {
        self.current_dir(dir);
    }
    fn policy_env_clear(&mut self) {
        self.env_clear();
    }
    fn policy_env(&mut self, key: &str, value: &str) {
        self.env(key, value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_policy_is_unrestricted() {
        let p = SandboxPolicy::default();
        assert!(p.cwd.is_none(), "default cwd must be None (inherit)");
        assert!(
            p.allowed_paths.is_empty(),
            "default allowed_paths must be empty"
        );
        assert!(
            matches!(p.env, SandboxEnv::Inherit),
            "default env must be Inherit"
        );
        assert!(
            !p.allow_net,
            "default allow_net must be false (deny network on sandboxed runs)"
        );
    }

    #[test]
    fn minimal_allowlist_pins_cwd_and_sets_allowlist_env() {
        let p = SandboxPolicy::minimal_allowlist(PathBuf::from("/tmp"));
        assert_eq!(p.cwd.as_deref(), Some(std::path::Path::new("/tmp")));
        match p.env {
            SandboxEnv::Allowlist(vars) => {
                assert!(vars.contains(&"PATH".to_string()));
                assert!(vars.contains(&"HOME".to_string()));
                assert!(vars.contains(&"TERM".to_string()));
            }
            _ => panic!("expected Allowlist env"),
        }
    }

    #[test]
    fn apply_policy_sets_cwd_on_std_command() {
        // No direct getter on `std::process::Command` for `cwd` on stable
        // Rust before the `get_current_dir` API stabilized, so we verify by
        // spawning a subshell that prints `pwd` and observing the output.
        let tmp = tempfile::tempdir().expect("tempdir");
        let mut cmd = std::process::Command::new("/bin/sh");
        let policy = SandboxPolicy {
            cwd: Some(tmp.path().to_path_buf()),
            allowed_paths: vec![],
            env: SandboxEnv::Inherit,
            allow_net: false,
        };
        policy.apply_to_command(&mut cmd);
        cmd.arg("-c").arg("pwd");
        let out = cmd.output().expect("spawn /bin/sh -c pwd");
        let stdout = String::from_utf8_lossy(&out.stdout);
        // tempdir paths on macOS resolve through /private; canonicalize both
        // sides so the comparison survives the /var ↔ /private/var symlink.
        let observed = std::fs::canonicalize(stdout.trim()).expect("canonicalize stdout pwd");
        let expected = std::fs::canonicalize(tmp.path()).expect("canonicalize tmp");
        assert_eq!(observed, expected);
    }

    #[test]
    fn sandbox_env_empty_clears_env() {
        // Set a marker var that should NOT survive the scrub.
        std::env::set_var("JEKKO_SANDBOX_TEST_MARKER", "leaked");
        let mut cmd = std::process::Command::new("/usr/bin/env");
        let policy = SandboxPolicy {
            cwd: None,
            allowed_paths: vec![],
            env: SandboxEnv::Empty,
            allow_net: false,
        };
        policy.apply_to_command(&mut cmd);
        let out = cmd.output().expect("spawn /usr/bin/env");
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(
            !stdout.contains("JEKKO_SANDBOX_TEST_MARKER"),
            "Empty env policy leaked marker: {stdout:?}"
        );
        std::env::remove_var("JEKKO_SANDBOX_TEST_MARKER");
    }

    #[test]
    fn sandbox_env_allowlist_passes_only_named_vars() {
        std::env::set_var("JEKKO_SANDBOX_ALLOWED_VAR", "allowed_value");
        std::env::set_var("JEKKO_SANDBOX_BLOCKED_VAR", "blocked_value");
        let mut cmd = std::process::Command::new("/usr/bin/env");
        let policy = SandboxPolicy {
            cwd: None,
            allowed_paths: vec![],
            env: SandboxEnv::Allowlist(vec!["JEKKO_SANDBOX_ALLOWED_VAR".to_string()]),
            allow_net: false,
        };
        policy.apply_to_command(&mut cmd);
        let out = cmd.output().expect("spawn /usr/bin/env");
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(
            stdout.contains("JEKKO_SANDBOX_ALLOWED_VAR=allowed_value"),
            "allowlist dropped allowed var: {stdout:?}"
        );
        assert!(
            !stdout.contains("JEKKO_SANDBOX_BLOCKED_VAR"),
            "allowlist leaked unlisted var: {stdout:?}"
        );
        std::env::remove_var("JEKKO_SANDBOX_ALLOWED_VAR");
        std::env::remove_var("JEKKO_SANDBOX_BLOCKED_VAR");
    }

    #[test]
    fn sandbox_env_allowlist_skips_unset_vars_silently() {
        // Name a var that is guaranteed not set; policy must not panic and
        // must still clear the rest of the env.
        let mut cmd = std::process::Command::new("/usr/bin/env");
        let policy = SandboxPolicy {
            cwd: None,
            allowed_paths: vec![],
            env: SandboxEnv::Allowlist(vec!["JEKKO_DEFINITELY_NOT_SET_XYZ".to_string()]),
            allow_net: false,
        };
        policy.apply_to_command(&mut cmd);
        let out = cmd.output().expect("spawn /usr/bin/env");
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(
            !stdout.contains("JEKKO_DEFINITELY_NOT_SET_XYZ"),
            "unset allowlisted var should not appear: {stdout:?}"
        );
    }

    #[test]
    fn apply_to_pty_builder_sets_cwd() {
        let mut b = CommandBuilder::new("/bin/echo");
        let p = SandboxPolicy {
            cwd: Some(PathBuf::from("/tmp")),
            allowed_paths: vec![],
            env: SandboxEnv::Inherit,
            allow_net: false,
        };
        p.apply_to_pty_builder(&mut b);
        let got = b.get_cwd().map(|s| s.to_string_lossy().to_string());
        assert_eq!(got.as_deref(), Some("/tmp"));
    }

    #[test]
    fn apply_to_pty_builder_clears_env_for_empty() {
        std::env::set_var("JEKKO_PTY_SCRUB_MARKER", "x");
        let mut b = CommandBuilder::new("/bin/echo");
        let p = SandboxPolicy {
            cwd: None,
            allowed_paths: vec![],
            env: SandboxEnv::Empty,
            allow_net: false,
        };
        p.apply_to_pty_builder(&mut b);
        // After env_clear with no allowlist, no extra env entries should be set.
        let extra: Vec<_> = b.iter_extra_env_as_str().collect();
        assert!(
            extra.is_empty(),
            "expected no extra env after Empty policy, got {extra:?}"
        );
        std::env::remove_var("JEKKO_PTY_SCRUB_MARKER");
    }

    #[test]
    fn apply_to_pty_builder_allowlist_passes_named() {
        std::env::set_var("JEKKO_PTY_ALLOWED", "yes");
        let mut b = CommandBuilder::new("/bin/echo");
        let p = SandboxPolicy {
            cwd: None,
            allowed_paths: vec![],
            env: SandboxEnv::Allowlist(vec!["JEKKO_PTY_ALLOWED".to_string()]),
            allow_net: false,
        };
        p.apply_to_pty_builder(&mut b);
        let extra: Vec<(String, String)> = b
            .iter_extra_env_as_str()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        assert!(
            extra
                .iter()
                .any(|(k, v)| k == "JEKKO_PTY_ALLOWED" && v == "yes"),
            "expected allowlisted var in PTY env, got {extra:?}"
        );
        std::env::remove_var("JEKKO_PTY_ALLOWED");
    }
}
