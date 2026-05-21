//! Multi-user key pool for `~/.jekko/users/<user_id>/llm.env`.
//!
//! Layout:
//!
//! ```text
//! ~/.jekko/
//! ├── users/
//! │   ├── user/                  ← always-on default (locked + unlocked)
//! │   │   ├── llm.env
//! │   │   └── state.sqlite       ← owned by [`crate::key_balancer`] consumers
//! │   ├── user_1/                ← unlock-only; auto-detected if dir + llm.env present
//! │   │   └── llm.env
//! │   └── user_2/                ← drop in at any time; picked up on next scan
//! │       └── llm.env
//! └── jekko.env.bak              ← post-migration only
//! ```
//!
//! Locked mode (no jnoccio unlock): only `users/user/` is read.
//! Unlocked mode: every `users/*/` subdir containing `llm.env` becomes a
//! candidate. Each `(provider, user)` pair is a unique candidate; the model
//! axis is layered on top by the balancer.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

use crate::setup::{catalog_entry, parse_env_lines, ModelKeySource};

/// Canonical default user id used in single-user (locked) mode.
pub const DEFAULT_USER_ID: &str = "user";

/// Per-user filename inside `~/.jekko/users/<user_id>/`.
pub const LLM_ENV_FILENAME: &str = "llm.env";

/// Per-user state sqlite filename owned by the balancer.
pub const STATE_DB_FILENAME: &str = "state.sqlite";

/// One discovered user directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserDir {
    /// User id (the directory name under `users/`).
    pub user_id: String,
    /// Absolute path of `~/.jekko/users/<user_id>/`.
    pub dir: PathBuf,
    /// Absolute path of `<dir>/llm.env`.
    pub llm_env_path: PathBuf,
    /// Absolute path of `<dir>/state.sqlite`.
    pub state_db_path: PathBuf,
}

/// One credential candidate sourced from a user's `llm.env`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyCandidate {
    /// User dir id.
    pub user_id: String,
    /// Provider id (matches `CatalogEntry::provider_id`).
    pub provider_id: String,
    /// Resolved env-var name that produced the value (first non-blank from the
    /// catalog's `env_names` list).
    pub env_name: String,
    /// Raw credential value.
    pub key: String,
    /// Path the value came from (for tracing / status output).
    pub source_path: PathBuf,
    /// Source classification.
    pub source: ModelKeySource,
}

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
///   contains a readable `llm.env`. The default `user/` dir is always
///   included (even with no `llm.env`) so single-user installs still work.
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

/// One parsed `llm.env` snapshot plus the mtime it was read at.
#[derive(Debug, Clone)]
struct ParsedEnv {
    mtime: Option<SystemTime>,
    values: BTreeMap<String, String>,
}

/// Cached multi-user credential pool.
///
/// Restats every `llm.env` at most once per `ttl` and reparses files whose
/// mtime advanced. Designed for the hot per-request path: a no-op rescan is a
/// handful of `stat(2)` calls.
#[derive(Debug)]
pub struct KeyPool {
    unlocked: bool,
    users_root: PathBuf,
    ttl: Duration,
    last_scan: Option<Instant>,
    cached_dirs: Vec<UserDir>,
    cached_envs: BTreeMap<String, ParsedEnv>,
}

impl KeyPool {
    /// Default rescan window — short enough to feel live when dropping in a new
    /// `users/user_N/` dir, cheap enough to call on every agent turn.
    pub const DEFAULT_TTL: Duration = Duration::from_secs(2);

    /// Build a pool against the global users root resolved from `JEKKO_HOME` /
    /// `HOME`. Returns `None` only if neither var is set.
    pub fn new(unlocked: bool) -> Option<Self> {
        let users_root = users_root()?;
        Some(Self::with_root(users_root, unlocked))
    }

    /// Build a pool against an explicit users root (tests and isolated
    /// installs).
    pub fn with_root(users_root: PathBuf, unlocked: bool) -> Self {
        Self {
            unlocked,
            users_root,
            ttl: Self::DEFAULT_TTL,
            last_scan: None,
            cached_dirs: Vec::new(),
            cached_envs: BTreeMap::new(),
        }
    }

    /// Override the rescan TTL (tests rely on `Duration::ZERO` to force a
    /// rescan on every call).
    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.ttl = ttl;
        self
    }

    /// All currently known user dirs (after rescan).
    pub fn dirs(&mut self) -> &[UserDir] {
        self.ensure_fresh();
        &self.cached_dirs
    }

    /// All credential candidates for `provider_id`, in deterministic dir
    /// order. Single env file may contribute at most one candidate per
    /// provider — the first non-blank value among the catalog's `env_names`
    /// wins, matching `select_credential`'s legacy semantics.
    pub fn candidates(&mut self, provider_id: &str) -> Vec<KeyCandidate> {
        self.ensure_fresh();
        let Some(entry) = catalog_entry(provider_id) else {
            return Vec::new();
        };
        let mut out = Vec::with_capacity(self.cached_dirs.len());
        for dir in &self.cached_dirs {
            let Some(env) = self.cached_envs.get(&dir.user_id) else {
                continue;
            };
            for env_name in entry.env_names {
                if let Some(value) = env.values.get(*env_name) {
                    if !value.trim().is_empty() {
                        out.push(KeyCandidate {
                            user_id: dir.user_id.clone(),
                            provider_id: provider_id.to_string(),
                            env_name: (*env_name).to_string(),
                            key: value.clone(),
                            source_path: dir.llm_env_path.clone(),
                            source: ModelKeySource::UserLlmEnv,
                        });
                        break;
                    }
                }
            }
        }
        out
    }

    fn ensure_fresh(&mut self) {
        if let Some(last) = self.last_scan {
            if last.elapsed() < self.ttl {
                return;
            }
        }
        let dirs = discover_in(&self.users_root, self.unlocked);
        let mut envs = BTreeMap::new();
        for dir in &dirs {
            let mtime = fs::metadata(&dir.llm_env_path)
                .ok()
                .and_then(|m| m.modified().ok());
            let reuse = self
                .cached_envs
                .get(&dir.user_id)
                .filter(|cached| cached.mtime == mtime)
                .cloned();
            let parsed = reuse.unwrap_or_else(|| ParsedEnv {
                mtime,
                values: read_env(&dir.llm_env_path),
            });
            envs.insert(dir.user_id.clone(), parsed);
        }
        self.cached_dirs = dirs;
        self.cached_envs = envs;
        self.last_scan = Some(Instant::now());
    }
}

fn read_env(path: &Path) -> BTreeMap<String, String> {
    #[allow(clippy::manual_unwrap_or_default)]
    match fs::read_to_string(path) {
        Ok(text) => parse_env_lines(&text).into_iter().collect(),
        Err(_) => BTreeMap::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write_env(root: &Path, user: &str, contents: &str) {
        let dir = root.join(user);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join(LLM_ENV_FILENAME), contents).unwrap();
    }

    #[test]
    fn locked_returns_only_default_dir() {
        let tmp = TempDir::new().unwrap();
        write_env(tmp.path(), "user", "OPENAI_API_KEY=k0\n");
        write_env(tmp.path(), "user_1", "OPENAI_API_KEY=k1\n");
        write_env(tmp.path(), "user_2", "OPENAI_API_KEY=k2\n");
        let dirs = discover_in(tmp.path(), false);
        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0].user_id, "user");
    }

    #[test]
    fn unlocked_includes_extras_sorted_after_default() {
        let tmp = TempDir::new().unwrap();
        write_env(tmp.path(), "user_2", "OPENAI_API_KEY=k2\n");
        write_env(tmp.path(), "user_1", "OPENAI_API_KEY=k1\n");
        write_env(tmp.path(), "alice", "OPENAI_API_KEY=ka\n");
        let dirs = discover_in(tmp.path(), true);
        let ids: Vec<_> = dirs.iter().map(|d| d.user_id.clone()).collect();
        assert_eq!(ids, vec!["user", "alice", "user_1", "user_2"]);
    }

    #[test]
    fn unlocked_skips_dirs_without_llm_env() {
        let tmp = TempDir::new().unwrap();
        fs::create_dir_all(tmp.path().join("empty")).unwrap();
        write_env(tmp.path(), "user_1", "OPENAI_API_KEY=k\n");
        let ids: Vec<_> = discover_in(tmp.path(), true)
            .iter()
            .map(|d| d.user_id.clone())
            .collect();
        assert_eq!(ids, vec!["user", "user_1"]);
    }

    #[test]
    fn candidates_match_first_nonblank_env_name_per_provider() {
        let tmp = TempDir::new().unwrap();
        // Google has three env_names in priority order; only the second is set.
        write_env(tmp.path(), "user", "GEMINI_API_KEY=g1\n");
        write_env(tmp.path(), "user_1", "GOOGLE_API_KEY=g2\n");

        let mut pool =
            KeyPool::with_root(tmp.path().to_path_buf(), true).with_ttl(Duration::from_secs(0));
        let cands = pool.candidates("google");
        let ids: Vec<_> = cands.iter().map(|c| c.user_id.clone()).collect();
        assert_eq!(ids, vec!["user", "user_1"]);
        assert_eq!(cands[0].env_name, "GEMINI_API_KEY");
        assert_eq!(cands[0].key, "g1");
        assert_eq!(cands[1].env_name, "GOOGLE_API_KEY");
        assert_eq!(cands[1].key, "g2");
    }

    #[test]
    fn candidates_skip_blank_values() {
        let tmp = TempDir::new().unwrap();
        write_env(tmp.path(), "user", "OPENAI_API_KEY=\n");
        write_env(tmp.path(), "user_1", "OPENAI_API_KEY=k1\n");
        let mut pool =
            KeyPool::with_root(tmp.path().to_path_buf(), true).with_ttl(Duration::from_secs(0));
        let cands = pool.candidates("openai");
        assert_eq!(cands.len(), 1);
        assert_eq!(cands[0].user_id, "user_1");
    }

    #[test]
    fn newly_added_user_dir_is_picked_up_on_rescan() {
        let tmp = TempDir::new().unwrap();
        write_env(tmp.path(), "user", "OPENAI_API_KEY=k0\n");
        let mut pool =
            KeyPool::with_root(tmp.path().to_path_buf(), true).with_ttl(Duration::from_secs(0));
        assert_eq!(pool.candidates("openai").len(), 1);

        write_env(tmp.path(), "user_2", "OPENAI_API_KEY=k2\n");
        let users: Vec<_> = pool
            .candidates("openai")
            .into_iter()
            .map(|c| c.user_id)
            .collect();
        assert_eq!(users, vec!["user", "user_2"]);
    }

    #[test]
    fn ttl_caches_within_window() {
        let tmp = TempDir::new().unwrap();
        write_env(tmp.path(), "user", "OPENAI_API_KEY=k0\n");
        let mut pool =
            KeyPool::with_root(tmp.path().to_path_buf(), true).with_ttl(Duration::from_secs(60));
        let first = pool.candidates("openai").len();
        write_env(tmp.path(), "user_1", "OPENAI_API_KEY=k1\n");
        // Within TTL we should still see only the originally-discovered dirs.
        assert_eq!(pool.candidates("openai").len(), first);
    }

    #[test]
    fn unknown_provider_returns_no_candidates() {
        let tmp = TempDir::new().unwrap();
        write_env(tmp.path(), "user", "OPENAI_API_KEY=k\n");
        let mut pool =
            KeyPool::with_root(tmp.path().to_path_buf(), false).with_ttl(Duration::from_secs(0));
        assert!(pool.candidates("nope-not-real").is_empty());
    }
}
