use std::time::{Duration, Instant};

use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};
use ratatui::Frame;

use super::Logo;

// ---------------------------------------------------------------------------
// Single-tone splash (Packet G snapshot surface, v1 layout).
// ---------------------------------------------------------------------------

/// Splash screen rendered during the brief boot window before the first route
/// paints. Ports `splash-screen.tsx`'s "JEKKO" + tagline + version layout.
///
/// Serves as the snapshot-test surface and the narrow-terminal compact render
/// for `SplashState`. The streaming production splash lives below in
/// [`SplashState`].
#[derive(Clone, Debug)]
pub struct Splash<'a> {
    pub tagline: &'a str,
    pub version: &'a str,
}

impl<'a> Splash<'a> {
    pub fn new(tagline: &'a str, version: &'a str) -> Self {
        Self { tagline, version }
    }
}

impl<'a> Widget for &Splash<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(30),
                Constraint::Length(5),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Min(0),
            ])
            .split(area);

        (&Logo).render(chunks[1], buf);

        Paragraph::new(Span::styled(
            self.tagline.to_string(),
            Style::default()
                .fg(Color::Rgb(0xd8, 0xde, 0xe9))
                .add_modifier(Modifier::ITALIC),
        ))
        .alignment(Alignment::Center)
        .render(chunks[2], buf);

        Paragraph::new(Line::from(Span::styled(
            self.version.to_string(),
            Style::default().fg(Color::Rgb(0x7d, 0x85, 0x90)),
        )))
        .alignment(Alignment::Center)
        .render(chunks[3], buf);
    }
}

// ---------------------------------------------------------------------------
// Streaming 2-pane production splash.
// ---------------------------------------------------------------------------

/// Retained for forward-compatibility with callers that import this constant.
/// No longer participates in the dismiss decision — splash now blocks until
/// the boot stream reaches `All systems Ready`. Update at your own risk; the
/// real gate is [`SplashState::ready_to_dismiss`].
#[deprecated(note = "splash dismiss is now boot-stream driven; see ready_to_dismiss")]
pub const MIN_HOLD: Duration = Duration::from_millis(800);

/// Safety bail. The dismiss decision is normally driven by the boot stream
/// reaching its final line + [`FINAL_FLOURISH_HOLD`]; this caps the worst
/// case if step cadence stalls (e.g. system clock jumps).
pub const MAX_HOLD: Duration = Duration::from_millis(15_000);

/// How long the final "All systems Ready" line holds on screen after lighting
/// up, before the splash dismisses. Gives the success flourish a beat.
pub const FINAL_FLOURISH_HOLD: Duration = Duration::from_millis(450);

/// Cadence at which `tick()` advances to the next boot-log line. Mirrors
/// `STEP_MS` in `splash-screen.tsx`.
pub const STEP_CADENCE: Duration = Duration::from_millis(275);

/// Cadence at which `tick()` rotates the spinner glyph. Slightly faster than
/// step cadence so the active line looks alive.
pub const SPINNER_CADENCE: Duration = Duration::from_millis(120);

const SPINNER_FRAMES: &[char] = &[
    '\u{280B}', // ⠋
    '\u{2819}', // ⠙
    '\u{2839}', // ⠹
    '\u{2838}', // ⠸
    '\u{283C}', // ⠼
    '\u{2834}', // ⠴
    '\u{2826}', // ⠦
    '\u{2827}', // ⠧
    '\u{2807}', // ⠇
    '\u{280F}', // ⠏
];

/// A line in the canonical boot script. `phase` is the left column, `message`
/// is the right column. Mirrors `BOOT_SCRIPT` from `splash-screen.tsx`.
#[derive(Clone, Copy, Debug)]
struct BootLine {
    phase: &'static str,
    message: &'static str,
}

const BOOT_STEPS: &[BootLine] = &[
    BootLine {
        phase: "runtime",
        message: "initialized",
    },
    BootLine {
        phase: "plugins",
        message: "hydrated",
    },
    BootLine {
        phase: "workspace",
        message: "indexed",
    },
    BootLine {
        phase: "daemon",
        message: "connected",
    },
    BootLine {
        phase: "sync",
        message: "ready",
    },
    BootLine {
        phase: "jnoccio",
        message: "detected",
    },
    BootLine {
        phase: "jankurai",
        message: "watching score",
    },
    BootLine {
        phase: "All systems",
        message: "Ready",
    },
];

const PHASE_COL_WIDTH: usize = 12;
const MESSAGE_COL_WIDTH: usize = 16;

const NEVERHUMAN: &str = "N \u{00B7} E \u{00B7} V \u{00B7} E \u{00B7} R \u{00B7} H \u{00B7} U \u{00B7} M \u{00B7} A \u{00B7} N";
const TAGLINE: &str = "the agentic gecko \u{00B7} climbs hard problems";

// Theme — refined-amber dark. Mirrors `crates/jekko-core/src/theme`.
const AMBER: Color = Color::Rgb(0xd4, 0xa8, 0x43);
const AMBER_DIM: Color = Color::Rgb(0x8a, 0x6c, 0x2b);
const TEXT: Color = Color::Rgb(0xd8, 0xde, 0xe9);
const TEXT_MUTED: Color = Color::Rgb(0x7d, 0x85, 0x90);
const TEXT_DIM: Color = Color::Rgb(0x4a, 0x50, 0x5a);
const SUCCESS: Color = Color::Rgb(0x84, 0xc4, 0x69);
const WARN: Color = Color::Rgb(0xe0, 0xc3, 0x60);

/// Streaming splash state. Owned by `App`. Tick each frame, then call
/// [`SplashState::ready_to_dismiss`] to know whether the splash window has
/// expired.
#[derive(Debug)]
pub struct SplashState {
    started_at: Instant,
    step_index: usize,
    last_step_at: Instant,
    spinner_frame: u8,
    last_spinner_at: Instant,
}

impl SplashState {
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            started_at: now,
            step_index: 1,
            last_step_at: now,
            spinner_frame: 0,
            last_spinner_at: now,
        }
    }

    pub fn tick(&mut self) {
        let now = Instant::now();
        if now.duration_since(self.last_spinner_at) >= SPINNER_CADENCE {
            self.spinner_frame = self.spinner_frame.wrapping_add(1);
            self.last_spinner_at = now;
        }
        if self.step_index < BOOT_STEPS.len()
            && now.duration_since(self.last_step_at) >= STEP_CADENCE
        {
            self.step_index += 1;
            self.last_step_at = now;
        }
    }

    pub fn elapsed(&self) -> Duration {
        self.started_at.elapsed()
    }

    /// Dismiss once the boot stream reaches its final line and the success
    /// flourish has held for [`FINAL_FLOURISH_HOLD`]. `app_ready` is no longer
    /// gated — the splash blocks app entry until "All systems Ready" lights up
    /// and gets a brief moment on screen. `MAX_HOLD` remains as a safety bail
    /// in case the step cadence stalls.
    pub fn ready_to_dismiss(&self, _app_ready: bool) -> bool {
        if self.elapsed() >= MAX_HOLD {
            return true;
        }
        if self.step_index < BOOT_STEPS.len() {
            return false;
        }
        // step_index reached the end. last_step_at was bumped when we lit the
        // final line; hold for FINAL_FLOURISH_HOLD so the user actually sees
        // the success flourish before we cut.
        self.last_step_at.elapsed() >= FINAL_FLOURISH_HOLD
    }

    fn spinner_glyph(&self) -> char {
        SPINNER_FRAMES
            .get((self.spinner_frame as usize) % SPINNER_FRAMES.len())
            .copied()
            .unwrap_or('*')
    }

    pub fn render(&self, frame: &mut Frame<'_>, area: Rect) {
        if area.width < 40 || area.height < 8 {
            self.render_compact(frame, area);
            return;
        }
        if area.width < 80 {
            self.render_narrow(frame, area);
            return;
        }
        self.render_two_pane(frame, area);
    }

    fn render_compact(&self, frame: &mut Frame<'_>, area: Rect) {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),
                Constraint::Length(4),
                Constraint::Length(1),
                Constraint::Min(0),
            ])
            .split(area);
        frame.render_widget(&Logo::ascii(), rows[1]);
        let hint = Paragraph::new(Span::styled(
            "starting\u{2026}",
            Style::default().fg(TEXT_MUTED),
        ))
        .alignment(Alignment::Center);
        frame.render_widget(hint, rows[2]);
    }

    fn render_narrow(&self, frame: &mut Frame<'_>, area: Rect) {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2),
                Constraint::Length(2),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .split(area);
        frame.render_widget(self.wordmark_paragraph(), rows[0]);
        frame.render_widget(self.wordmark_underline_paragraph(), rows[1]);
        frame.render_widget(self.boot_log_paragraph(), rows[2]);
        frame.render_widget(self.footer_paragraph(), rows[3]);
    }

    fn render_two_pane(&self, frame: &mut Frame<'_>, area: Rect) {
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(area);

        let left_inner = pad(columns[0], 2, 1);
        let right_inner = pad(columns[1], 2, 1);

        let left_rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .split(left_inner);
        frame.render_widget(self.boot_log_paragraph(), left_rows[0]);
        frame.render_widget(self.footer_paragraph(), left_rows[1]);

        let right_rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Min(0),
            ])
            .split(right_inner);
        frame.render_widget(self.wordmark_paragraph(), right_rows[1]);
        frame.render_widget(self.wordmark_underline_paragraph(), right_rows[2]);
        frame.render_widget(self.tagline_paragraph(), right_rows[4]);
        frame.render_widget(self.loading_paragraph(), right_rows[6]);
    }

    fn wordmark_paragraph(&self) -> Paragraph<'static> {
        Paragraph::new(Line::from(Span::styled(
            NEVERHUMAN,
            Style::default().fg(TEXT).add_modifier(Modifier::BOLD),
        )))
        .alignment(Alignment::Center)
    }

    fn wordmark_underline_paragraph(&self) -> Paragraph<'static> {
        let underline = "\u{2594}".repeat(NEVERHUMAN.chars().count());
        Paragraph::new(Line::from(Span::styled(
            underline,
            Style::default().fg(AMBER),
        )))
        .alignment(Alignment::Center)
    }

    fn tagline_paragraph(&self) -> Paragraph<'static> {
        Paragraph::new(Line::from(Span::styled(
            TAGLINE,
            Style::default()
                .fg(TEXT_MUTED)
                .add_modifier(Modifier::ITALIC),
        )))
        .alignment(Alignment::Center)
    }

    fn loading_paragraph(&self) -> Paragraph<'static> {
        let spinner = self.spinner_glyph().to_string();
        Paragraph::new(Line::from(vec![
            Span::styled(spinner, Style::default().fg(AMBER)),
            Span::raw(" "),
            Span::styled("loading\u{2026}", Style::default().fg(TEXT_MUTED)),
        ]))
        .alignment(Alignment::Center)
    }

    fn boot_log_paragraph(&self) -> Paragraph<'static> {
        let elapsed_ms = self.elapsed().as_millis();
        let spinner = self.spinner_glyph();
        let active = self.step_index.saturating_sub(1);
        let lines: Vec<Line<'static>> = BOOT_STEPS
            .iter()
            .enumerate()
            .map(|(idx, step)| {
                let is_done = idx < active;
                let is_active = idx == active;
                let is_final = idx == BOOT_STEPS.len() - 1;
                let (glyph, glyph_color) = if is_done {
                    let g = if is_final { '\u{25CF}' } else { '\u{2713}' };
                    (g, SUCCESS)
                } else if is_active {
                    (spinner, AMBER)
                } else {
                    (' ', TEXT_DIM)
                };
                let (phase_color, message_color) = if is_done {
                    (TEXT, TEXT_MUTED)
                } else if is_active {
                    (AMBER, TEXT)
                } else {
                    (TEXT_DIM, TEXT_DIM)
                };
                let ts = if is_done || is_active {
                    format_timestamp(elapsed_ms)
                } else {
                    "        ".to_string()
                };
                Line::from(vec![
                    Span::styled(ts, Style::default().fg(TEXT_MUTED)),
                    Span::raw(" "),
                    Span::styled("\u{25B8}", Style::default().fg(AMBER_DIM)),
                    Span::raw(" "),
                    Span::styled(
                        pad_end(step.phase, PHASE_COL_WIDTH),
                        Style::default().fg(phase_color),
                    ),
                    Span::raw(" "),
                    Span::styled(
                        pad_end(step.message, MESSAGE_COL_WIDTH),
                        Style::default().fg(message_color),
                    ),
                    Span::raw(" "),
                    Span::styled(glyph.to_string(), Style::default().fg(glyph_color)),
                ])
            })
            .collect();
        Paragraph::new(lines)
    }

    fn footer_paragraph(&self) -> Paragraph<'static> {
        let elapsed_ms = self.elapsed().as_millis();
        let pid = std::process::id();
        let version = env!("CARGO_PKG_VERSION");
        let text =
            format!("version v{version}  \u{00B7}  PID {pid}  \u{00B7}  cold start {elapsed_ms}ms");
        Paragraph::new(Line::from(Span::styled(text, Style::default().fg(WARN))))
    }
}

impl Default for SplashState {
    fn default() -> Self {
        Self::new()
    }
}

fn pad_end(value: &str, width: usize) -> String {
    let len = value.chars().count();
    if len >= width {
        return value.to_string();
    }
    let mut out = String::with_capacity(value.len() + width - len);
    out.push_str(value);
    for _ in 0..(width - len) {
        out.push(' ');
    }
    out
}

fn format_timestamp(elapsed_ms: u128) -> String {
    let secs = (elapsed_ms as f64) / 1000.0;
    format!("[+{secs:.2}s]")
}

fn pad(area: Rect, dx: u16, dy: u16) -> Rect {
    let width = area.width.saturating_sub(dx.saturating_mul(2));
    let height = area.height.saturating_sub(dy.saturating_mul(2));
    if width == 0 || height == 0 {
        return area;
    }
    Rect {
        x: area.x.saturating_add(dx),
        y: area.y.saturating_add(dy),
        width,
        height,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    #[test]
    fn defaults_to_first_step_running() {
        let state = SplashState::new();
        assert_eq!(state.step_index, 1);
        assert!(!state.ready_to_dismiss(false));
    }

    #[test]
    fn ready_blocked_until_final_boot_line() {
        let mut state = SplashState::new();
        // Even with a long elapsed window, dismiss is blocked while there
        // are still boot lines to advance through.
        state.started_at = Instant::now() - Duration::from_millis(900);
        assert!(
            !state.ready_to_dismiss(true),
            "step_index < BOOT_STEPS.len() still"
        );
        // Advance to the last line, then prove the flourish gate.
        state.step_index = BOOT_STEPS.len();
        state.last_step_at = Instant::now(); // just lit up
        assert!(
            !state.ready_to_dismiss(true),
            "FINAL_FLOURISH_HOLD has not elapsed yet"
        );
        state.last_step_at = Instant::now() - (FINAL_FLOURISH_HOLD + Duration::from_millis(50));
        assert!(state.ready_to_dismiss(true));
    }

    #[test]
    fn hard_cap_dismisses_even_when_stream_stalled() {
        let mut state = SplashState::new();
        // Pretend the step cadence stalled (e.g. clock jump). The hard cap
        // should still let the user out.
        state.started_at = Instant::now() - (MAX_HOLD + Duration::from_millis(100));
        assert!(state.ready_to_dismiss(false));
    }

    #[test]
    fn spinner_glyph_is_braille() {
        let state = SplashState::new();
        let g = state.spinner_glyph();
        assert!(SPINNER_FRAMES.contains(&g));
    }

    #[test]
    fn tick_advances_step_after_cadence() {
        let mut state = SplashState::new();
        state.last_step_at = Instant::now() - (STEP_CADENCE + Duration::from_millis(5));
        let before = state.step_index;
        state.tick();
        assert_eq!(state.step_index, before + 1);
    }

    #[test]
    fn tick_stops_at_final_step() {
        let mut state = SplashState::new();
        state.step_index = BOOT_STEPS.len();
        state.last_step_at = Instant::now() - (STEP_CADENCE * 4);
        state.tick();
        assert_eq!(state.step_index, BOOT_STEPS.len());
    }

    #[test]
    fn renders_two_pane_at_120x30() {
        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).expect("terminal");
        let state = SplashState::new();
        terminal
            .draw(|frame| state.render(frame, frame.area()))
            .expect("draw");
        let text = terminal.backend().to_string();
        assert!(text.contains("loading"));
        assert!(text.contains("runtime"));
    }

    #[test]
    fn renders_narrow_at_60x20() {
        let backend = TestBackend::new(60, 20);
        let mut terminal = Terminal::new(backend).expect("terminal");
        let state = SplashState::new();
        terminal
            .draw(|frame| state.render(frame, frame.area()))
            .expect("draw");
        let text = terminal.backend().to_string();
        assert!(text.contains("runtime"));
    }

    #[test]
    fn renders_compact_at_30x6() {
        let backend = TestBackend::new(30, 6);
        let mut terminal = Terminal::new(backend).expect("terminal");
        let state = SplashState::new();
        terminal
            .draw(|frame| state.render(frame, frame.area()))
            .expect("draw");
        let text = terminal.backend().to_string();
        assert!(text.contains("starting"));
    }
}
