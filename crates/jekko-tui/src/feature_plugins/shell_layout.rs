//! Section: Shell Layout
//!
//! Computes per-frame `AppRects` for the Shell route body slot.
//!
//! The chrome (header rows + footer) is laid out by `App::draw`; this module
//! only divides the body slot into the Reasoning pane (LEFT, flex) and the
//! Inspector pane (RIGHT, fixed-width), plus the bottom Composer strip.
//!
//! ```text
//! ┌──────────────────────────────────────────────┬────────────────────────┐
//! │ ╭─ Reasoning ──────────────── idle ─╮        │ ╭─ Fusion ───────────╮ │
//! │ │ You                               │        │ │ Models    78 / 79  │ │
//! │ │   Please run the audit…           │        │ │ Agents     0 / 0   │ │
//! │ │                                   │        │ │ Calls          0   │ │
//! │ │ Jekko · idle                      │        │ │                    │ │
//! │ │   Waiting to start. Press Enter.  │        │ │ [1] Board [2] Speed│ │
//! │ ╰───────────────────────────────────╯        │ ╰────────────────────╯ │
//! ├──────────────────────────────────────────────┴────────────────────────┤
//! │ ╭─ Prompt ─────────────────────────────────────────────── 0 chars ─╮  │
//! │ │ Ask Jekko…                                                        │  │
//! │ ╰───────────────────────────────────────────────────────────────────╯  │
//! └───────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! Inspector responsive widths (per spec):
//!
//! | terminal width | inspector width | notes |
//! |----------------|-----------------|-------|
//! | `< 110`        | hidden          | full-width Reasoning |
//! | `110..124`     | 36 cols         | compact |
//! | `125..159`     | 40 cols         | normal |
//! | `>= 160`       | 44 cols         | full |

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};
use ratatui::Frame;

use crate::app::App;
use crate::engagement::EngagementState;
use crate::feature_plugins::jankurai::{JankuraiPanel, JankuraiSnapshot};
use crate::feature_plugins::ShellTab;
use crate::theme;
use crate::transcript::route::render_transcript_window;

// ── Responsive thresholds ───────────────────────────────────────────────────

/// Minimum terminal width to show the inspector at all.
pub const INSPECTOR_HIDE_BELOW: u16 = 110;
/// Widths in which the inspector transitions from compact to normal.
pub const INSPECTOR_NORMAL_AT: u16 = 125;
/// Width at which the inspector reaches full size.
pub const INSPECTOR_FULL_AT: u16 = 160;

// ── Layout types ─────────────────────────────────────────────────────────────

/// Resolved per-frame rectangles for the Shell route body.
///
/// The header rows and footer are NOT included — those are handled by
/// `App::draw` before calling `compute()`.
#[derive(Clone, Copy, Debug)]
pub struct AppRects {
    /// LEFT reasoning/transcript pane (flex).
    pub reasoning: Rect,
    /// RIGHT inspector pane. `None` when terminal is too narrow or sidebar off.
    pub inspector: Option<Rect>,
    /// Bottom composer (prompt textarea).
    pub composer: Rect,
}

/// Compute Shell body rectangles for the given body slot.
///
/// `area` is `chrome[body]` from `App::draw` — everything below the 2-row
/// header and above the 1-row footer.
pub fn compute(area: Rect, sidebar_open: bool) -> AppRects {
    // Reserve composer at the bottom (4 rows: border + 2 content + border).
    let composer_rows: u16 = 4;
    let reasoning_min: u16 = 1;
    let needed = composer_rows.saturating_add(reasoning_min);
    let (body_area, composer_area) = if area.height >= needed {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(reasoning_min), Constraint::Length(composer_rows)])
            .split(area);
        (rows[0], rows[1])
    } else {
        (
            Rect { x: area.x, y: area.y, width: area.width, height: 0 },
            area,
        )
    };

    let inspector_width = inspector_width_for(area.width, sidebar_open);
    let (reasoning, inspector) = match inspector_width {
        Some(w) if w < body_area.width => {
            // 1-column gutter between the two panes.
            let gutter: u16 = 1;
            let inspector_with_gutter = w + gutter;
            if inspector_with_gutter >= body_area.width {
                // No room for gutter — collapse inspector.
                (body_area, None)
            } else {
                let cols = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Min(0),
                        Constraint::Length(gutter),
                        Constraint::Length(w),
                    ])
                    .split(body_area);
                (cols[0], Some(cols[2]))
            }
        }
        _ => (body_area, None),
    };

    AppRects { reasoning, inspector, composer: composer_area }
}

/// Resolve inspector width, honouring the sidebar toggle.
pub fn inspector_width_for(total_width: u16, sidebar_open: bool) -> Option<u16> {
    if !sidebar_open || total_width < INSPECTOR_HIDE_BELOW {
        return None;
    }
    if total_width >= INSPECTOR_FULL_AT {
        Some(44)
    } else if total_width >= INSPECTOR_NORMAL_AT {
        Some(40)
    } else {
        Some(36)
    }
}

// ── Pane renderers ───────────────────────────────────────────────────────────

/// Paint the LEFT Reasoning pane.
///
/// When the transcript is empty, renders the JEKKO logo + engage hint with
/// the empty-state slide animation. When non-empty, renders the transcript
/// scroll window inside a `Reasoning` panel block.
pub fn render_reasoning_pane(frame: &mut Frame, area: Rect, app: &App) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    frame.render_widget(ratatui::widgets::Clear, area);

    if app.transcript.is_empty() {
        render_empty_reasoning(frame, area, app.engagement);
        return;
    }

    // Reasoning panel block with status in title-bottom.
    let status = if app.is_audit_running {
        "audit"
    } else if app.is_streaming {
        "streaming"
    } else {
        "idle"
    };
    let focused = app.prompt_focused; // transcript pane focused when prompt not focused
    let block = theme::panel_block("Reasoning", Some(status), !focused);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let buf = frame.buffer_mut();
    render_transcript_window(&app.transcript, inner, buf);
}

/// Paint the RIGHT Inspector pane.
pub fn render_inspector_pane(frame: &mut Frame, area: Rect, app: &App) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    match app.shell_tab {
        ShellTab::Jnoccio => {
            frame.render_widget(&app.jnoccio_panel, area);
        }
        ShellTab::RepoIntel => {
            let panel = JankuraiPanel::new(JankuraiSnapshot::default());
            frame.render_widget(&panel, area);
        }
        ShellTab::History => render_history_inspector(frame, area),
    }
}

/// History inspector: shows saved-session list (or empty state).
fn render_history_inspector(frame: &mut Frame, area: Rect) {
    let focused = false;
    let block = theme::panel_block("History", None, focused);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let body = vec![
        Line::from(Span::styled("No saved sessions yet.", Style::default().fg(theme::TEXT))),
        Line::from(Span::raw("")),
        Line::from(Span::styled(
            "Start a session to populate this list.",
            Style::default().fg(theme::TEXT_MUTED),
        )),
    ];
    frame.render_widget(Paragraph::new(body).wrap(Wrap { trim: false }), inner);
}

// ── Empty-state ───────────────────────────────────────────────────────────────

const LOGO_LINES: u16 = 5;
const LOGO_HINT_GAP: u16 = 2;
const LOGO_SLIDE_DISTANCE: u16 = LOGO_LINES + LOGO_HINT_GAP;

/// Paint the empty-state Reasoning area: JEKKO logo + engage hint.
pub fn render_empty_reasoning(frame: &mut Frame, area: Rect, engagement: EngagementState) {
    use ratatui::layout::Alignment;

    if engagement.is_engaged() {
        return;
    }

    let stack = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(0),
            Constraint::Length(LOGO_LINES),
            Constraint::Length(LOGO_HINT_GAP),
            Constraint::Length(2),
            Constraint::Min(0),
        ])
        .split(area);

    let logo_slot = stack[1];
    let hint_slot = stack[3];

    let progress = engagement.slide_progress();
    let offset_rows = (progress * LOGO_SLIDE_DISTANCE as f32).floor() as u16;

    if offset_rows < LOGO_SLIDE_DISTANCE {
        let intended_y = (logo_slot.y as i32) - (offset_rows as i32);
        let area_top = area.y as i32;
        let (draw_y, clip_top) = if intended_y >= area_top {
            (intended_y as u16, 0u16)
        } else {
            (area.y, (area_top - intended_y) as u16)
        };
        let remaining = LOGO_LINES.saturating_sub(clip_top);
        if remaining > 0 {
            let translated = Rect {
                x: logo_slot.x,
                y: draw_y,
                width: logo_slot.width,
                height: remaining,
            };
            let builder = crate::components::LogoBuilder::default_face()
                .with_alignment(Alignment::Left);
            frame.render_widget(&builder, translated);
        }
    }

    if hint_slot.height >= 2 {
        let hint_color = if engagement.is_engaging() {
            Color::Rgb(0x6a, 0x54, 0x21)
        } else {
            theme::TEXT_MUTED
        };
        let primary = Paragraph::new(Line::from(Span::styled(
            "Press Enter to engage",
            Style::default().fg(theme::TEXT).add_modifier(Modifier::BOLD),
        )))
        .alignment(Alignment::Left);
        let secondary = Paragraph::new(Line::from(Span::styled(
            "Type and press Enter to send",
            Style::default().fg(hint_color),
        )))
        .alignment(Alignment::Left);

        let hint_rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Length(1)])
            .split(hint_slot);
        frame.render_widget(primary, hint_rows[0]);
        frame.render_widget(secondary, hint_rows[1]);
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inspector_hidden_when_narrow() {
        assert_eq!(inspector_width_for(109, true), None);
        assert_eq!(inspector_width_for(0, true), None);
    }

    #[test]
    fn inspector_widths_match_spec() {
        assert_eq!(inspector_width_for(110, true), Some(36));
        assert_eq!(inspector_width_for(124, true), Some(36));
        assert_eq!(inspector_width_for(125, true), Some(40));
        assert_eq!(inspector_width_for(159, true), Some(40));
        assert_eq!(inspector_width_for(160, true), Some(44));
        assert_eq!(inspector_width_for(240, true), Some(44));
    }

    #[test]
    fn inspector_respects_sidebar_toggle() {
        assert_eq!(inspector_width_for(200, false), None);
    }

    #[test]
    fn compute_reasoning_fills_when_inspector_hidden() {
        let area = Rect::new(0, 0, 100, 20);
        let layout = compute(area, true);
        assert!(layout.inspector.is_none());
        assert_eq!(layout.reasoning.width, 100);
    }

    #[test]
    fn compute_splits_reasoning_and_inspector() {
        let area = Rect::new(0, 0, 160, 20);
        let layout = compute(area, true);
        assert!(layout.inspector.is_some());
        let insp = layout.inspector.unwrap();
        assert_eq!(insp.width, 44);
        // reasoning + gutter + inspector = total width
        assert_eq!(layout.reasoning.width + 1 + insp.width, 160);
    }

    #[test]
    fn compute_composer_at_bottom() {
        let area = Rect::new(0, 0, 140, 20);
        let layout = compute(area, true);
        assert_eq!(layout.composer.height, 4);
        assert_eq!(layout.composer.y, area.y + area.height - 4);
    }

    #[test]
    fn compute_degenerate_short_terminal() {
        let layout = compute(Rect::new(0, 0, 200, 3), true);
        assert_eq!(layout.composer.height, 3);
        assert_eq!(layout.reasoning.height, 0);
    }
}
