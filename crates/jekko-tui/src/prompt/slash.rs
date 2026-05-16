//! Slash-command popup state.
//!
//! Detects when `/` appears at column 0 and exposes a filterable command list.
//! The actual command catalog is owned by the host crate (jekko-cli); the
//! prompt module ships with a small built-in command list so the widget is usable
//! in isolation.

/// One row in the slash popup.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SlashCommand {
    /// Stable identifier (e.g. `"help"`, `"quit"`).
    pub id: String,
    /// Display label without the leading slash (e.g. `"help"`).
    pub label: String,
    /// Optional one-line description.
    pub description: Option<String>,
}

impl SlashCommand {
    /// Construct a command with no description.
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
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

/// Built-in slash commands.
pub fn builtin_commands() -> Vec<SlashCommand> {
    vec![
        SlashCommand::new("help", "help").with_description("Show keybind reference"),
        SlashCommand::new("quit", "quit").with_description("Exit Jekko"),
        SlashCommand::new("new", "new").with_description("Start a new session"),
        SlashCommand::new("model", "model").with_description("Pick a model"),
        SlashCommand::new("theme", "theme").with_description("Pick a theme"),
        SlashCommand::new("audit", "audit").with_description("Run jankurai audit"),
        SlashCommand::new("audit-check", "audit-check")
            .with_description("Check for duplicate code"),
        SlashCommand::new("jankurai-status", "jankurai-status")
            .with_description("Show current jankurai score"),
    ]
}

/// Slash popup state.
#[derive(Clone, Debug)]
pub struct SlashPopup {
    catalog: Vec<SlashCommand>,
    /// Lowercased query (whatever follows the leading `/`).
    query: String,
    /// Cursor into `filtered()`.
    cursor: usize,
    open: bool,
}

impl Default for SlashPopup {
    fn default() -> Self {
        Self::with_catalog(builtin_commands())
    }
}

impl SlashPopup {
    /// Build a popup over an explicit catalog.
    pub fn with_catalog(catalog: Vec<SlashCommand>) -> Self {
        Self {
            catalog,
            query: String::new(),
            cursor: 0,
            open: false,
        }
    }

    /// Replace the catalog (e.g. once Packet B populates it).
    pub fn set_catalog(&mut self, catalog: Vec<SlashCommand>) {
        self.catalog = catalog;
        self.cursor = 0;
    }

    /// Whether the popup is currently visible.
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Open the popup with an empty query.
    pub fn open(&mut self) {
        self.open = true;
        self.query.clear();
        self.cursor = 0;
    }

    /// Close the popup.
    pub fn close(&mut self) {
        self.open = false;
        self.query.clear();
        self.cursor = 0;
    }

    /// Update the query string. Resets the cursor.
    pub fn set_query(&mut self, query: impl Into<String>) {
        self.query = query.into().to_lowercase();
        self.cursor = 0;
    }

    /// Current cursor position.
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// Move cursor by `delta`, wrapping around the filtered list.
    pub fn move_cursor(&mut self, delta: isize) {
        let len = self.filtered().len();
        if len == 0 {
            self.cursor = 0;
            return;
        }
        let len_i = len as isize;
        let mut idx = self.cursor as isize + delta;
        while idx < 0 {
            idx += len_i;
        }
        self.cursor = (idx % len_i) as usize;
    }

    /// Filtered command list (prefix match against the query).
    pub fn filtered(&self) -> Vec<SlashCommand> {
        if self.query.is_empty() {
            return self.catalog.clone();
        }
        self.catalog
            .iter()
            .filter(|c| c.label.to_lowercase().starts_with(&self.query))
            .cloned()
            .collect()
    }

    /// Currently selected entry, if any.
    pub fn selected(&self) -> Option<SlashCommand> {
        self.filtered().get(self.cursor).cloned()
    }

    /// Inspect the current query.
    pub fn query(&self) -> &str {
        &self.query
    }
}

/// True if `buffer` matches the slash-popup trigger condition. The popup opens
/// when the buffer begins with `/` and contains no whitespace before the
/// cursor (i.e. the user is still typing the command name).
pub fn buffer_triggers_slash(buffer: &str) -> bool {
    buffer.starts_with('/') && !buffer.contains(char::is_whitespace)
}

/// Extract the query string from a triggering buffer (slash already stripped).
pub fn query_from_buffer(buffer: &str) -> &str {
    buffer.strip_prefix('/').unwrap_or(buffer)
}
