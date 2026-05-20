/// Returns `true` if we're on macOS *and* the sandbox binary is callable on
/// the current `PATH`. On any other host this always returns `false` and
/// callers should fall back to advisory mode.
pub fn sandbox_available() -> bool {
    if !cfg!(target_os = "macos") {
        return false;
    }
    which_on_path(SANDBOX_EXEC_BIN).is_some()
}

/// If the macOS sandbox binary is available *and* `policy` has fs scope (either
/// `cwd` is set or `allowed_paths` is non-empty), return a new
/// (program, args) tuple of the form
/// `(sandbox_binary, ["-p", "<profile>", original_program, ...original_args])`.
/// Otherwise return the input unchanged.
///
/// A policy whose only non-default fields are `env` and/or `allow_net`
/// deliberately short-circuits to the unchanged input: env scrubbing is
/// already handled by [`SandboxPolicy::apply_to_command`], and a network-only
/// policy without any filesystem scope has nothing useful for the wrapper to
/// enforce.
pub fn wrap_command(
    program: &str,
    args: &[String],
    policy: &SandboxPolicy,
) -> (String, Vec<String>) {
    if !sandbox_available() {
        return (program.to_string(), args.to_vec());
    }
    if policy.cwd.is_none() && policy.allowed_paths.is_empty() {
        return (program.to_string(), args.to_vec());
    }

    let profile = render_profile(policy);
    let mut new_args: Vec<String> = Vec::with_capacity(args.len() + 3);
    new_args.push("-p".to_string());
    new_args.push(profile);
    new_args.push(program.to_string());
    new_args.extend(args.iter().cloned());

    (SANDBOX_EXEC_BIN.to_string(), new_args)
}

/// Minimal PATH lookup. Avoids dragging in `which` as a dependency — we only
/// need a single boolean answer and don't care about edge cases like Windows
/// `.exe` suffixes (we're on macOS or returning false).
fn which_on_path(bin: &str) -> Option<std::path::PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(bin);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}
