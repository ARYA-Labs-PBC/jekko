use std::time::{Duration, Instant};

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::Span;
use ratatui::widgets::{Paragraph, Widget};

const FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
const FRAME_PERIOD: Duration = Duration::from_millis(80);

/// Simplified 10-frame Braille spinner. Routes that need richer animation can
/// replace it later.
#[derive(Clone, Debug)]
pub struct Spinner {
    pub started_at: Instant,
    pub color: Color,
}

impl Spinner {
    pub fn now() -> Self {
        Self {
            started_at: Instant::now(),
            color: Color::Rgb(0xd4, 0xa8, 0x43),
        }
    }

    fn frame(&self) -> &'static str {
        let idx = (self.started_at.elapsed().as_millis() / FRAME_PERIOD.as_millis()) as usize
            % FRAMES.len();
        FRAMES[idx]
    }
}

impl Widget for &Spinner {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let span = Span::styled(self.frame().to_string(), Style::default().fg(self.color));
        Paragraph::new(span).render(area, buf);
    }
}
