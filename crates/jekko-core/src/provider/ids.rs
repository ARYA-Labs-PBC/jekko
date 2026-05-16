//! Provider / model identifier newtypes and the composite [`ModelRef`].
//!
//! Ported from `packages/jekko/src/provider/schema.ts`.

use serde::{Deserialize, Serialize};

use crate::session::IdParseError;

string_newtype!(
    /// Strongly-typed provider identifier (e.g. `"anthropic"`, `"openai"`).
    pub ProviderId, "provider");

string_newtype!(
    /// Strongly-typed model identifier (e.g. `"claude-sonnet-4"`).
    pub ModelId, "model");

/// Composite `provider/model[/variant]` reference.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ModelRef {
    /// Provider identifier.
    #[serde(rename = "providerID")]
    pub provider_id: ProviderId,
    /// Model identifier.
    pub id: ModelId,
    /// Variant (defaults to `"default"`).
    #[serde(default = "default_variant")]
    pub variant: String,
}

/// Default value for [`ModelRef::variant`] (`"default"`).
pub(crate) fn default_variant() -> String {
    "default".to_string()
}

impl ModelRef {
    /// Parse a `provider/model` string into a [`ModelRef`] with `variant = "default"`.
    pub fn parse(input: &str) -> Result<Self, IdParseError> {
        let (provider, model) = input.split_once('/').ok_or(IdParseError {
            kind: "model",
            value: input.to_string(),
        })?;
        Ok(Self {
            provider_id: provider.parse()?,
            id: model.parse()?,
            variant: default_variant(),
        })
    }
}
