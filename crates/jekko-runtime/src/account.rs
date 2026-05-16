//! Account credential storage.
//!
//! Ported from `packages/jekko/src/account/`. Persistence lives in
//! [`jekko_store::account`]; this module hosts the in-runtime
//! representation plus a thin convenience layer that lets callers
//! work with rich types instead of SQL rows.

use chrono::Utc;
use jekko_store::account::AccountRow;
use serde::{Deserialize, Serialize};

/// In-runtime view of an account.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountInfo {
    /// Account id.
    pub id: String,
    /// User email.
    pub email: String,
    /// Endpoint URL (e.g. control plane root).
    pub url: String,
    /// Opaque access token.
    pub access_token: String,
    /// Opaque refresh token.
    pub refresh_token: String,
    /// Optional token expiry (ms since epoch).
    #[serde(default)]
    pub token_expiry: Option<i64>,
}

impl AccountInfo {
    /// Construct a fresh in-runtime account with auto-generated timestamps.
    pub fn fresh_row(
        id: impl Into<String>,
        email: impl Into<String>,
        url: impl Into<String>,
        access_token: impl Into<String>,
        refresh_token: impl Into<String>,
    ) -> AccountRow {
        let now = Utc::now().timestamp_millis();
        AccountRow {
            id: id.into(),
            email: email.into(),
            url: url.into(),
            access_token: access_token.into(),
            refresh_token: refresh_token.into(),
            token_expiry: None,
            time_created: now,
            time_updated: now,
        }
    }
}

/// Convert a [`AccountRow`] into an [`AccountInfo`].
pub fn from_row(row: AccountRow) -> AccountInfo {
    AccountInfo {
        id: row.id,
        email: row.email,
        url: row.url,
        access_token: row.access_token,
        refresh_token: row.refresh_token,
        token_expiry: row.token_expiry,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_via_row() {
        let row = AccountInfo::fresh_row("acc_1", "a@b", "https://x", "a", "r");
        let info = from_row(row.clone());
        assert_eq!(info.id, "acc_1");
        assert_eq!(info.email, "a@b");
    }
}
