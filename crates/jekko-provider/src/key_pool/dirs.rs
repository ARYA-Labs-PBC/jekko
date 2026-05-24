use std::fs;
use std::path::{Path, PathBuf};

use super::types::{UserDir, DEFAULT_USER_ID, LLM_ENV_FILENAME, STATE_DB_FILENAME};

/// Resolve the canonical `~/.jekko/users/` root.
///
/// Honors `JEKKO_HOME` if set (used by tests and isolated installs), otherwise
/// falls back to `$HOME/.jekko/users/`. Returns `None` when neither is
/// available — callers should treat that as "no candidates".
pub fn users_root() -> Option<PathBuf> {
    if let Some(custom) = std::env::var_os("JEKKO_HOME") {
        let path = PathBuf::from(custom);
        if !path.as_os_str().is_empty() {
            return Some(path.join("users"));
        }
    }
    std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".jekko").join("users"))
}

/// Build a [`UserDir`] for a given user id under the supplied users root.
pub fn user_dir(users_root: &Path, user_id: &str) -> UserDir {
    let dir = users_root.join(user_id);
    UserDir {
        user_id: user_id.to_string(),
        llm_env_path: dir.join(LLM_ENV_FILENAME),
        state_db_path: dir.join(STATE_DB_FILENAME),
        dir,
    }
}

/// Discover user dirs from `users/`.
///
/// - `unlocked = false`: returns only `users/user/` (always, even if `llm.env`
///   is missing — caller will create on demand).
/// - `unlocked = true`: returns every immediate subdir of `users/` that
///   contains a readable `llm.env`. The default `user/` dir is always included
///   (even with no `llm.env`) so single-user installs still work.
///
/// The default `user/` entry is always sorted first; remaining entries are
/// returned in dir-name order for deterministic balancing.
pub fn discover_user_dirs(unlocked: bool) -> Vec<UserDir> {
    let Some(root) = users_root() else {
        return Vec::new();
    };
    discover_in(&root, unlocked)
}

/// Test-friendly variant of [`discover_user_dirs`] that takes the `users/`
/// root explicitly.
pub fn discover_in(users_root: &Path, unlocked: bool) -> Vec<UserDir> {
    let default = user_dir(users_root, DEFAULT_USER_ID);
    if !unlocked {
        return vec![default];
    }

    let mut dirs: Vec<UserDir> = match fs::read_dir(users_root) {
        Ok(iter) => iter
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().ok().is_some_and(|t| t.is_dir()))
            .filter_map(|entry| {
                let name = entry.file_name().to_string_lossy().into_owned();
                if name == DEFAULT_USER_ID {
                    return None;
                }
                let candidate = user_dir(users_root, &name);
                if candidate.llm_env_path.is_file() {
                    Some(candidate)
                } else {
                    None
                }
            })
            .collect(),
        Err(_) => Vec::new(),
    };
    dirs.sort_by(|a, b| a.user_id.cmp(&b.user_id));
    let mut out = Vec::with_capacity(dirs.len() + 1);
    out.push(default);
    out.extend(dirs);
    out
}
