//! Frecency tracking for commands, file mentions, and other rankable items.
//!
//! Ports the in-memory math from
//! `packages/jekko/src/cli/cmd/tui/component/prompt/frecency.tsx`.
//! The on-disk JSONL persistence is deferred — this layer keeps only the
//! `HashMap` state behind a small struct API.

use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Ranked entry returned by [`Frecency::top_n`].
#[derive(Clone, Debug, PartialEq)]
pub struct FrecencyRank {
    /// Caller-supplied identifier (command id, file path, etc.).
    pub id: String,
    /// Total number of times the entry has been bumped.
    pub count: u64,
    /// Computed frecency score (higher = more recent or more frequent).
    pub score: f64,
}

/// Per-entry data tracked by [`Frecency`].
#[derive(Clone, Copy, Debug)]
struct Entry {
    count: u64,
    last_used: Instant,
}

/// Frecency table mapping `id -> (count, last_used)`.
#[derive(Debug, Default, Clone)]
pub struct Frecency {
    data: HashMap<String, Entry>,
    now: Option<Instant>,
}

impl Frecency {
    /// Construct an empty table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Override the "current time" used when ranking. Test-only escape hatch
    /// to keep `Instant`-based math deterministic.
    pub fn set_now(&mut self, now: Instant) {
        self.now = Some(now);
    }

    /// Record a use of `id`, incrementing the count and updating `last_used`.
    pub fn bump(&mut self, id: impl Into<String>) {
        let now = match self.now {
            Some(t) => t,
            None => Instant::now(),
        };
        let entry = self.data.entry(id.into()).or_insert(Entry {
            count: 0,
            last_used: now,
        });
        entry.count += 1;
        entry.last_used = now;
    }

    /// Number of tracked entries.
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Whether the table is empty.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Count for `id` (zero if unknown).
    pub fn count_of(&self, id: &str) -> u64 {
        self.data.get(id).map(|e| e.count).unwrap_or(0)
    }

    /// Last-used timestamp for `id`, if any.
    pub fn last_used_of(&self, id: &str) -> Option<Instant> {
        self.data.get(id).map(|e| e.last_used)
    }

    /// Top `n` entries ranked by frecency (count weighted by recency).
    pub fn top_n(&self, n: usize) -> Vec<FrecencyRank> {
        let now = match self.now {
            Some(t) => t,
            None => Instant::now(),
        };
        let mut ranked: Vec<FrecencyRank> = self
            .data
            .iter()
            .map(|(id, entry)| FrecencyRank {
                id: id.clone(),
                count: entry.count,
                score: score(entry, now),
            })
            .collect();
        ranked.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(b.count.cmp(&a.count))
                .then(a.id.cmp(&b.id))
        });
        ranked.truncate(n);
        ranked
    }
}

fn score(entry: &Entry, now: Instant) -> f64 {
    let elapsed = now.saturating_duration_since(entry.last_used);
    let days = elapsed.as_secs_f64() / Duration::from_secs(86_400).as_secs_f64();
    let weight = 1.0 / (1.0 + days);
    (entry.count as f64) * weight
}
