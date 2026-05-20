#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    use crate::engine::sandbox_policy::SandboxEnv;

    #[test]
    #[cfg(not(target_os = "macos"))]
    fn sandbox_available_returns_false_on_non_macos() {
        assert!(
            !sandbox_available(),
            "macOS sandbox binary is macOS-only; non-macOS hosts must report unavailable"
        );
    }

    #[test]
    fn render_profile_emits_version_and_deny_default() {
        let policy = SandboxPolicy::default();
        let p = render_profile(&policy);
        assert!(
            p.starts_with("(version 1)"),
            "SBPL profile must start with `(version 1)`; got:\n{p}"
        );
        assert!(
            p.contains("(deny default)"),
            "SBPL profile must include `(deny default)`; got:\n{p}"
        );
    }

    #[test]
    fn render_profile_includes_cwd_subpath() {
        let policy = SandboxPolicy {
            cwd: Some(PathBuf::from("/tmp/jekko-test-cwd")),
            allowed_paths: vec![],
            env: SandboxEnv::Inherit,
            allow_net: false,
        };
        let p = render_profile(&policy);
        assert!(
            p.contains("(subpath \"/tmp/jekko-test-cwd\")"),
            "cwd must appear as a (subpath ...) clause; got:\n{p}"
        );
    }

    #[test]
    fn render_profile_includes_allowed_paths() {
        let policy = SandboxPolicy {
            cwd: None,
            allowed_paths: vec![
                PathBuf::from("/tmp/jekko-allow-a"),
                PathBuf::from("/tmp/jekko-allow-b"),
            ],
            env: SandboxEnv::Inherit,
            allow_net: false,
        };
        let p = render_profile(&policy);
        assert!(
            p.contains("(subpath \"/tmp/jekko-allow-a\")"),
            "first allowed path missing; got:\n{p}"
        );
        assert!(
            p.contains("(subpath \"/tmp/jekko-allow-b\")"),
            "second allowed path missing; got:\n{p}"
        );
    }

    #[test]
    fn render_profile_omits_network_when_allow_net_false() {
        let policy = SandboxPolicy {
            cwd: Some(PathBuf::from("/tmp")),
            allowed_paths: vec![],
            env: SandboxEnv::Inherit,
            allow_net: false,
        };
        let p = render_profile(&policy);
        assert!(
            p.contains("(deny network*)"),
            "allow_net=false must emit (deny network*); got:\n{p}"
        );
        assert!(
            !p.contains("(allow network*)"),
            "allow_net=false must NOT emit (allow network*); got:\n{p}"
        );
    }

    #[test]
    fn render_profile_includes_network_when_allow_net_true() {
        let policy = SandboxPolicy {
            cwd: Some(PathBuf::from("/tmp")),
            allowed_paths: vec![],
            env: SandboxEnv::Inherit,
            allow_net: true,
        };
        let p = render_profile(&policy);
        assert!(
            p.contains("(allow network*)"),
            "allow_net=true must emit (allow network*); got:\n{p}"
        );
        assert!(
            !p.contains("(deny network*)"),
            "allow_net=true must NOT emit (deny network*); got:\n{p}"
        );
    }

    #[test]
    fn render_profile_includes_system_essentials() {
        let p = render_profile(&SandboxPolicy::default());
        for needle in [
            "(subpath \"/usr/lib\")",
            "(subpath \"/System/Library\")",
            "(literal \"/dev/null\")",
            "(allow mach-lookup)",
            "(allow process-exec*)",
        ] {
            assert!(
                p.contains(needle),
                "missing essential rule {needle}; got:\n{p}"
            );
        }
    }

    #[test]
    fn wrap_command_returns_original_when_policy_empty() {
        let policy = SandboxPolicy::default();
        let (prog, args) = wrap_command("/bin/echo", &["hello".to_string()], &policy);
        assert_eq!(prog, "/bin/echo");
        assert_eq!(args, vec!["hello".to_string()]);
    }

    #[test]
    fn wrap_command_returns_original_when_only_env_set() {
        let policy = SandboxPolicy {
            cwd: None,
            allowed_paths: vec![],
            env: SandboxEnv::Empty,
            allow_net: false,
        };
        let (prog, _args) = wrap_command("/bin/echo", &[], &policy);
        assert_eq!(prog, "/bin/echo");
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn wrap_command_prepends_sandbox_exec_on_macos() {
        if !sandbox_available() {
            eprintln!("skipping: macOS sandbox binary not on PATH");
            return;
        }
        let policy = SandboxPolicy {
            cwd: Some(PathBuf::from("/tmp")),
            allowed_paths: vec![PathBuf::from("/tmp/jekko-test-allow")],
            env: SandboxEnv::Inherit,
            allow_net: false,
        };
        let (prog, args) = wrap_command("/bin/echo", &["x".to_string()], &policy);
        assert_eq!(prog, SANDBOX_EXEC_BIN);
        assert_eq!(args.first().map(String::as_str), Some("-p"));
        assert!(
            args.get(1)
                .map(|s| s.contains("(version 1)"))
                .unwrap_or(false),
            "second arg must be the SBPL profile; got args={args:?}"
        );
        assert_eq!(args.get(2).map(String::as_str), Some("/bin/echo"));
        assert_eq!(args.get(3).map(String::as_str), Some("x"));
    }

    #[test]
    fn push_subpath_escapes_embedded_quotes() {
        let mut s = String::new();
        push_subpath(&mut s, Path::new("/tmp/has\"quote"));
        assert!(
            s.contains("(subpath \"/tmp/has\\\"quote\")"),
            "embedded quote not escaped; got: {s}"
        );
    }

    #[test]
    fn push_subpath_skips_empty_path() {
        let mut s = String::new();
        push_subpath(&mut s, Path::new(""));
        assert!(s.is_empty(), "empty path must not emit a subpath clause");
    }
}
