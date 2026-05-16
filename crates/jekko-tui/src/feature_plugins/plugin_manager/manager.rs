//! Plugin manager state machine and key handling.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::row::{PluginRow, PluginRowKind};

/// Plugin manager dialog.
#[derive(Clone, Debug)]
pub struct PluginManager {
    pub(super) rows: Vec<PluginRow>,
    pub(super) cursor: usize,
    /// Set to true when the user pressed `Shift+I` to request an install.
    install_requested: bool,
    /// Set to true when the user pressed `Space` or `Enter` on a row.
    toggle_requested: bool,
    /// Set to true when the user pressed `Esc`/`q`.
    exit_requested: bool,
    /// Dialog width.
    pub(super) width: u16,
    /// Dialog height.
    pub(super) height: u16,
}

impl PluginManager {
    /// Build a manager around a row list.
    pub fn new(rows: Vec<PluginRow>) -> Self {
        let height = (rows.len().min(12) as u16).saturating_add(6).max(8);
        Self {
            rows,
            cursor: 0,
            install_requested: false,
            toggle_requested: false,
            exit_requested: false,
            width: 80,
            height,
        }
    }

    /// Replace the row list. Cursor is clamped to the new length.
    pub fn set_rows(&mut self, rows: Vec<PluginRow>) {
        self.rows = rows;
        if self.rows.is_empty() {
            self.cursor = 0;
        } else if self.cursor >= self.rows.len() {
            self.cursor = self.rows.len() - 1;
        }
    }

    /// Sort rows so internal plugins come first, then by id.
    pub fn sort(&mut self) {
        self.rows.sort_by(|a, b| match (a.kind, b.kind) {
            (PluginRowKind::Internal, PluginRowKind::External) => std::cmp::Ordering::Less,
            (PluginRowKind::External, PluginRowKind::Internal) => std::cmp::Ordering::Greater,
            _ => a.id.cmp(&b.id),
        });
    }

    /// Read-only access to the rows.
    pub fn rows(&self) -> &[PluginRow] {
        &self.rows
    }

    /// Current cursor index (0..rows.len()).
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// Currently-highlighted row.
    pub fn selected(&self) -> Option<&PluginRow> {
        self.rows.get(self.cursor)
    }

    /// True after the user pressed `Shift+I`.
    pub fn install_requested(&self) -> bool {
        self.install_requested
    }

    /// Clear the install request flag.
    pub fn clear_install(&mut self) {
        self.install_requested = false;
    }

    /// True after the user pressed `Space` or `Enter`.
    pub fn toggle_requested(&self) -> bool {
        self.toggle_requested
    }

    /// Clear the toggle flag.
    pub fn clear_toggle(&mut self) {
        self.toggle_requested = false;
    }

    /// True after the user pressed `Esc` or `q`.
    pub fn exit_requested(&self) -> bool {
        self.exit_requested
    }

    /// Clear the exit flag.
    pub fn clear_exit(&mut self) {
        self.exit_requested = false;
    }

    /// Move the cursor by a signed delta, wrapping around.
    pub fn move_cursor(&mut self, delta: isize) {
        if self.rows.is_empty() {
            self.cursor = 0;
            return;
        }
        let len = self.rows.len() as isize;
        let mut idx = self.cursor as isize + delta;
        while idx < 0 {
            idx += len;
        }
        self.cursor = (idx % len) as usize;
    }

    /// Dispatch a key. Returns `true` when consumed.
    pub fn dispatch_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.exit_requested = true;
                true
            }
            KeyCode::Char('I') if key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.install_requested = true;
                true
            }
            KeyCode::Char('i') if key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.install_requested = true;
                true
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.move_cursor(1);
                true
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.move_cursor(-1);
                true
            }
            KeyCode::Char('g') if !key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.cursor = 0;
                true
            }
            KeyCode::Char('G') => {
                if !self.rows.is_empty() {
                    self.cursor = self.rows.len() - 1;
                }
                true
            }
            KeyCode::Char(' ') | KeyCode::Enter => {
                if !self.rows.is_empty() {
                    self.toggle_requested = true;
                }
                true
            }
            _ => false,
        }
    }
}
