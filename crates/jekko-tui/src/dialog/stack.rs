use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::Widget;

use super::command::CommandPalette;
use super::select::SelectDialog;

/// Type-tagged enum of all dialog kinds. Component-specific dialogs (model,
/// provider, etc.) compose `SelectDialog`/`CommandPalette` so they don't need
/// their own variants in this enum.
#[derive(Clone, Debug)]
pub enum Dialog {
    Select(SelectDialog),
    Command(CommandPalette),
}

impl Dialog {
    /// Width × height tuple, used by callers that want to know the dialog
    /// footprint before deciding whether to suppress underlying chrome.
    pub fn footprint(&self) -> (u16, u16) {
        match self {
            Dialog::Select(d) => (d.width, d.height),
            Dialog::Command(_) => (64, 18),
        }
    }
}

impl Widget for &Dialog {
    fn render(self, area: Rect, buf: &mut Buffer) {
        match self {
            Dialog::Select(d) => d.render(area, buf),
            Dialog::Command(c) => c.render(area, buf),
        }
    }
}

/// Modal stack. The top of the stack receives input; lower dialogs are not
/// drawn.
#[derive(Clone, Debug, Default)]
pub struct DialogStack {
    items: Vec<Dialog>,
}

impl DialogStack {
    pub fn push(&mut self, d: Dialog) {
        self.items.push(d);
    }

    pub fn pop(&mut self) -> Option<Dialog> {
        self.items.pop()
    }

    pub fn top(&self) -> Option<&Dialog> {
        self.items.last()
    }

    pub fn top_mut(&mut self) -> Option<&mut Dialog> {
        self.items.last_mut()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }
}

impl Widget for &DialogStack {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if let Some(top) = self.top() {
            top.render(area, buf);
        }
    }
}
