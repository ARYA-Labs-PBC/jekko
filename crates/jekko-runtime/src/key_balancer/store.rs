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
            Ok(match opt {
                Some(usage) => usage,
                None => KeyUsage::default(),
            })
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
