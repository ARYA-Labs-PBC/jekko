//! Message domain helpers.
//!
//! Ported from `packages/jekko/src/session/message.ts`. The TS file is
//! 1200+ LOC because it carries the v2 event-sourced schema; here we
//! expose the narrow surface needed by [`crate::session`]: roles, common
//! content blocks, and a couple of constructors.

use serde::{Deserialize, Serialize};

/// Roles a message can hold.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// User-authored message.
    User,
    /// Assistant-authored message.
    Assistant,
    /// Tool-call invocation or response.
    Tool,
    /// System prompt / instruction.
    System,
}

impl Role {
    /// String form used on the wire.
    pub fn as_wire(&self) -> &'static str {
        match self {
            Role::User => "user",
            Role::Assistant => "assistant",
            Role::Tool => "tool",
            Role::System => "system",
        }
    }
}

/// One content block. Mirrors the `MessageV2.Part` discriminator from
/// `packages/jekko/src/session/message.ts`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ContentBlock {
    /// Plain text.
    Text {
        /// Body text.
        text: String,
    },
    /// Tool call (assistant -> tool).
    Tool {
        /// Tool id (matches [`crate::tool::Tool::id`]).
        tool: String,
        /// Call id (correlation with the response).
        #[serde(rename = "callID", default)]
        call_id: String,
        /// Tool input.
        input: serde_json::Value,
    },
    /// Tool result (tool -> assistant).
    ToolResult {
        /// Call id of the originating tool call.
        #[serde(rename = "callID")]
        call_id: String,
        /// Tool output payload.
        output: serde_json::Value,
    },
    /// File attachment.
    File {
        /// File URI.
        uri: String,
        /// MIME type.
        mime: String,
        /// Display name.
        #[serde(default)]
        name: Option<String>,
    },
}

/// Convenience constructor for a single-text message.
pub fn text(text: impl Into<String>) -> Vec<ContentBlock> {
    vec![ContentBlock::Text { text: text.into() }]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn role_round_trip() {
        let json = serde_json::to_string(&Role::Assistant).unwrap();
        assert_eq!(json, "\"assistant\"");
        let r: Role = serde_json::from_str("\"tool\"").unwrap();
        assert_eq!(r, Role::Tool);
    }

    #[test]
    fn tool_block_round_trip() {
        let block = ContentBlock::Tool {
            tool: "bash".into(),
            call_id: "call_1".into(),
            input: serde_json::json!({ "cmd": "ls" }),
        };
        let json = serde_json::to_string(&block).unwrap();
        let back: ContentBlock = serde_json::from_str(&json).unwrap();
        assert_eq!(back, block);
    }
}
