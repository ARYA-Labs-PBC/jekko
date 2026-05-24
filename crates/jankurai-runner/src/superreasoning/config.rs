use serde::{Deserialize, Serialize};

use crate::model_client::CredentialSourcePolicy;

/// Superreasoning worker cap shared by live and deterministic workflows.
pub const MAX_SUPERREASONING_WORKERS: usize = 10;

/// Runbook-level superreasoning options.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SuperReasoningConfig {
    /// Enable packet and gate artifacts.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Worker cap, clamped to [`MAX_SUPERREASONING_WORKERS`].
    #[serde(default = "default_max_workers")]
    pub max_workers: usize,
    /// Credential source policy for live child runs.
    #[serde(default)]
    pub credential_policy: CredentialSourcePolicy,
    /// Require negative memory artifacts.
    #[serde(default = "default_true")]
    pub require_negative_memory: bool,
    /// Require unsupported-claims ledger.
    #[serde(default = "default_true")]
    pub require_unsupported_claims_ledger: bool,
    /// Require replay receipt before completion.
    #[serde(default = "default_true")]
    pub require_replay_gate: bool,
    /// Require parity failures to block completion.
    #[serde(default = "default_true")]
    pub parity_fail_on_required: bool,
}

impl Default for SuperReasoningConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_workers: default_max_workers(),
            credential_policy: CredentialSourcePolicy::UsersOnly,
            require_negative_memory: true,
            require_unsupported_claims_ledger: true,
            require_replay_gate: true,
            parity_fail_on_required: true,
        }
    }
}

impl SuperReasoningConfig {
    /// Return the effective worker cap.
    pub fn effective_max_workers(&self) -> usize {
        self.max_workers.clamp(1, MAX_SUPERREASONING_WORKERS)
    }
}

fn default_true() -> bool {
    true
}

fn default_max_workers() -> usize {
    MAX_SUPERREASONING_WORKERS
}
