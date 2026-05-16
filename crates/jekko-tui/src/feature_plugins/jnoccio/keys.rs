//! Key dispatch for the Jnoccio panel.
//!
//! Top-level [`JnoccioPanel::dispatch_key`] picks the right sub-handler based
//! on which sub-mode (help overlay, drawer, search prompt) is currently
//! focused, then falls back to the main pane bindings.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::model::JnoccioTab;
use super::panel::JnoccioPanel;

impl JnoccioPanel {
    /// Handle a key event. Returns `true` when consumed.
    pub fn dispatch_key(&mut self, key: KeyEvent) -> bool {
        if self.help_open {
            return self.dispatch_help(key);
        }
        if self.drawer_open {
            return self.dispatch_drawer(key);
        }
        if self.search_active {
            return self.dispatch_search(key);
        }
        self.dispatch_main(key)
    }

    fn dispatch_help(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Char('?') | KeyCode::Esc => {
                self.help_open = false;
                true
            }
            _ => true,
        }
    }

    fn dispatch_drawer(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.drawer_open = false;
                true
            }
            _ => true,
        }
    }

    fn dispatch_search(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Esc => {
                self.search_active = false;
                self.search_query.clear();
                true
            }
            KeyCode::Enter => {
                self.search_active = false;
                true
            }
            KeyCode::Backspace => {
                self.search_query.pop();
                true
            }
            KeyCode::Char(ch) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.search_query.push(ch);
                true
            }
            _ => true,
        }
    }

    fn dispatch_main(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.exit_requested = true;
                true
            }
            KeyCode::Char('?') => {
                self.toggle_help();
                true
            }
            KeyCode::Char('/') => {
                self.search_active = true;
                self.search_query.clear();
                true
            }
            KeyCode::Char('p') => {
                self.toggle_pause();
                true
            }
            KeyCode::Char('s') => {
                self.cycle_sort();
                true
            }
            KeyCode::Enter => {
                if !matches!(self.tab, JnoccioTab::Feed | JnoccioTab::Agents) {
                    self.drawer_open = true;
                }
                true
            }
            KeyCode::Char(c @ '1'..='6') => {
                let idx = (c as usize) - ('1' as usize);
                if let Some(tab) = JnoccioTab::ALL.get(idx) {
                    self.switch_tab(*tab);
                }
                true
            }
            KeyCode::Tab | KeyCode::Right => {
                if key.modifiers.contains(KeyModifiers::SHIFT) {
                    self.prev_tab();
                } else {
                    self.next_tab();
                }
                true
            }
            KeyCode::BackTab | KeyCode::Left => {
                self.prev_tab();
                true
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.cursor = self.cursor.saturating_add(1);
                true
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.cursor = self.cursor.saturating_sub(1);
                true
            }
            KeyCode::Char('g') if !key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.cursor = 0;
                true
            }
            KeyCode::Char('G') => {
                self.cursor = usize::MAX;
                true
            }
            _ => false,
        }
    }
}
