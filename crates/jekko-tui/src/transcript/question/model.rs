//! Data model + builders for `QuestionCard`.

/// How a question accepts answers.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum QuestionMode {
    /// User must pick exactly one option (or supply a custom answer).
    Single,
    /// User may pick multiple options (or add custom answers).
    Multi,
}

/// One pickable option.
#[derive(Clone, Debug)]
pub struct QuestionChoice {
    /// The label shown to the user (also the value returned on reply).
    pub label: String,
    /// Optional secondary description line.
    pub description: Option<String>,
}

impl QuestionChoice {
    /// Build a choice from a label.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            description: None,
        }
    }
    /// Attach a description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

/// Event yielded by [`QuestionCard::handle_key`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum QuestionEvent {
    /// User submitted final answers.
    Submitted {
        /// Selected labels (or the custom answer text) in original order.
        answers: Vec<String>,
    },
    /// User cancelled.
    Rejected,
    /// User toggled custom-answer editing.
    EditingChanged {
        /// New state.
        editing: bool,
    },
}

/// Inline question card state.
#[derive(Clone, Debug)]
pub struct QuestionCard {
    /// Stable request id.
    pub request_id: String,
    /// Question prompt text.
    pub prompt: String,
    /// Options the user can pick from.
    pub options: Vec<QuestionChoice>,
    /// Mode (single/multi).
    pub mode: QuestionMode,
    /// Allow custom typed answer (rendered as the last option).
    pub allow_custom: bool,
    /// Current cursor over the (options + optional custom) list.
    pub cursor: usize,
    /// Answers picked so far. For `Single`, holds at most one entry.
    pub picked: Vec<String>,
    /// In-progress custom answer text.
    pub custom_text: String,
    /// Whether the custom-answer textarea is in editing mode.
    pub editing_custom: bool,
}

impl QuestionCard {
    /// Build a new question card.
    pub fn new(
        request_id: impl Into<String>,
        prompt: impl Into<String>,
        options: Vec<QuestionChoice>,
    ) -> Self {
        Self {
            request_id: request_id.into(),
            prompt: prompt.into(),
            options,
            mode: QuestionMode::Single,
            allow_custom: true,
            cursor: 0,
            picked: Vec::new(),
            custom_text: String::new(),
            editing_custom: false,
        }
    }
    /// Set the answer mode.
    pub fn with_mode(mut self, mode: QuestionMode) -> Self {
        self.mode = mode;
        self
    }
    /// Toggle whether a custom answer is offered.
    pub fn with_custom(mut self, allow: bool) -> Self {
        self.allow_custom = allow;
        self
    }

    pub(super) fn total(&self) -> usize {
        self.options.len() + if self.allow_custom { 1 } else { 0 }
    }

    pub(super) fn is_custom_slot(&self, idx: usize) -> bool {
        self.allow_custom && idx == self.options.len()
    }

    /// Force a submit (e.g. caller pressed Confirm in some surrounding UI).
    pub fn submit(&self) -> QuestionEvent {
        QuestionEvent::Submitted {
            answers: self.picked.clone(),
        }
    }

    /// Snapshot.
    pub fn snapshot(&self) -> String {
        format!(
            "question[{}|{:?}|cursor={}|picked={}|editing={}] {}",
            self.request_id,
            self.mode,
            self.cursor,
            self.picked.join(","),
            self.editing_custom,
            self.prompt
        )
    }
}
