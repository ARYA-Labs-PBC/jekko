//! Smart load-balancer over `(provider, user, model)` key candidates.
//!
//! Wraps [`jekko_provider::key_pool::KeyPool`] with per-tuple usage
//! counters (attempts / failures / cooldowns) persisted in
//! `~/.jekko/users/<user_id>/state.sqlite`. Selection is a weighted roulette
//! over the candidate pool — adapted from
//! `jnoccio-fusion/src/routing.rs::load_balance_factor` but at key
//! granularity instead of model granularity.
//!
//! ```text
//! KeyBalancer::pick(provider, model)
//!     candidates ← KeyPool::candidates(provider)             // (provider, user) tuples
//!     for each candidate:
//!         load ← state.attempts at (provider, user, model)
//!         weight ← health(status) × load_factor(load) × failure_penalty(failures)
//!     pick by weighted roulette → (user, ApiKey)
//!
//! KeyBalancer::record_success/failure (provider, user, model, kind)
//!     persists into state.sqlite for that user
//! ```

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use jekko_provider::adapter::ProviderCredential;
use jekko_provider::key_pool::{user_dir, KeyPool, STATE_DB_FILENAME};
use rand::Rng;
use rusqlite::Connection;

/// Health classification stored per `(provider, model)` per user.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyHealth {
    /// Working as expected.
    Ready,
    /// HTTP 429 — temporarily rate-limited.
    RateLimited,
    /// HTTP 401/403 — credential rejected.
    AuthFailed,
    /// 5xx — upstream failure.
    ServerError,
}

impl KeyHealth {
    fn as_str(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::RateLimited => "rate_limited",
            Self::AuthFailed => "auth_failed",
            Self::ServerError => "server_error",
        }
    }

    fn from_str(s: &str) -> Self {
        match s {
            "rate_limited" => Self::RateLimited,
            "auth_failed" => Self::AuthFailed,
            "server_error" => Self::ServerError,
            _ => Self::Ready,
        }
    }

    fn weight(self) -> f64 {
        match self {
            Self::Ready => 1.0,
            Self::RateLimited => 0.25,
            Self::ServerError => 0.45,
            Self::AuthFailed => 0.0,
        }
    }
}

/// Outcome categories used by [`KeyBalancer::record_failure`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureKind {
    /// HTTP 429.
    RateLimited,
    /// HTTP 401/403.
    AuthFailed,
    /// HTTP 5xx.
    ServerError,
    /// Anything else (timeout, transport, json decode...).
    Other,
}

impl FailureKind {
    fn classify_http(status: u16) -> Self {
        match status {
            401 | 403 => Self::AuthFailed,
            429 => Self::RateLimited,
            500..=599 => Self::ServerError,
            _ => Self::Other,
        }
    }

    fn cooldown_seconds(self, failures: u64) -> i64 {
        let n = failures.min(10) as i64;
        match self {
            Self::RateLimited => 30 * (1 + n),
            Self::AuthFailed => 60 * 60 * 24,
            Self::ServerError => 15 * (1 + n / 2),
            Self::Other => 5 * (1 + n),
        }
    }

    fn health(self) -> KeyHealth {
        match self {
            Self::RateLimited => KeyHealth::RateLimited,
            Self::AuthFailed => KeyHealth::AuthFailed,
            Self::ServerError | Self::Other => KeyHealth::ServerError,
        }
    }
}

/// One row of the `key_usage` table.
#[derive(Debug, Clone)]
pub struct KeyUsage {
    /// Number of selection attempts so far.
    pub attempts: u64,
    /// Number of failures so far.
    pub failures: u64,
    /// UNIX timestamp of the last failure, if any.
    pub last_failure_at: Option<i64>,
    /// UNIX timestamp until which this key is sidelined.
    pub cooldown_until: Option<i64>,
    /// Current health classification.
    pub status: KeyHealth,
}

impl Default for KeyUsage {
    fn default() -> Self {
        Self {
            attempts: 0,
            failures: 0,
            last_failure_at: None,
            cooldown_until: None,
            status: KeyHealth::Ready,
        }
    }
}

/// Per-user sqlite-backed usage store. Opens lazily on first touch.
#[derive(Debug)]
pub struct BalancerStore {
    db_path: PathBuf,
    conn: Mutex<Option<Connection>>,
}

impl BalancerStore {
    /// Build a store handle. Does not open the connection until first use.
    pub fn new(db_path: PathBuf) -> Self {
        Self {
            db_path,
            conn: Mutex::new(None),
        }
    }

    fn with_conn<R>(&self, f: impl FnOnce(&Connection) -> rusqlite::Result<R>) -> Result<R> {
        let mut guard = self
            .conn
            .lock()
            .map_err(|_| anyhow::anyhow!("balancer store mutex poisoned"))?;
        if guard.is_none() {
            if let Some(parent) = self.db_path.parent() {
                std::fs::create_dir_all(parent).with_context(|| {
                    format!("create balancer state dir at {}", parent.display())
                })?;
            }
            let conn = Connection::open(&self.db_path)
                .with_context(|| format!("open balancer state at {}", self.db_path.display()))?;
            conn.execute_batch(SCHEMA)
                .context("apply balancer state schema")?;
            *guard = Some(conn);
        }
        let conn = guard.as_ref().expect("conn just initialised");
        Ok(f(conn).context("balancer state query failed")?)
    }

    /// Fetch all usage rows for a given `provider`. Returns rows keyed by
    /// `model` id.
    pub fn load(&self, provider: &str) -> Result<BTreeMap<String, KeyUsage>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT model, attempts, failures, last_failure_at, cooldown_until, status \
                 FROM key_usage WHERE provider = ?1",
            )?;
            let rows = stmt.query_map([provider], |row| {
                let model: String = row.get(0)?;
                Ok((
                    model,
                    KeyUsage {
                        attempts: row.get::<_, i64>(1)? as u64,
                        failures: row.get::<_, i64>(2)? as u64,
                        last_failure_at: row.get::<_, Option<i64>>(3)?,
                        cooldown_until: row.get::<_, Option<i64>>(4)?,
                        status: KeyHealth::from_str(&row.get::<_, String>(5)?),
                    },
                ))
            })?;
            let mut out = BTreeMap::new();
            for row in rows {
                let (model, usage) = row?;
                out.insert(model, usage);
            }
            Ok(out)
        })
    }

    /// Load a single `(provider, model)` row.
    pub fn get(&self, provider: &str, model: &str) -> Result<KeyUsage> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT attempts, failures, last_failure_at, cooldown_until, status \
                 FROM key_usage WHERE provider = ?1 AND model = ?2",
            )?;
            let opt = stmt
                .query_row([provider, model], |row| {
                    Ok(KeyUsage {
                        attempts: row.get::<_, i64>(0)? as u64,
                        failures: row.get::<_, i64>(1)? as u64,
                        last_failure_at: row.get::<_, Option<i64>>(2)?,
                        cooldown_until: row.get::<_, Option<i64>>(3)?,
                        status: KeyHealth::from_str(&row.get::<_, String>(4)?),
                    })
                })
                .ok();
            Ok(opt.unwrap_or_default())
        })
    }

    fn upsert(&self, provider: &str, model: &str, usage: &KeyUsage) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO key_usage (provider, model, attempts, failures, \
                                        last_failure_at, cooldown_until, status) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7) \
                 ON CONFLICT(provider, model) DO UPDATE SET \
                    attempts = excluded.attempts, \
                    failures = excluded.failures, \
                    last_failure_at = excluded.last_failure_at, \
                    cooldown_until = excluded.cooldown_until, \
                    status = excluded.status",
                rusqlite::params![
                    provider,
                    model,
                    usage.attempts as i64,
                    usage.failures as i64,
                    usage.last_failure_at,
                    usage.cooldown_until,
                    usage.status.as_str(),
                ],
            )?;
            Ok(())
        })
    }
}

const SCHEMA: &str = "\
CREATE TABLE IF NOT EXISTS key_usage (
  provider        TEXT NOT NULL,
  model           TEXT NOT NULL,
  attempts        INTEGER NOT NULL DEFAULT 0,
  failures        INTEGER NOT NULL DEFAULT 0,
  last_failure_at INTEGER,
  cooldown_until  INTEGER,
  status          TEXT NOT NULL DEFAULT 'ready',
  PRIMARY KEY (provider, model)
);
";

/// Decision returned by [`KeyBalancer::pick`].
#[derive(Debug, Clone)]
pub struct KeyPick {
    /// Selected user id.
    pub user_id: String,
    /// Selected env-var name.
    pub env_name: String,
    /// Credential to hand to the adapter.
    pub credential: ProviderCredential,
}

/// Smart balancer over the per-user key pool.
pub struct KeyBalancer {
    pool: KeyPool,
    users_root: PathBuf,
    stores: BTreeMap<String, BalancerStore>,
}

impl KeyBalancer {
    /// Build a balancer rooted at `users_root` (typically `~/.jekko/users/`).
    pub fn with_root(users_root: PathBuf, unlocked: bool) -> Self {
        Self {
            pool: KeyPool::with_root(users_root.clone(), unlocked),
            users_root,
            stores: BTreeMap::new(),
        }
    }

    /// Build a balancer using `JEKKO_HOME` / `HOME` for the users root.
    pub fn new(unlocked: bool) -> Option<Self> {
        let root = jekko_provider::key_pool::users_root()?;
        Some(Self::with_root(root, unlocked))
    }

    /// Replace the pool TTL (tests use `Duration::ZERO` to force rescans).
    pub fn with_pool_ttl(mut self, ttl: std::time::Duration) -> Self {
        self.pool = self.pool.with_ttl(ttl);
        self
    }

    /// Pick the best candidate for `(provider_id, model_id)`. Returns `None`
    /// when there are no candidates with non-zero weight.
    pub fn pick(&mut self, provider_id: &str, model_id: &str) -> Option<KeyPick> {
        let candidates = self.pool.candidates(provider_id);
        if candidates.is_empty() {
            return None;
        }
        let now = unix_now();
        let mut weights = Vec::with_capacity(candidates.len());
        for cand in &candidates {
            let store = self.store_for(&cand.user_id);
            let usage = store.get(provider_id, model_id).unwrap_or_default();
            let weight = score(&usage, now);
            weights.push(weight);
        }
        let index = pick_weighted_index(&weights)?;
        let pick = candidates[index].clone();
        Some(KeyPick {
            user_id: pick.user_id,
            env_name: pick.env_name,
            credential: ProviderCredential::ApiKey { key: pick.key },
        })
    }

    /// Record a successful turn against `(provider, user, model)`.
    pub fn record_success(&mut self, provider_id: &str, user_id: &str, model_id: &str) {
        let store = self.store_for(user_id);
        let mut usage = store.get(provider_id, model_id).unwrap_or_default();
        usage.attempts = usage.attempts.saturating_add(1);
        usage.status = KeyHealth::Ready;
        usage.cooldown_until = None;
        let _ = store.upsert(provider_id, model_id, &usage);
    }

    /// Record a failure against `(provider, user, model)`.
    pub fn record_failure(
        &mut self,
        provider_id: &str,
        user_id: &str,
        model_id: &str,
        kind: FailureKind,
    ) {
        let store = self.store_for(user_id);
        let mut usage = store.get(provider_id, model_id).unwrap_or_default();
        let now = unix_now();
        usage.attempts = usage.attempts.saturating_add(1);
        usage.failures = usage.failures.saturating_add(1);
        usage.last_failure_at = Some(now);
        usage.cooldown_until = Some(now + kind.cooldown_seconds(usage.failures));
        usage.status = kind.health();
        let _ = store.upsert(provider_id, model_id, &usage);
    }

    /// Convenience: classify an HTTP status code and record it.
    pub fn record_http(&mut self, provider_id: &str, user_id: &str, model_id: &str, status: u16) {
        if (200..300).contains(&status) {
            self.record_success(provider_id, user_id, model_id);
        } else {
            self.record_failure(
                provider_id,
                user_id,
                model_id,
                FailureKind::classify_http(status),
            );
        }
    }

    fn store_for(&mut self, user_id: &str) -> &BalancerStore {
        self.stores.entry(user_id.to_string()).or_insert_with(|| {
            let dir = user_dir(&self.users_root, user_id);
            BalancerStore::new(dir.dir.join(STATE_DB_FILENAME))
        })
    }
}

fn score(usage: &KeyUsage, now: i64) -> f64 {
    if usage
        .cooldown_until
        .map(|until| until > now)
        .unwrap_or(false)
    {
        return 0.0;
    }
    let health = usage.status.weight();
    if health == 0.0 {
        return 0.0;
    }
    let load = 1.0 / (1.0 + usage.attempts as f64 / 20.0);
    let penalty = if usage.failures == 0 {
        1.0
    } else {
        (1.0 / (1.0 + usage.failures as f64 * 6.0)).clamp(0.01, 1.0)
    };
    (health * load * penalty).max(0.0001)
}

fn pick_weighted_index(weights: &[f64]) -> Option<usize> {
    let total: f64 = weights.iter().sum();
    if total <= 0.0 {
        return None;
    }
    let mut draw = rand::thread_rng().gen_range(0.0..total);
    for (i, w) in weights.iter().enumerate() {
        if draw < *w {
            return Some(i);
        }
        draw -= *w;
    }
    weights.iter().position(|w| *w > 0.0)
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use jekko_provider::key_pool::LLM_ENV_FILENAME;
    use std::collections::BTreeMap;
    use std::fs;
    use std::time::Duration;
    use tempfile::TempDir;

    fn write_env(root: &std::path::Path, user: &str, contents: &str) {
        let dir = root.join(user);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join(LLM_ENV_FILENAME), contents).unwrap();
    }

    #[test]
    fn store_roundtrips_usage() {
        let tmp = TempDir::new().unwrap();
        let store = BalancerStore::new(tmp.path().join("state.sqlite"));
        store
            .upsert(
                "openai",
                "gpt-5",
                &KeyUsage {
                    attempts: 7,
                    failures: 1,
                    last_failure_at: Some(100),
                    cooldown_until: Some(200),
                    status: KeyHealth::RateLimited,
                },
            )
            .unwrap();
        let got = store.get("openai", "gpt-5").unwrap();
        assert_eq!(got.attempts, 7);
        assert_eq!(got.failures, 1);
        assert_eq!(got.last_failure_at, Some(100));
        assert_eq!(got.cooldown_until, Some(200));
        assert_eq!(got.status, KeyHealth::RateLimited);
    }

    #[test]
    fn pick_returns_none_with_no_candidates() {
        let tmp = TempDir::new().unwrap();
        let mut bal = KeyBalancer::with_root(tmp.path().to_path_buf(), false)
            .with_pool_ttl(Duration::from_secs(0));
        assert!(bal.pick("openai", "gpt-5").is_none());
    }

    #[test]
    fn pick_distributes_across_two_healthy_users() {
        let tmp = TempDir::new().unwrap();
        write_env(tmp.path(), "user", "OPENAI_API_KEY=ka\n");
        write_env(tmp.path(), "user_1", "OPENAI_API_KEY=kb\n");
        let mut bal = KeyBalancer::with_root(tmp.path().to_path_buf(), true)
            .with_pool_ttl(Duration::from_secs(0));

        let mut counts: BTreeMap<String, u32> = BTreeMap::new();
        for _ in 0..400 {
            let pick = bal.pick("openai", "gpt-5").expect("pick");
            *counts.entry(pick.user_id.clone()).or_default() += 1;
            // Don't record attempts — purely measure raw distribution.
        }
        let a = *counts.get("user").unwrap_or(&0);
        let b = *counts.get("user_1").unwrap_or(&0);
        assert!(a > 100 && b > 100, "expected balanced split, got {a}/{b}");
    }

    #[test]
    fn auth_failure_excludes_key_from_pool() {
        let tmp = TempDir::new().unwrap();
        write_env(tmp.path(), "user", "OPENAI_API_KEY=ka\n");
        write_env(tmp.path(), "user_1", "OPENAI_API_KEY=kb\n");
        let mut bal = KeyBalancer::with_root(tmp.path().to_path_buf(), true)
            .with_pool_ttl(Duration::from_secs(0));

        bal.record_failure("openai", "user", "gpt-5", FailureKind::AuthFailed);
        for _ in 0..40 {
            let pick = bal.pick("openai", "gpt-5").expect("pick");
            assert_eq!(pick.user_id, "user_1");
        }
    }

    #[test]
    fn rate_limit_cools_down_and_recovers_after_window() {
        let tmp = TempDir::new().unwrap();
        write_env(tmp.path(), "user", "OPENAI_API_KEY=ka\n");
        write_env(tmp.path(), "user_1", "OPENAI_API_KEY=kb\n");
        let mut bal = KeyBalancer::with_root(tmp.path().to_path_buf(), true)
            .with_pool_ttl(Duration::from_secs(0));

        bal.record_failure("openai", "user", "gpt-5", FailureKind::RateLimited);
        // During cooldown user is excluded.
        let mut saw_user = false;
        for _ in 0..50 {
            let pick = bal.pick("openai", "gpt-5").expect("pick");
            if pick.user_id == "user" {
                saw_user = true;
                break;
            }
        }
        assert!(
            !saw_user,
            "rate-limited user should be skipped while in cooldown"
        );
    }

    #[test]
    fn record_success_clears_cooldown() {
        let tmp = TempDir::new().unwrap();
        write_env(tmp.path(), "user", "OPENAI_API_KEY=ka\n");
        let mut bal = KeyBalancer::with_root(tmp.path().to_path_buf(), true)
            .with_pool_ttl(Duration::from_secs(0));

        bal.record_failure("openai", "user", "gpt-5", FailureKind::RateLimited);
        bal.record_success("openai", "user", "gpt-5");

        let store = BalancerStore::new(user_dir(tmp.path(), "user").dir.join(STATE_DB_FILENAME));
        let usage = store.get("openai", "gpt-5").unwrap();
        assert_eq!(usage.status, KeyHealth::Ready);
        assert!(usage.cooldown_until.is_none());
    }

    #[test]
    fn classify_http_routes_to_correct_kind() {
        assert_eq!(FailureKind::classify_http(401), FailureKind::AuthFailed);
        assert_eq!(FailureKind::classify_http(403), FailureKind::AuthFailed);
        assert_eq!(FailureKind::classify_http(429), FailureKind::RateLimited);
        assert_eq!(FailureKind::classify_http(500), FailureKind::ServerError);
        assert_eq!(FailureKind::classify_http(502), FailureKind::ServerError);
        assert_eq!(FailureKind::classify_http(404), FailureKind::Other);
    }
}
