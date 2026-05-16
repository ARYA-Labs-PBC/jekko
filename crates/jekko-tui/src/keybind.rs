//! Section: Keybindings
//!
//! Single source of truth for all key hints displayed in the footer and nav.
//!
//! Rule: the label shown in the UI **must** match the actual key that works.
//! Never display `[J]` if the real binding is `Ctrl+J`.
//!
//! Usage:
//! ```rust,ignore
//! let hints = keybind::hints_for(FocusTarget::Composer);
//! footer.render_hints(hints, available_width);
//! ```

/// Which pane/context currently owns the keyboard.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FocusTarget {
    Composer,
    Reasoning,
    Inspector,
    Modal,
}

/// A single displayable key hint: `[key] label`.
#[derive(Clone, Copy, Debug)]
pub struct KeyHint {
    /// Displayed key badge, e.g. `"Tab"`, `"F1"`, `"Ctrl+C"`.
    pub key: &'static str,
    /// Short action label, e.g. `"Pane"`, `"Send"`, `"Quit"`.
    pub label: &'static str,
    /// Lower = higher priority. Hints are dropped highest-number-first when
    /// the footer is too narrow. Priority 1 hints are always shown if possible.
    pub priority: u8,
}

impl KeyHint {
    const fn new(key: &'static str, label: &'static str, priority: u8) -> Self {
        Self { key, label, priority }
    }

    /// Rendered width: `[key] label` + 3-space separator.
    pub fn render_width(&self) -> usize {
        // "[" + key + "] " + label = key.len() + label.len() + 3
        self.key.len() + self.label.len() + 3
    }
}

// ── Context hint sets ────────────────────────────────────────────────────────

pub const HINTS_COMPOSER: &[KeyHint] = &[
    KeyHint::new("Tab", "Pane", 1),
    KeyHint::new("/", "Commands", 1),
    KeyHint::new("Enter", "Send", 1),
    KeyHint::new("?", "Help", 2),
    KeyHint::new("Ctrl+C", "Quit", 3),
];

pub const HINTS_REASONING: &[KeyHint] = &[
    KeyHint::new("Tab", "Pane", 1),
    KeyHint::new("↑↓/j k", "Scroll", 1),
    KeyHint::new("/", "Search", 2),
    KeyHint::new("?", "Help", 2),
    KeyHint::new("Ctrl+C", "Quit", 3),
];

pub const HINTS_INSPECTOR: &[KeyHint] = &[
    KeyHint::new("1-6", "Tabs", 1),
    KeyHint::new("↑↓/j k", "Select", 1),
    KeyHint::new("Enter", "Detail", 1),
    KeyHint::new("/", "Search", 2),
    KeyHint::new("Esc", "Back", 2),
    KeyHint::new("Ctrl+C", "Quit", 3),
];

pub const HINTS_MODAL: &[KeyHint] = &[
    KeyHint::new("Esc", "Close", 1),
    KeyHint::new("↑↓", "Scroll", 1),
    KeyHint::new("/", "Filter", 2),
];

// ── Nav labels ───────────────────────────────────────────────────────────────

/// Top-nav tab definitions used by NavBar.
pub struct NavTab {
    pub key: &'static str,
    pub label: &'static str,
}

pub const NAV_TABS: &[NavTab] = &[
    NavTab { key: "F1", label: "Chat" },
    NavTab { key: "F2", label: "Repo Intel" },
    NavTab { key: "F3", label: "History" },
];

// ── Hint selector ────────────────────────────────────────────────────────────

pub fn hints_for(focus: FocusTarget) -> &'static [KeyHint] {
    match focus {
        FocusTarget::Composer => HINTS_COMPOSER,
        FocusTarget::Reasoning => HINTS_REASONING,
        FocusTarget::Inspector => HINTS_INSPECTOR,
        FocusTarget::Modal => HINTS_MODAL,
    }
}
