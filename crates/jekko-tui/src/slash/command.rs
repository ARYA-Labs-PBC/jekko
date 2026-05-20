use std::borrow::Cow;

use super::user_defined::UserCommand;

/// One slash command row. Backing strings are `Cow` so builtins can stay
/// `&'static` while user-defined entries own their data.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SlashCommand {
    pub id: Cow<'static, str>,
    pub description: Cow<'static, str>,
    /// Optional command body (only populated for user-defined commands).
    pub body: Option<Cow<'static, str>>,
}

impl SlashCommand {
    pub const fn builtin(id: &'static str, description: &'static str) -> Self {
        Self {
            id: Cow::Borrowed(id),
            description: Cow::Borrowed(description),
            body: None,
        }
    }

    pub fn user_defined(cmd: UserCommand) -> Self {
        Self {
            id: Cow::Owned(cmd.id),
            description: Cow::Owned(cmd.description),
            body: Some(Cow::Owned(cmd.body)),
        }
    }

    pub fn id(&self) -> &str {
        self.id.as_ref()
    }

    pub fn description(&self) -> &str {
        self.description.as_ref()
    }

    pub fn body(&self) -> Option<&str> {
        self.body.as_deref()
    }

    pub fn is_user_defined(&self) -> bool {
        self.body.is_some()
    }
}
