use std::env;
use std::path::Path;
use std::process::Command;

/// Snapshot of the workspace data shown in the splash subtitle. Cheap to
/// build via [`SplashContext::detect`]; read once in the runtime, then pass
/// into `render_splash` each frame.
#[derive(Clone, Debug)]
pub struct SplashContext {
    /// Crate version (e.g. `"0.1.0"`). Defaults to `CARGO_PKG_VERSION`;
    /// `JEKKO_VERSION_OVERRIDE` wins for downstream packaging.
    pub version: String,
    /// `~`-relative cwd display (e.g. `"~/code/jekko"`). Falls back to the
    /// absolute path when `$HOME` is not a prefix of the cwd.
    pub cwd: String,
    /// Active git branch when the cwd is inside a repo and `git` is on PATH.
    pub branch: Option<String>,
}

impl SplashContext {
    /// Build context from the environment. Never returns `Err`; missing data
    /// degrades gracefully.
    pub fn detect() -> Self {
        let version = env::var("JEKKO_VERSION_OVERRIDE")
            .unwrap_or_else(|_| env!("CARGO_PKG_VERSION").to_string());
        let cwd = current_cwd_display();
        let branch = current_git_branch();
        Self {
            version,
            cwd,
            branch,
        }
    }
}

fn current_cwd_display() -> String {
    let cwd = env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf());
    if let Some(home) = env::var_os("HOME") {
        let home_path = Path::new(&home);
        if let Ok(rel) = cwd.strip_prefix(home_path) {
            if rel.as_os_str().is_empty() {
                return "~".to_string();
            }
            return format!("~/{}", rel.display());
        }
    }
    cwd.display().to_string()
}

fn current_git_branch() -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let s = String::from_utf8(output.stdout).ok()?;
    let trimmed = s.trim().to_string();
    if trimmed.is_empty() || trimmed == "HEAD" {
        None
    } else {
        Some(trimmed)
    }
}
