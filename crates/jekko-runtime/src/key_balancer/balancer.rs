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
            let usage = stored_usage(store.get(provider_id, model_id));
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
        let mut usage = stored_usage(store.get(provider_id, model_id));
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
        let mut usage = stored_usage(store.get(provider_id, model_id));
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

#[allow(clippy::manual_unwrap_or_default)]
fn stored_usage(result: Result<KeyUsage>) -> KeyUsage {
    match result {
        Ok(value) => value,
        Err(_) => KeyUsage {
            attempts: 0,
            failures: 0,
            last_failure_at: None,
            cooldown_until: None,
            status: KeyHealth::Ready,
        },
    }
}
