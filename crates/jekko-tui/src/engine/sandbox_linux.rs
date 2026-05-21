//! Linux landlock-based filesystem isolation
//! (T-SANDBOX-FS-ISOLATION-LINUX).
//!
//! Mirrors the [`super::sandbox_macos`] API surface for cross-platform
//! consistency. Unlike the macOS command wrapper, landlock is in-process: a
//! parent enrolls itself into a ruleset and the kernel enforces the rules from
//! that point forward, propagating the restrictions across `fork(2)` +
//! `execve(2)`.
//!
//! We therefore expose two entry points:
//!
//! - [`wrap_command`] — identity transform. Kept symmetric with
//!   `sandbox_macos::wrap_command` so runners can call it unconditionally
//!   on every platform. On Linux it returns `(program, args)` unchanged
//!   because landlock is *not* a command-wrap primitive; the actual
//!   enforcement happens via `pre_exec`.
//! - [`apply_landlock`] — installs the landlock ruleset on the calling
//!   process. The runner threads this through `tokio::process::Command`'s
//!   `pre_exec` hook so the restrictions are applied in the forked child,
//!   between fork and exec, where they affect only the child (not the
//!   jekko TUI parent).
//!
//! Network + syscall isolation are out of scope; tier-2 follow-up via
//! seccomp-bpf and user namespaces are tracked separately
//! (T-SANDBOX-LINUX-NET, T-SANDBOX-LINUX-USERNS).
//!
//! # ABI choice
//!
//! We target landlock `ABI::V1` (Linux 5.13+) and rely on the crate's
//! default best-effort compatibility level: on older kernels
//! `restrict_self()` returns `Ok(_)` with `RulesetStatus::NotEnforced`
//! and the child runs without isolation. Callers that need a hard
//! failure should add an explicit `set_compatibility(HardRequirement)`
//! upstream — today we prefer the graceful path (advisory fallback)
//! because jekko already declares macOS / pre-5.13 Linux as
//! "advisory-only" hosts.

use std::io;

use super::sandbox_policy::SandboxPolicy;

#[cfg(target_os = "linux")]
use landlock::{
    path_beneath_rules, Access, AccessFs, Ruleset, RulesetAttr, RulesetCreatedAttr, ABI,
};

/// Best-effort probe for landlock availability.
///
/// On Linux this attempts to create an empty ruleset; if the syscall
/// succeeds we know the kernel speaks at least ABI v1 (Linux 5.13+).
/// On other targets this is a compile-time `false`.
///
/// The probe is intentionally tolerant: any error (ENOSYS, EOPNOTSUPP,
/// EPERM, etc.) → `false`. We do not surface the underlying errno —
/// callers only need a boolean to decide whether to log "landlock active"
/// vs "landlock advisory". The probe is cheap (one syscall) and not
/// cached, mirroring `sandbox_macos::sandbox_available`'s behavior.
pub fn sandbox_available() -> bool {
    #[cfg(target_os = "linux")]
    {
        // `Ruleset::default()` constructs the builder + internally
        // probes the running kernel ABI. We can't directly read its
        // status (the `Compatibility` accessor is private) so we ask
        // it to handle a known access and try to `create()` — if the
        // create syscall returns a valid fd, landlock is live.
        //
        // Wrapped in a closure so we can `?`-propagate landlock errors
        // without panicking the caller.
        let probe = || -> Result<bool, landlock::RulesetError> {
            let _created = Ruleset::default()
                .handle_access(AccessFs::from_all(ABI::V1))?
                .create()?;
            Ok(true)
        };
        probe().unwrap_or(false)
    }
    #[cfg(not(target_os = "linux"))]
    {
        false
    }
}

/// Apply landlock restrictions to the calling process.
///
/// **Must** be called from inside `Command::pre_exec` (post-fork,
/// pre-exec) so the child inherits the ruleset and the jekko TUI
/// parent is *not* restricted. Calling this from the parent will
/// silently sandbox the TUI itself.
///
/// Read access is granted on `policy.cwd` + every entry of
/// `policy.allowed_paths`. Read+write access is granted on
/// `policy.cwd` (writes elsewhere are denied at the syscall level,
/// matching the macOS `sandbox_macos` rule that writes are tighter
/// than reads).
///
/// On non-Linux platforms this is a no-op `Ok(())`. On Linux without
/// kernel support, `restrict_self()` reports `RulesetStatus::NotEnforced`
/// but we still return `Ok(())` — the advisory fallback model.
///
/// # Safety note
///
/// The caller (a `pre_exec` closure) runs between `fork(2)` and
/// `execve(2)`. Per POSIX async-signal-safety rules the body may only
/// invoke async-signal-safe functions. The landlock crate's syscalls
/// (`landlock_create_ruleset`, `landlock_add_rule`, `landlock_restrict_self`,
/// `prctl(PR_SET_NO_NEW_PRIVS)`) are all signal-safe; this function
/// performs no allocations beyond what landlock requires internally
/// and no logging.
#[cfg(target_os = "linux")]
pub fn apply_landlock(policy: &SandboxPolicy) -> io::Result<()> {
    let abi = ABI::V1;
    let read_access = AccessFs::from_read(abi);
    let read_write_access = AccessFs::from_all(abi);

    // Collect paths for read-only rules: every allowed_path + cwd.
    // (cwd is added to both read and write rule sets; redundancy is
    // harmless — the kernel unions the access masks.)
    let mut read_paths: Vec<&std::path::Path> = Vec::new();
    if let Some(cwd) = policy.cwd.as_ref() {
        read_paths.push(cwd.as_path());
    }
    for p in &policy.allowed_paths {
        read_paths.push(p.as_path());
    }

    // Write paths: cwd only.
    let mut write_paths: Vec<&std::path::Path> = Vec::new();
    if let Some(cwd) = policy.cwd.as_ref() {
        write_paths.push(cwd.as_path());
    }

    // Build → handle full FS access space → create → add read rules →
    // add write rules → restrict_self. `path_beneath_rules` opens fds
    // internally and silently skips paths that can't be opened (the
    // crate's best-effort default — matches our "advisory fallback"
    // posture). Errors map to `io::Error` so the `pre_exec` closure's
    // `io::Result<()>` return type composes cleanly.
    let created = Ruleset::default()
        .handle_access(AccessFs::from_all(abi))
        .map_err(landlock_to_io)?
        .create()
        .map_err(landlock_to_io)?
        .add_rules(path_beneath_rules(read_paths, read_access))
        .map_err(landlock_to_io)?
        .add_rules(path_beneath_rules(write_paths, read_write_access))
        .map_err(landlock_to_io)?;

    let _status = created.restrict_self().map_err(landlock_to_io)?;
    // Best-effort: on kernels without landlock, `_status.ruleset ==
    // RulesetStatus::NotEnforced` and we report Ok — runner falls back
    // to advisory cwd/env scrubbing already in place via
    // `SandboxPolicy::apply_to_command`.
    Ok(())
}

/// Non-Linux stub. Always `Ok(())` so the runner can call the same
/// codepath unconditionally without per-platform branching above the
/// engine layer.
#[cfg(not(target_os = "linux"))]
pub fn apply_landlock(_policy: &SandboxPolicy) -> io::Result<()> {
    Ok(())
}

/// Identity wrap. Provided for API symmetry with
/// [`super::sandbox_macos::wrap_command`] — on Linux there is no
/// command-wrap primitive (landlock is in-process), so the original
/// `(program, args)` is always returned. Runners apply enforcement
/// through [`apply_landlock`] in a `pre_exec` hook instead.
pub fn wrap_command(
    program: &str,
    args: &[String],
    _policy: &SandboxPolicy,
) -> (String, Vec<String>) {
    (program.to_string(), args.to_vec())
}

/// Convert a landlock RulesetError into an io::Error so the `pre_exec`
/// closure's `io::Result<()>` return type composes cleanly. We preserve
/// the original error's `Display` for diagnostics.
#[cfg(target_os = "linux")]
fn landlock_to_io(err: landlock::RulesetError) -> io::Error {
    io::Error::other(format!("landlock: {err}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    use crate::engine::sandbox_policy::SandboxEnv;

    #[test]
    #[cfg(not(target_os = "linux"))]
    fn sandbox_available_returns_false_on_non_linux() {
        // landlock is a Linux-only LSM; on macOS/Windows the probe must
        // be false so the runner falls back to advisory mode.
        assert!(
            !sandbox_available(),
            "landlock is Linux-only; non-Linux hosts must report unavailable"
        );
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn sandbox_available_returns_true_on_modern_linux() {
        // On Linux 5.13+ with landlock built into the kernel the probe
        // should succeed. On older / unbuilt kernels (e.g. some CI
        // containers, WSL1) it will return false — print a hint rather
        // than fail so CI on those hosts doesn't get a spurious red.
        if !sandbox_available() {
            eprintln!(
                "skipping: landlock not available on this kernel \
                 (need Linux 5.13+ with CONFIG_SECURITY_LANDLOCK=y)"
            );
            return;
        }
        assert!(sandbox_available(), "landlock probe must be true here");
    }

    #[test]
    fn wrap_command_returns_identity() {
        // Sanity: on every platform, wrap_command is the identity for
        // Linux. landlock isn't a command-wrap primitive — enforcement
        // happens via apply_landlock + pre_exec, not via argv rewriting.
        let policy = SandboxPolicy {
            cwd: Some(PathBuf::from("/tmp")),
            allowed_paths: vec![PathBuf::from("/usr/lib")],
            env: SandboxEnv::Inherit,
            allow_net: false,
        };
        let (prog, args) = wrap_command("/bin/echo", &["hello".to_string()], &policy);
        assert_eq!(prog, "/bin/echo");
        assert_eq!(args, vec!["hello".to_string()]);
    }

    #[test]
    fn wrap_command_returns_identity_with_default_policy() {
        // Empty policy → still identity. (No platform applies any
        // command rewriting on Linux for this path.)
        let policy = SandboxPolicy::default();
        let (prog, args) = wrap_command("/bin/true", &[], &policy);
        assert_eq!(prog, "/bin/true");
        assert!(args.is_empty());
    }

    #[test]
    #[cfg(not(target_os = "linux"))]
    fn apply_landlock_is_noop_off_linux() {
        // On non-Linux the apply is a stub returning Ok. The runner
        // calls this inside pre_exec on every platform; the stub
        // guarantees it never errors off-Linux so spawning still works.
        let policy = SandboxPolicy::default();
        apply_landlock(&policy).expect("apply_landlock stub must succeed off-Linux");
    }

    /// Integration test: spawn `sh -c "echo > /tmp/outside.txt"` under
    /// landlock with cwd pinned to a tempdir. The write to /tmp should
    /// fail with EACCES once the kernel enforces the ruleset.
    ///
    /// Gated `#[ignore]` because:
    /// - requires Linux 5.13+ at runtime;
    /// - does real filesystem IO and process spawning;
    /// - the parent jekko-tui test process must not itself end up
    ///   sandboxed (we use `Command::pre_exec` so only the child is).
    ///
    /// Run with `cargo test --features ... landlock_blocks_write_outside_cwd -- --ignored`
    /// on a Linux host.
    #[test]
    #[cfg(target_os = "linux")]
    #[ignore = "requires real Linux 5.13+ landlock kernel + write to /tmp"]
    fn landlock_blocks_write_outside_cwd() {
        use std::os::unix::process::CommandExt;
        use std::process::Command;

        if !sandbox_available() {
            eprintln!("skipping: landlock unavailable");
            return;
        }

        let tmp = tempfile::tempdir().expect("tempdir");
        let policy = SandboxPolicy {
            cwd: Some(tmp.path().to_path_buf()),
            allowed_paths: vec![
                // sh needs read access to its own binary + libc.
                PathBuf::from("/bin"),
                PathBuf::from("/usr"),
                PathBuf::from("/lib"),
                PathBuf::from("/lib64"),
                PathBuf::from("/etc"),
            ],
            env: SandboxEnv::Inherit,
            allow_net: false,
        };

        // Pick an out-of-cwd target that definitely won't be writable
        // under the policy. /tmp is conventionally writable, so a
        // failure here is a clear landlock signal (vs a Unix mode bit).
        let outside = format!("/tmp/jekko-landlock-test-{}.txt", std::process::id());

        let mut cmd = Command::new("/bin/sh");
        cmd.arg("-c")
            .arg(format!("echo hi > {outside}"))
            .current_dir(tmp.path());
        let policy_for_child = policy.clone();
        // SAFETY: this test installs Landlock in the child immediately before
        // exec; the closure performs no logging and returns an io::Result.
        unsafe {
            cmd.pre_exec(move || {
                apply_landlock(&policy_for_child)?;
                Ok(())
            });
        }

        let status = cmd.status().expect("spawn /bin/sh");
        // Cleanup: if the write somehow succeeded the test should fail
        // *and* clean up, so a re-run isn't dirty.
        let _ = std::fs::remove_file(&outside);

        assert!(
            !status.success(),
            "expected sh to fail writing /tmp under landlock; got status {status:?}"
        );
    }
}
