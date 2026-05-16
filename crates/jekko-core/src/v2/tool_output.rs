//! Tool-output content types.
//!
//! Ported from `packages/jekko/src/v2/tool-output.ts`.
use serde::{Deserialize, Serialize};

/// One content block produced by a tool call.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Content {
    /// Plain-text content.
    Text {
        /// Body text.
        text: String,
    },
    /// File content.
    File {
        /// File URI.
        uri: String,
        /// MIME type.
        mime: String,
        /// Optional file name.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
}

/// Structured (free-form JSON) output payload.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Structured(pub serde_json::Map<String, serde_json::Value>);
