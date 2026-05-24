/// One owned diff body line carried by [`ChatEvent::Diff`]. Mirrors the
/// borrowed [`crate::transcript::inline_cards::DiffLine`] but stores `text` as
/// `String` so the payload can be moved across channel boundaries without a
/// lifetime parameter. The runtime materializes a borrowed view onto these
/// values at render time (`render_diff` / `render_diff_into`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiffBlockLine {
    /// Add / Del / Ctx.
    pub kind: crate::transcript::inline_cards::DiffLineKind,
    /// Original-file lineno (when known).
    pub old_lineno: Option<usize>,
    /// New-file lineno (when known).
    pub new_lineno: Option<usize>,
    /// Body text without the leading sigil.
    pub text: String,
}

/// Events streamed back from the chat backend to the inline runtime.
#[derive(Clone, Debug)]
pub enum ChatEvent {
    /// An incremental text delta from the assistant.
    AssistantDelta(String),
    /// A completed reasoning/thinking card.
    Reasoning { reasoning_id: String, text: String },
    /// Tool lifecycle / output event from the backend.
    Tool(ToolEvent),
    /// A parsed unified-diff block ready to render as a transcript card.
    Diff {
        /// Display path for the card header.
        path: String,
        /// Body lines in original document order.
        hunks: Vec<DiffBlockLine>,
    },
    /// The assistant turn finished cleanly.
    TurnComplete,
    /// The assistant turn failed; render an error notice.
    TurnFailed(String),
    /// Informational system notice.
    Notice(NoticeKind, String),
    /// Structured runtime lifecycle or service event.
    Runtime(crate::action::RuntimeEvent),
}

/// Trait for any backend the inline runtime can drive.
pub trait ChatBackend: Send + 'static {
    /// Submit a user prompt. Returns a receiver of streaming events for this
    /// turn. The runtime reads until `TurnComplete` or `TurnFailed`.
    fn start_turn(&mut self, prompt: String, cancel: CancellationToken) -> Receiver<ChatEvent>;
}
