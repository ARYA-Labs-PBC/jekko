//! Key-event handling for `QuestionCard`.

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use super::model::{QuestionCard, QuestionEvent, QuestionMode};

impl QuestionCard {
    /// Process a key event. Returns an event when the user submits/cancels
    /// or toggles editing.
    pub fn handle_key(&mut self, evt: KeyEvent) -> Option<QuestionEvent> {
        if evt.kind == KeyEventKind::Release {
            return None;
        }
        if self.editing_custom {
            return self.handle_editing(evt);
        }
        match (evt.code, evt.modifiers) {
            (KeyCode::Up | KeyCode::Char('k'), _) => {
                self.move_cursor(-1);
                None
            }
            (KeyCode::Down | KeyCode::Char('j'), _) => {
                self.move_cursor(1);
                None
            }
            (KeyCode::Esc, _) => Some(QuestionEvent::Rejected),
            (KeyCode::Char(c), _) if c.is_ascii_digit() && c != '0' => {
                let total = self.total().min(9);
                let idx = (c as u8 - b'1') as usize;
                if idx < total {
                    self.cursor = idx;
                    self.select_current()
                } else {
                    None
                }
            }
            (KeyCode::Enter, _) => self.select_current(),
            _ => None,
        }
    }

    fn move_cursor(&mut self, delta: isize) {
        let total = self.total() as isize;
        if total == 0 {
            return;
        }
        let mut next = self.cursor as isize + delta;
        while next < 0 {
            next += total;
        }
        self.cursor = (next % total) as usize;
    }

    fn toggle_at(&mut self, idx: usize) {
        let label = if self.is_custom_slot(idx) {
            self.custom_text.clone()
        } else {
            self.options[idx].label.clone()
        };
        if label.is_empty() {
            return;
        }
        if let Some(pos) = self.picked.iter().position(|l| l == &label) {
            self.picked.remove(pos);
        } else {
            self.picked.push(label);
        }
    }

    fn pick_single(&mut self, label: String) -> QuestionEvent {
        self.picked.clear();
        self.picked.push(label.clone());
        QuestionEvent::Submitted {
            answers: vec![label],
        }
    }

    fn select_current(&mut self) -> Option<QuestionEvent> {
        if self.is_custom_slot(self.cursor) {
            // Custom slot: enter editing mode unless we already have text and
            // are in multi-mode (then toggle).
            match self.mode {
                QuestionMode::Multi if !self.custom_text.is_empty() => {
                    self.toggle_at(self.cursor);
                    None
                }
                _ => {
                    self.editing_custom = true;
                    Some(QuestionEvent::EditingChanged { editing: true })
                }
            }
        } else {
            let label = self.options[self.cursor].label.clone();
            match self.mode {
                QuestionMode::Single => Some(self.pick_single(label)),
                QuestionMode::Multi => {
                    self.toggle_at(self.cursor);
                    None
                }
            }
        }
    }

    fn handle_editing(&mut self, evt: KeyEvent) -> Option<QuestionEvent> {
        match (evt.code, evt.modifiers) {
            (KeyCode::Esc, _) => {
                self.editing_custom = false;
                Some(QuestionEvent::EditingChanged { editing: false })
            }
            (KeyCode::Enter, _) => {
                let text = self.custom_text.trim().to_string();
                self.editing_custom = false;
                if text.is_empty() {
                    return Some(QuestionEvent::EditingChanged { editing: false });
                }
                match self.mode {
                    QuestionMode::Single => Some(self.pick_single(text)),
                    QuestionMode::Multi => {
                        if !self.picked.iter().any(|l| l == &text) {
                            self.picked.push(text);
                        }
                        Some(QuestionEvent::EditingChanged { editing: false })
                    }
                }
            }
            (KeyCode::Backspace, _) => {
                self.custom_text.pop();
                None
            }
            (KeyCode::Char(c), m) if !m.contains(KeyModifiers::CONTROL) => {
                self.custom_text.push(c);
                None
            }
            _ => None,
        }
    }
}
