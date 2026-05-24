use std::path::PathBuf;

use portable_pty::CommandBuilder;

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
