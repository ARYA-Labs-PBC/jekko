//! Auth-v2 storage schema (accounts + credentials).
//!
//! Ported from `packages/jekko/src/v2/auth.ts`. Only the on-disk shapes are
//! represented; the layer/Service plumbing belongs in higher crates.
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::session::{AccountId, ServiceId};

/// Sample OAuth API key used internally by the TypeScript runtime.
pub const OAUTH_SAMPLE_KEY: &str = "jekko-oauth-sample-key";

/// Either an OAuth or an API-key credential.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Credential {
    /// OAuth refresh + access token pair.
    OAuth {
        /// Refresh token.
        refresh: String,
        /// Access token.
        access: String,
        /// Expiry as a Unix timestamp (non-negative integer).
        expires: u64,
    },
    /// Raw API key.
    Api {
        /// API key.
        key: String,
        /// Optional string metadata.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        metadata: Option<BTreeMap<String, String>>,
    },
}

/// One credentialled account.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Account {
    /// Account identifier.
    pub id: AccountId,
    /// Service identifier (e.g. `"anthropic"`).
    #[serde(rename = "serviceID")]
    pub service_id: ServiceId,
    /// Human description.
    pub description: String,
    /// Stored credential.
    pub credential: Credential,
}

/// Persistent on-disk shape of `auth-v2.json`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct AuthFile {
    /// Schema version. The TS runtime expects `2`.
    pub version: u32,
    /// All known accounts keyed by id.
    #[serde(default)]
    pub accounts: BTreeMap<String, Account>,
    /// Active account per service.
    #[serde(default)]
    pub active: BTreeMap<String, AccountId>,
}
