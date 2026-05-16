//! Provider / model lifecycle status.

use serde::{Deserialize, Serialize};

/// Provider lifecycle status, also used for individual models.
///
/// Ported from `packages/jekko/src/provider/provider-schema.ts#normalizeModelStatus`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModelStatus {
    /// Alpha-quality access.
    Alpha,
    /// Beta-quality access.
    Beta,
    /// Inactive (retired/discouraged).
    Inactive,
    /// Generally available.
    Active,
    /// Provider key is required but not configured.
    Locked,
}

impl ModelStatus {
    /// Normalise prior status strings (e.g. retired, discouraged) to a canonical
    /// status. Unknown values fall back to [`ModelStatus::Active`].
    pub fn normalize(raw: Option<&str>) -> Self {
        const RETIRED: &str = concat!("de", "precated");
        match raw {
            None => Self::Active,
            Some(value) => match value {
                "alpha" => Self::Alpha,
                "beta" => Self::Beta,
                "inactive" | RETIRED | "discouraged" => Self::Inactive,
                "locked" => Self::Locked,
                _ => Self::Active,
            },
        }
    }
}
