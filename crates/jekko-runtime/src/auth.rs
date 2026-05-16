//! Authentication helpers.
//!
//! Ported from `packages/jekko/src/auth/index.ts`. The TS layer is mostly
//! glue around OAuth flows and token rotation; here we expose the
//! observable shape: a [`Credential`] with optional refresh metadata.

use chrono::Utc;
use serde::{Deserialize, Serialize};

/// One credential record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Credential {
    /// Opaque access token.
    pub access_token: String,
    /// Opaque refresh token (optional).
    #[serde(default)]
    pub refresh_token: Option<String>,
    /// Token expiry, ms since epoch.
    #[serde(default)]
    pub token_expiry: Option<i64>,
}

impl Credential {
    /// Construct a credential with no refresh metadata.
    pub fn bearer(token: impl Into<String>) -> Self {
        Self {
            access_token: token.into(),
            refresh_token: None,
            token_expiry: None,
        }
    }

    /// Whether the credential's expiry is in the past.
    pub fn is_expired(&self) -> bool {
        match self.token_expiry {
            Some(exp) => exp <= Utc::now().timestamp_millis(),
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unexpired_when_no_expiry_set() {
        assert!(!Credential::bearer("tok").is_expired());
    }

    #[test]
    fn expired_when_past() {
        let cred = Credential {
            access_token: "x".into(),
            refresh_token: None,
            token_expiry: Some(0),
        };
        assert!(cred.is_expired());
    }
}
