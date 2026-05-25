//! Canonical credential-source policy for runtime model calls.
//!
//! ZYAL flows MUST use `UsersOnly` (the safe default) so per-user key isolation
//! is preserved. The `Any` variant exists for legacy single-user development
//! paths and SHOULD NOT be selected by new code.

use serde::{Deserialize, Serialize};

/// Credential source policy forwarded to live runtime child processes via the
/// `JEKKO_KEY_SOURCE_POLICY` env var.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CredentialSourcePolicy {
    /// Runtime may use its normal credential resolution order. Reserved for
    /// legacy development paths; not used by ZYAL production flows.
    Any,
    /// Runtime may only read keys from `~/.jekko/users/*/llm.env`. Default for
    /// every ZYAL flow.
    #[default]
    UsersOnly,
}

impl CredentialSourcePolicy {
    /// Environment-variable value understood by `jekko-runtime`.
    pub fn env_value(self) -> &'static str {
        match self {
            Self::Any => "any",
            Self::UsersOnly => "users-only",
        }
    }

    /// Whether this policy restricts credentials to the per-user pool.
    pub fn users_only(self) -> bool {
        matches!(self, Self::UsersOnly)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_users_only() {
        let policy = CredentialSourcePolicy::default();
        assert!(policy.users_only());
        assert_eq!(policy.env_value(), "users-only");
    }

    #[test]
    fn serde_roundtrip_is_kebab_case() {
        let any: CredentialSourcePolicy = serde_json::from_str("\"any\"").unwrap();
        assert_eq!(any, CredentialSourcePolicy::Any);
        let users: CredentialSourcePolicy = serde_json::from_str("\"users-only\"").unwrap();
        assert_eq!(users, CredentialSourcePolicy::UsersOnly);
        assert_eq!(
            serde_json::to_string(&CredentialSourcePolicy::UsersOnly).unwrap(),
            "\"users-only\""
        );
    }
}
