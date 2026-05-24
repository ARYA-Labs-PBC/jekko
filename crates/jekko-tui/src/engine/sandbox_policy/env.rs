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
