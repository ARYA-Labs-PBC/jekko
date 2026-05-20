use std::path::Path;

use super::action::SlashAction;
use super::builtins::BUILTIN_SLASH;
use super::command::SlashCommand;
use super::submenu::{SlashSubmenu, SLASH_SUBMENUS};
use super::user_defined;

/// Two-source slash command catalog. Builtins are the static parity list;
/// user-defined entries are loaded from `.jankurai/commands/*.md`.
#[derive(Clone, Debug, Default)]
pub struct SlashCatalog {
    builtins: Vec<SlashCommand>,
    user_defined: Vec<SlashCommand>,
}

impl SlashCatalog {
    /// Catalog with the static built-in set only.
    pub fn new() -> Self {
        Self {
            builtins: BUILTIN_SLASH.to_vec(),
            user_defined: Vec::new(),
        }
    }

    /// Attach the user-defined commands found under
    /// `<workspace_root>/.jankurai/commands/`. Per-file errors from the loader
    /// are dropped here; call [`user_defined::load_from_workspace`] directly
    /// if you need them.
    pub fn with_user_commands(mut self, workspace_root: &Path) -> Self {
        let report = user_defined::load_from_workspace(workspace_root);
        let builtin_ids: std::collections::HashSet<&str> =
            self.builtins.iter().map(|c| c.id()).collect();
        for cmd in report.commands {
            if builtin_ids.contains(cmd.id.as_str()) {
                continue;
            }
            self.user_defined.push(SlashCommand::user_defined(cmd));
        }
        self
    }

    /// Iterate every command, builtins first then user-defined.
    pub fn all(&self) -> impl Iterator<Item = &SlashCommand> {
        self.builtins.iter().chain(self.user_defined.iter())
    }

    pub fn len(&self) -> usize {
        self.builtins.len() + self.user_defined.len()
    }

    pub fn is_empty(&self) -> bool {
        self.builtins.is_empty() && self.user_defined.is_empty()
    }

    /// Filter by id prefix. Empty query returns every visible command.
    pub fn filter(&self, query: &str) -> Vec<&SlashCommand> {
        let q = query.to_lowercase();
        self.all()
            .filter(|cmd| q.is_empty() || cmd.id().starts_with(&q))
            .collect()
    }

    pub fn find(&self, id: &str) -> Option<&SlashCommand> {
        self.all().find(|cmd| cmd.id() == id)
    }

    /// Resolve a command id to its action.
    pub fn action_for(&self, id: &str) -> SlashAction {
        match self.find(id) {
            Some(cmd) if cmd.is_user_defined() => SlashAction::UserDefined {
                id: cmd.id().to_string(),
                body: match cmd.body() {
                    Some(body) => body.to_string(),
                    None => String::new(),
                },
            },
            Some(_) => SlashAction::for_builtin_id(id),
            None => SlashAction::Unknown,
        }
    }

    pub fn submenu_for(&self, id: &str) -> Option<&'static SlashSubmenu> {
        self.find(id)?;
        SLASH_SUBMENUS
            .iter()
            .find(|submenu| submenu.parent_id == id)
    }
}
