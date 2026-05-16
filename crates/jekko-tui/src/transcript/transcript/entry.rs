//! Discriminant enum that ties one transcript row to a card variant.

use crate::transcript::cards::{AssistantCard, ReasoningCard, SystemCard, ToolCard, UserCard};
use crate::transcript::permission::PermissionCard;
use crate::transcript::question::QuestionCard;

/// One row in the transcript. The variant set matches the union of all card
/// kinds exposed by the public API.
#[derive(Clone, Debug)]
pub enum TranscriptEntry {
    /// A user-authored message card.
    User(UserCard),
    /// An assistant turn (possibly multi-part — text, reasoning, tool call).
    Assistant(AssistantCard),
    /// A single tool invocation card.
    Tool(ToolCard),
    /// Standalone reasoning trace (when not nested inside an assistant turn).
    Reasoning(ReasoningCard),
    /// System/status notice (revert markers, daemon transitions, etc.).
    System(SystemCard),
    /// Inline permission request awaiting reply.
    Permission(PermissionCard),
    /// Inline interactive question awaiting reply.
    Question(QuestionCard),
}

impl TranscriptEntry {
    /// Approximate row height — used by the scroll logic. Cards render in
    /// variable heights so this is intentionally a rough estimator; the
    /// renderer is the authority during the actual paint.
    pub fn estimated_rows(&self) -> u16 {
        match self {
            TranscriptEntry::User(card) => card.estimated_rows(),
            TranscriptEntry::Assistant(card) => card.estimated_rows(),
            TranscriptEntry::Tool(card) => card.estimated_rows(),
            TranscriptEntry::Reasoning(card) => card.estimated_rows(),
            TranscriptEntry::System(_) => 2,
            TranscriptEntry::Permission(_) => 8,
            TranscriptEntry::Question(_) => 10,
        }
    }

    /// Short label suitable for activity feeds.
    pub fn kind_label(&self) -> &'static str {
        match self {
            TranscriptEntry::User(_) => "user",
            TranscriptEntry::Assistant(_) => "assistant",
            TranscriptEntry::Tool(_) => "tool",
            TranscriptEntry::Reasoning(_) => "reasoning",
            TranscriptEntry::System(_) => "system",
            TranscriptEntry::Permission(_) => "permission",
            TranscriptEntry::Question(_) => "question",
        }
    }
}
