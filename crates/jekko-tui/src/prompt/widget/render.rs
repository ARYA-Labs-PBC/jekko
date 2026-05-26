//! `Widget` implementation for the `Prompt` composite.
//!
//! Paints the `›` gutter, defers body drawing to the embedded `TextArea`, and
//! overlays the optional right-aligned model label. Split from `widget.rs` to
//! keep the parent under the LOC budget.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Style, Stylize};
use ratatui::widgets::Widget;

use super::{prompt_glyph, Prompt, PROMPT_PREFIX_WIDTH};
use crate::theme::codex::BLUE_PATH;

impl Widget for &Prompt {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        // Paint the `›` prefix gutter (column 0) + space (column 1) on every
        // text row. The first row gets the blue `›` glyph; continuation rows
        // get blank columns so wrapped/multi-line text aligns under the first
        // body character.
        //
        // We only paint into the gutter if the area is wide enough to leave
        // at least one body column. Below that, fall back to the legacy
        // gutter-less render so we don't lose all editable space on a 1-col
        // sliver.
        if area.width <= PROMPT_PREFIX_WIDTH {
            Widget::render(&self.textarea, area, buf);
            self.render_model_label(area, buf);
            return;
        }

        let prefix_style = Style::default().fg(BLUE_PATH);
        let blank_style = Style::default();
        let glyph = prompt_glyph();
        for row_offset in 0..area.height {
            let y = area.y + row_offset;
            if row_offset == 0 {
                buf.set_string(area.x, y, glyph, prefix_style);
                buf.set_string(area.x + 1, y, " ", blank_style);
            } else {
                buf.set_string(area.x, y, "  ", blank_style);
            }
        }

        // Render the textarea into the inner area (shifted right by the
        // prefix width). `tui_textarea` owns its own viewport and cursor
        // placement; because we shifted `area.x`, the cursor it emits will
        // also land in the shifted region automatically.
        let inner = Rect {
            x: area.x + PROMPT_PREFIX_WIDTH,
            y: area.y,
            width: area.width - PROMPT_PREFIX_WIDTH,
            height: area.height,
        };
        Widget::render(&self.textarea, inner, buf);

        self.render_model_label(area, buf);
    }
}

impl Prompt {
    /// Paint the optional right-aligned model label (e.g. `claude-opus-4-7`).
    ///
    /// Drawn on the first row of the prompt area, right-justified inside the
    /// full render rect (not the post-prefix inner rect) so the label hugs the
    /// far right column regardless of prefix width.
    pub(super) fn render_model_label(&self, area: Rect, buf: &mut Buffer) {
        let Some(label) = &self.model_label else {
            return;
        };
        let label_width = label.chars().count() as u16;
        if label_width == 0 || label_width >= area.width {
            return;
        }
        let x = area.x + area.width - label_width;
        // Dim the label so it doesn't compete with the active composer text.
        buf.set_string(x, area.y, label, Style::default().dim());
    }
}
