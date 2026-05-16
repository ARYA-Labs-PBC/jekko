//! Session identifiers and message domain types.
//!
//! Ported from `packages/jekko/src/session/schema.ts` and
//! `packages/jekko/src/v2/session*.ts`. All types are pure data
//! (no I/O, no async). Identifiers serialize transparently as strings.
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Error returned when parsing a strongly-typed identifier.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("invalid {kind} id: {value}")]
pub struct IdParseError {
    /// Identifier kind (e.g. "session", "message").
    pub kind: &'static str,
    /// Original value that failed to parse.
    pub value: String,
}

string_newtype!(
    /// Brand for a Session identifier (mirrors `SessionID`).
    pub SessionId, "session");
string_newtype!(
    /// Brand for a Message identifier (mirrors `MessageID`).
    pub MessageId, "message");
string_newtype!(
    /// Brand for a Part identifier (mirrors `PartID`).
    pub PartId, "part");
string_newtype!(
    /// Brand for a Permission identifier (mirrors `PermissionID`).
    pub PermissionId, "permission");
string_newtype!(
    /// Brand for an Event identifier (mirrors `EventV2.ID`).
    pub EventId, "event");
string_newtype!(
    /// Brand for a Workspace identifier.
    pub WorkspaceId, "workspace");
string_newtype!(
    /// Brand for an Account identifier (mirrors `AccountID`).
    pub AccountId, "account");
string_newtype!(
    /// Brand for a Service identifier (mirrors `ServiceID`).
    pub ServiceId, "service");

/// Delivery mode for a prompt.
///
/// Ported from `packages/jekko/src/v2/session.ts#Delivery`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Delivery {
    /// Process immediately on receipt.
    #[default]
    Immediate,
    /// Defer processing until the next idle window.
    Deferred,
}

/// Inline source range describing where a value came from in a prompt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Source {
    /// Inclusive start offset (character index).
    pub start: u32,
    /// Exclusive end offset (character index).
    pub end: u32,
    /// Original text covered by the range.
    pub text: String,
}

/// File attachment payload for a prompt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileAttachment {
    /// File location URI.
    pub uri: String,
    /// MIME type.
    pub mime: String,
    /// Optional human-readable file name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Optional description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Source-text origin (e.g. drag-and-drop region in the editor).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<Source>,
}

/// Agent attachment (an `@agent` mention).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentAttachment {
    /// Agent name.
    pub name: String,
    /// Source-text origin.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<Source>,
}

/// A user prompt.
///
/// Ported from `packages/jekko/src/v2/session-prompt.ts#Prompt`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Prompt {
    /// Plain-text portion of the prompt.
    pub text: String,
    /// Inline file attachments.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub files: Option<Vec<FileAttachment>>,
    /// Inline agent mentions.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agents: Option<Vec<AgentAttachment>>,
}

/// Tool output content block.
///
/// Ported from `packages/jekko/src/v2/tool-output.ts`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ToolContent {
    /// A text block.
    Text {
        /// Body text.
        text: String,
    },
    /// A file block.
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

/// Structured tool output (free-form JSON payload).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ToolStructured(pub serde_json::Map<String, serde_json::Value>);

/// Generic unknown error payload attached to events/messages.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnknownError {
    /// Tag carried over from the TypeScript union (`"unknown"`).
    #[serde(rename = "type")]
    pub kind: String,
    /// Error message.
    pub message: String,
}

impl UnknownError {
    /// Construct from a message, using the default `"unknown"` tag.
    pub fn message(message: impl Into<String>) -> Self {
        Self {
            kind: "unknown".to_string(),
            message: message.into(),
        }
    }
}

/// Token usage summary attached to assistant messages and step events.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct TokenUsage {
    /// Input tokens billed.
    pub input: f64,
    /// Output tokens billed.
    pub output: f64,
    /// Reasoning tokens billed.
    pub reasoning: f64,
    /// Cache hit/miss breakdown.
    pub cache: CacheUsage,
}

/// Cache portion of [`TokenUsage`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct CacheUsage {
    /// Tokens served from cache.
    pub read: f64,
    /// Tokens written to cache.
    pub write: f64,
}

/// Per-attempt retry error metadata for `Retried` events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetryError {
    /// Failure message.
    pub message: String,
    /// HTTP status code if available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status_code: Option<u32>,
    /// Whether the failure was deemed retryable.
    pub is_retryable: bool,
    /// Response headers (string-typed).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response_headers: Option<std::collections::BTreeMap<String, String>>,
    /// Response body (string-typed).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response_body: Option<String>,
    /// Free-form metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<std::collections::BTreeMap<String, String>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_id_round_trips() {
        let id: SessionId = "session_abc".parse().unwrap();
        assert_eq!(id.to_string(), "session_abc");
        assert_eq!(id.as_str(), "session_abc");
    }

    #[test]
    fn empty_id_rejected() {
        let err = "".parse::<MessageId>().unwrap_err();
        assert_eq!(err.kind, "message");
    }

    #[test]
    fn delivery_serializes_lowercase() {
        let json = serde_json::to_string(&Delivery::Deferred).unwrap();
        assert_eq!(json, "\"deferred\"");
        let de: Delivery = serde_json::from_str("\"immediate\"").unwrap();
        assert_eq!(de, Delivery::Immediate);
    }

    #[test]
    fn tool_content_tagged_union() {
        let text = ToolContent::Text {
            text: "hi".to_string(),
        };
        let json = serde_json::to_string(&text).unwrap();
        assert!(json.contains("\"type\":\"text\""));
        let decoded: ToolContent = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, text);
    }
}
