//! Jnoccio dashboard panel struct + non-key-handling methods.
//!
//! The key dispatch helpers live in [`super::keys`], the Ratatui rendering
//! lives in [`super::render`], and the static colour / sort tables live in
//! [`super::model`].

use super::model::{JnoccioConnection, JnoccioSnapshot, JnoccioTab, SORT_MODES};

/// Jnoccio Fusion dashboard panel.
///
/// Owns the per-panel UI state (selected tab, cursor, paused flag, help
/// overlay, search buffer, drawer model id). Use [`JnoccioPanel::set_snapshot`]
/// to update the displayed data, and [`JnoccioPanel::dispatch_key`] to feed
/// key events.
#[derive(Clone, Debug)]
pub struct JnoccioPanel {
    pub(super) snapshot: JnoccioSnapshot,
    pub(super) connection: JnoccioConnection,
    pub(super) tab: JnoccioTab,
    pub(super) cursor: usize,
    pub(super) paused: bool,
    pub(super) search_active: bool,
    pub(super) search_query: String,
    pub(super) help_open: bool,
    pub(super) sort_index: usize,
    pub(super) drawer_open: bool,
    /// Set to true by `Esc`/`q` so the App can exit the panel.
    pub(super) exit_requested: bool,
}

impl JnoccioPanel {
    /// Build a fresh panel with default UI state.
    pub fn new(snapshot: JnoccioSnapshot) -> Self {
        Self {
            snapshot,
            connection: JnoccioConnection::default(),
            tab: JnoccioTab::Board,
            cursor: 0,
            paused: false,
            search_active: false,
            search_query: String::new(),
            help_open: false,
            sort_index: 0,
            drawer_open: false,
            exit_requested: false,
        }
    }

    /// Replace the snapshot.
    pub fn set_snapshot(&mut self, snapshot: JnoccioSnapshot) {
        self.snapshot = snapshot;
    }

    /// Set the WS connection status.
    pub fn set_connection(&mut self, connection: JnoccioConnection) {
        self.connection = connection;
    }

    /// Return `(enabled_models, total_models)` from the current snapshot.
    /// Used by the header bar to display the live model count.
    pub fn snapshot_model_counts(&self) -> (u32, u32) {
        (self.snapshot.enabled_models, self.snapshot.total_models)
    }

    /// Currently-selected tab.
    pub fn tab(&self) -> JnoccioTab {
        self.tab
    }

    /// 0-indexed cursor offset within the active tab's list.
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// True when live updates are paused.
    pub fn paused(&self) -> bool {
        self.paused
    }

    /// True when the `?` overlay is visible.
    pub fn help_open(&self) -> bool {
        self.help_open
    }

    /// True when the `/` search prompt has focus.
    pub fn search_active(&self) -> bool {
        self.search_active
    }

    /// Current search buffer.
    pub fn search_query(&self) -> &str {
        &self.search_query
    }

    /// Set true after the user pressed `Esc`/`q` outside a sub-mode. The App is
    /// expected to pop the panel and reset this flag.
    pub fn exit_requested(&self) -> bool {
        self.exit_requested
    }

    /// Clear the exit flag once the App has reacted to it.
    pub fn clear_exit(&mut self) {
        self.exit_requested = false;
    }

    /// Move to a specific tab and reset the cursor + search.
    pub fn switch_tab(&mut self, tab: JnoccioTab) {
        self.tab = tab;
        self.cursor = 0;
        self.search_active = false;
        self.search_query.clear();
    }

    /// Next tab, wrapping.
    pub fn next_tab(&mut self) {
        let idx = self.tab.index();
        let next = JnoccioTab::ALL[(idx + 1) % JnoccioTab::ALL.len()];
        self.switch_tab(next);
    }

    /// Previous tab, wrapping.
    pub fn prev_tab(&mut self) {
        let idx = self.tab.index();
        let len = JnoccioTab::ALL.len();
        let prev = JnoccioTab::ALL[(idx + len - 1) % len];
        self.switch_tab(prev);
    }

    /// Toggle pause flag. Caller is expected to gate the WS event drain on
    /// this.
    pub fn toggle_pause(&mut self) {
        self.paused = !self.paused;
    }

    /// Toggle the `?` overlay.
    pub fn toggle_help(&mut self) {
        self.help_open = !self.help_open;
    }

    /// Rotate the sort mode (Board tab).
    pub fn cycle_sort(&mut self) {
        self.sort_index = (self.sort_index + 1) % SORT_MODES.len();
    }

    /// Active sort mode label.
    pub fn sort_label(&self) -> &'static str {
        SORT_MODES[self.sort_index]
    }

    /// Increment the calls counter.
    pub fn record_call(&mut self) {
        self.snapshot.calls += 1;
    }
}
