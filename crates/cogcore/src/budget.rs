//! LLM/embedding call budget for consolidation backends.
//!
//! Benchmark and tests use `Budget::ZERO` — no calls allowed, every
//! `check_and_consume` returns false. Production hosts may pass a
//! non-zero budget when ZYAL workflows wire a Jnoccio backend.

/// Tracks remaining LLM, embedding, and token quotas for an offline
/// consolidation pass. Cogcore consumers should hold the value by value
/// (it is `Copy`) so a consolidation pass can mutate locally without
/// disturbing host accounting.
#[derive(Debug, Clone, Copy, Default)]
pub struct Budget {
    /// Remaining LLM (chat-completion-style) calls.
    pub llm_calls_remaining: u32,
    /// Remaining embedding-call quota.
    pub embedding_calls_remaining: u32,
    /// Remaining token budget for combined call payloads.
    pub token_budget_remaining: u32,
}

impl Budget {
    /// All-zero budget. Benchmark and determinism suites pass this so
    /// no LLM-backed path can fire.
    pub const ZERO: Self = Budget {
        llm_calls_remaining: 0,
        embedding_calls_remaining: 0,
        token_budget_remaining: 0,
    };

    /// Try to spend `cost_calls` LLM calls. Returns true if budget allowed
    /// and was decremented; false if budget exhausted.
    pub fn check_and_consume_llm(&mut self, cost_calls: u32) -> bool {
        if self.llm_calls_remaining >= cost_calls {
            self.llm_calls_remaining -= cost_calls;
            true
        } else {
            false
        }
    }

    /// Try to spend `cost_calls` embedding calls. Returns true on success
    /// and false when the remaining quota is below `cost_calls`.
    pub fn check_and_consume_embedding(&mut self, cost_calls: u32) -> bool {
        if self.embedding_calls_remaining >= cost_calls {
            self.embedding_calls_remaining -= cost_calls;
            true
        } else {
            false
        }
    }

    /// Try to spend `cost_tokens` tokens against the shared token budget.
    /// Returns true on success; false when the quota cannot cover it.
    pub fn check_and_consume_tokens(&mut self, cost_tokens: u32) -> bool {
        if self.token_budget_remaining >= cost_tokens {
            self.token_budget_remaining -= cost_tokens;
            true
        } else {
            false
        }
    }

    /// True when any LLM-call quota is still available.
    pub fn has_any_llm(&self) -> bool {
        self.llm_calls_remaining > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_budget_rejects_everything() {
        let mut b = Budget::ZERO;
        assert!(!b.check_and_consume_llm(1));
        assert!(!b.check_and_consume_embedding(1));
        assert!(!b.check_and_consume_tokens(1));
        assert!(!b.has_any_llm());
    }

    #[test]
    fn non_zero_budget_drains() {
        let mut b = Budget {
            llm_calls_remaining: 5,
            embedding_calls_remaining: 100,
            token_budget_remaining: 10_000,
        };
        assert!(b.check_and_consume_llm(2));
        assert_eq!(b.llm_calls_remaining, 3);
        assert!(b.check_and_consume_llm(3));
        assert_eq!(b.llm_calls_remaining, 0);
        assert!(!b.check_and_consume_llm(1));
    }
}
