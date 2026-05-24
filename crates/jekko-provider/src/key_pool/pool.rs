use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

use crate::setup::{catalog_entry, parse_env_lines, ModelKeySource};

use super::dirs::{discover_in, users_root};
use super::types::{KeyCandidate, UserDir};

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
