//! Session history compaction.
//!
//! Ported from `packages/jekko/src/session/compaction.ts`. The TS module
//! is a multi-thousand-line state machine; here we expose the **policy
//! decision** so other services can decide whether to compact a session
//! without re-implementing the (still-in-flux) ranking logic.

use serde::{Deserialize, Serialize};

/// Inputs to [`should_compact`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompactionInputs {
    /// Total input + output tokens consumed so far in the session.
    pub used_tokens: u64,
    /// Provider context window in tokens.
    pub context_window: u64,
    /// Last compaction timestamp (ms since epoch), if any.
    pub last_compaction_ms: Option<i64>,
    /// Current time (ms since epoch).
    pub now_ms: i64,
}

/// Decision returned by [`should_compact`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CompactionDecision {
    /// Do nothing.
    Skip,
    /// Compaction is recommended.
    Compact,
    /// Compaction is mandatory (over context window).
    ForceCompact,
}

/// Default compaction threshold: 80% of context window.
pub const SOFT_THRESHOLD: f64 = 0.80;
/// Hard threshold: 95% of context window.
pub const HARD_THRESHOLD: f64 = 0.95;
/// Cool-down between compactions (ms).
pub const COOLDOWN_MS: i64 = 60_000;

/// Decide whether to compact a session.
pub fn should_compact(inputs: &CompactionInputs) -> CompactionDecision {
    if inputs.context_window == 0 {
        return CompactionDecision::Skip;
    }
    let ratio = inputs.used_tokens as f64 / inputs.context_window as f64;
    if ratio >= HARD_THRESHOLD {
        return CompactionDecision::ForceCompact;
    }
    if ratio < SOFT_THRESHOLD {
        return CompactionDecision::Skip;
    }
    if let Some(last) = inputs.last_compaction_ms {
        if inputs.now_ms - last < COOLDOWN_MS {
            return CompactionDecision::Skip;
        }
    }
    CompactionDecision::Compact
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skip_when_under_threshold() {
        let d = should_compact(&CompactionInputs {
            used_tokens: 100,
            context_window: 1000,
            last_compaction_ms: None,
            now_ms: 0,
        });
        assert_eq!(d, CompactionDecision::Skip);
    }

    #[test]
    fn force_when_over_hard() {
        let d = should_compact(&CompactionInputs {
            used_tokens: 990,
            context_window: 1000,
            last_compaction_ms: None,
            now_ms: 0,
        });
        assert_eq!(d, CompactionDecision::ForceCompact);
    }

    #[test]
    fn cooldown_prevents_repeat() {
        let d = should_compact(&CompactionInputs {
            used_tokens: 850,
            context_window: 1000,
            last_compaction_ms: Some(1000),
            now_ms: 2000,
        });
        assert_eq!(d, CompactionDecision::Skip);
    }
}
