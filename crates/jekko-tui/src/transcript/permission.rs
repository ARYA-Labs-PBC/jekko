//! Inline permission card.
//!
//! Ports `packages/jekko/src/cli/cmd/tui/routes/session/permission.tsx`. The
//! TS layer rides on a reactive store with three stages (`permission`,
//! `always`, `reject`). Here we expose the same state machine plus a simple
//! key-event dispatcher that yields a [`PermissionDecisionEvent`] when the
//! user confirms.
//!
//! The decision payload is intentionally a structural type rather than a
//! direct re-export of `jekko_runtime::permission::PermissionReply` so the
//! TUI crate stays decoupled from the runtime crate; callers in
//! `jekko-cli` / `jekko-server` translate the choice when forwarding the
//! reply over the bus.

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget, Wrap};

const COLOR_TEXT: Color = Color::Rgb(0xd8, 0xde, 0xe9);
const COLOR_TEXT_MUTED: Color = Color::Rgb(0x7d, 0x85, 0x90);
const COLOR_WARN: Color = Color::Rgb(0xf5, 0xa6, 0x23);
const COLOR_ERROR: Color = Color::Rgb(0xe0, 0x6c, 0x75);
const COLOR_PANEL: Color = Color::Rgb(0x12, 0x15, 0x1c);

/// The decision the user picked.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum PermissionChoice {
    /// Allow this single invocation only.
    Once,
    /// Allow this invocation and persist patterns until restart.
    Always,
    /// Reject. May carry a free-form follow-up message.
    Reject,
}

/// Stage of the inline permission flow.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum PermissionStage {
    /// Main choose-one stage with three buttons.
    Choose,
    /// Confirm-`always` substage.
    ConfirmAlways,
    /// Free-form rejection stage with an inline textarea.
    Reject,
}

/// Coarse permission kind. Caller may supply a more specific subtype via
/// [`PermissionCard::detail`].
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum PermissionAction {
    /// `edit` permission.
    Edit,
    /// `read` permission.
    Read,
    /// `shell` permission.
    Shell,
    /// `glob`/`grep`/`list` family.
    Search,
    /// `webfetch`/`websearch`/`research`.
    Network,
    /// External directory access.
    External,
    /// `task`/subagent permission.
    Task,
    /// `doom_loop` continuation.
    DoomLoop,
    /// Anything else.
    Other,
}

impl PermissionAction {
    fn icon(self) -> &'static str {
        match self {
            PermissionAction::Edit => "→",
            PermissionAction::Read => "→",
            PermissionAction::Shell => "#",
            PermissionAction::Search => "✱",
            PermissionAction::Network => "%",
            PermissionAction::External => "←",
            PermissionAction::Task => "#",
            PermissionAction::DoomLoop => "⟳",
            PermissionAction::Other => "⚙",
        }
    }
}

/// Event yielded by [`PermissionCard::handle_key`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PermissionDecisionEvent {
    /// User confirmed a choice — caller forwards over the runtime bus.
    Decided {
        /// The selected choice.
        choice: PermissionChoice,
        /// Free-form message (only set with `Reject`).
        message: Option<String>,
    },
    /// User cancelled out without deciding.
    Cancelled,
    /// User opened the `Always` confirmation substage.
    EnteredAlwaysStage,
    /// User opened the `Reject` substage.
    EnteredRejectStage,
}

/// Inline permission card state.
#[derive(Clone, Debug)]
pub struct PermissionCard {
    /// Stable request id passed back to the runtime on reply.
    pub request_id: String,
    /// Action kind.
    pub action: PermissionAction,
    /// Short, human-readable title (e.g. `"Edit foo.txt"`).
    pub title: String,
    /// Optional secondary line (path, command, etc.).
    pub detail: Option<String>,
    /// Patterns the runtime will persist on `Always`.
    pub patterns: Vec<String>,
    /// True when this request originates from a subagent — flips the
    /// `Reject` button into a "reason" sub-stage.
    pub subagent: bool,
    /// Current cursor over the three buttons (0=once, 1=always, 2=reject).
    pub cursor: u8,
    /// Stage of the flow.
    pub stage: PermissionStage,
    /// In-progress reject message (only populated when `stage == Reject`).
    pub reject_message: String,
}

impl PermissionCard {
    /// Build a new permission card.
    pub fn new(
        request_id: impl Into<String>,
        action: PermissionAction,
        title: impl Into<String>,
    ) -> Self {
        Self {
            request_id: request_id.into(),
            action,
            title: title.into(),
            detail: None,
            patterns: Vec::new(),
            subagent: false,
            cursor: 0,
            stage: PermissionStage::Choose,
            reject_message: String::new(),
        }
    }
    /// Attach a secondary detail line.
    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }
    /// Attach patterns that the runtime should persist on `Always`.
    pub fn with_patterns<I, S>(mut self, patterns: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.patterns = patterns.into_iter().map(Into::into).collect();
        self
    }
    /// Mark this card as originating from a subagent.
    pub fn for_subagent(mut self) -> Self {
        self.subagent = true;
        self
    }

    fn move_cursor(&mut self, delta: isize) {
        let len = 3isize;
        let mut next = self.cursor as isize + delta;
        while next < 0 {
            next += len;
        }
        self.cursor = (next % len) as u8;
    }

    fn current_choice(&self) -> PermissionChoice {
        match self.cursor {
            0 => PermissionChoice::Once,
            1 => PermissionChoice::Always,
            _ => PermissionChoice::Reject,
        }
    }

    /// Process a key event and optionally produce a decision event.
    pub fn handle_key(&mut self, evt: KeyEvent) -> Option<PermissionDecisionEvent> {
        if evt.kind == KeyEventKind::Release {
            return None;
        }
        match self.stage {
            PermissionStage::Choose => self.handle_choose(evt),
            PermissionStage::ConfirmAlways => self.handle_confirm_always(evt),
            PermissionStage::Reject => self.handle_reject(evt),
        }
    }

    fn handle_choose(&mut self, evt: KeyEvent) -> Option<PermissionDecisionEvent> {
        match (evt.code, evt.modifiers) {
            (KeyCode::Left | KeyCode::Char('h'), _) => {
                self.move_cursor(-1);
                Some(PermissionDecisionEvent::Cancelled).filter(|_| false)
            }
            (KeyCode::Right | KeyCode::Char('l'), _) => {
                self.move_cursor(1);
                None
            }
            (KeyCode::Enter, _) => {
                let choice = self.current_choice();
                match choice {
                    PermissionChoice::Once => Some(PermissionDecisionEvent::Decided {
                        choice,
                        message: None,
                    }),
                    PermissionChoice::Always => {
                        self.stage = PermissionStage::ConfirmAlways;
                        Some(PermissionDecisionEvent::EnteredAlwaysStage)
                    }
                    PermissionChoice::Reject => {
                        if self.subagent {
                            self.stage = PermissionStage::Reject;
                            self.reject_message.clear();
                            Some(PermissionDecisionEvent::EnteredRejectStage)
                        } else {
                            Some(PermissionDecisionEvent::Decided {
                                choice,
                                message: None,
                            })
                        }
                    }
                }
            }
            (KeyCode::Esc, _) => Some(PermissionDecisionEvent::Decided {
                choice: PermissionChoice::Reject,
                message: None,
            }),
            _ => None,
        }
    }

    fn handle_confirm_always(&mut self, evt: KeyEvent) -> Option<PermissionDecisionEvent> {
        match (evt.code, evt.modifiers) {
            (KeyCode::Enter, _) => Some(PermissionDecisionEvent::Decided {
                choice: PermissionChoice::Always,
                message: None,
            }),
            (KeyCode::Esc, _) => {
                self.stage = PermissionStage::Choose;
                None
            }
            _ => None,
        }
    }

    fn handle_reject(&mut self, evt: KeyEvent) -> Option<PermissionDecisionEvent> {
        match (evt.code, evt.modifiers) {
            (KeyCode::Enter, _) => {
                let msg = std::mem::take(&mut self.reject_message);
                let trimmed = msg.trim();
                let message = if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed.to_string())
                };
                self.stage = PermissionStage::Choose;
                Some(PermissionDecisionEvent::Decided {
                    choice: PermissionChoice::Reject,
                    message,
                })
            }
            (KeyCode::Esc, _) => {
                self.stage = PermissionStage::Choose;
                self.reject_message.clear();
                None
            }
            (KeyCode::Backspace, _) => {
                self.reject_message.pop();
                None
            }
            (KeyCode::Char(c), m) if !m.contains(KeyModifiers::CONTROL) => {
                self.reject_message.push(c);
                None
            }
            _ => None,
        }
    }

    /// Snapshot for `insta`.
    pub fn snapshot(&self) -> String {
        format!(
            "permission[{}|{:?}|cursor={}|stage={:?}] {} detail={:?}",
            self.request_id, self.action, self.cursor, self.stage, self.title, self.detail
        )
    }
}

fn button_label(choice: PermissionChoice) -> &'static str {
    match choice {
        PermissionChoice::Once => "Allow once",
        PermissionChoice::Always => "Allow always",
        PermissionChoice::Reject => "Reject",
    }
}

impl Widget for &PermissionCard {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .borders(Borders::LEFT)
            .border_style(Style::default().fg(COLOR_WARN))
            .style(Style::default().bg(COLOR_PANEL));
        let inner = block.inner(area);
        block.render(area, buf);

        let mut lines: Vec<Line> = Vec::new();
        lines.push(Line::from(vec![
            Span::styled(" △ ", Style::default().fg(COLOR_WARN)),
            Span::styled(
                "Permission required",
                Style::default().fg(COLOR_TEXT).add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::raw("   "),
            Span::styled(self.action.icon(), Style::default().fg(COLOR_TEXT_MUTED)),
            Span::raw(" "),
            Span::styled(self.title.clone(), Style::default().fg(COLOR_TEXT)),
        ]));
        if let Some(detail) = &self.detail {
            lines.push(Line::from(Span::styled(
                format!("    {detail}"),
                Style::default().fg(COLOR_TEXT_MUTED),
            )));
        }
        match self.stage {
            PermissionStage::Choose => {
                lines.push(Line::from(Span::raw("")));
                lines.push(button_row(self.cursor));
                lines.push(hint_row("← →  navigate · enter  confirm · esc  reject"));
            }
            PermissionStage::ConfirmAlways => {
                lines.push(Line::from(Span::raw("")));
                if self.patterns.is_empty() {
                    lines.push(Line::from(Span::styled(
                        "    This will allow the action until Jekko is restarted.",
                        Style::default().fg(COLOR_TEXT_MUTED),
                    )));
                } else {
                    lines.push(Line::from(Span::styled(
                        "    The following patterns will be allowed until Jekko is restarted:",
                        Style::default().fg(COLOR_TEXT_MUTED),
                    )));
                    for pattern in &self.patterns {
                        lines.push(Line::from(Span::styled(
                            format!("    - {pattern}"),
                            Style::default().fg(COLOR_TEXT),
                        )));
                    }
                }
                lines.push(hint_row("enter  confirm · esc  back"));
            }
            PermissionStage::Reject => {
                lines.push(Line::from(Span::raw("")));
                lines.push(Line::from(vec![
                    Span::styled(" △ ", Style::default().fg(COLOR_ERROR)),
                    Span::styled(
                        "Tell Jekko what to do differently",
                        Style::default().fg(COLOR_TEXT),
                    ),
                ]));
                lines.push(Line::from(vec![
                    Span::styled("    > ", Style::default().fg(COLOR_TEXT_MUTED)),
                    Span::styled(self.reject_message.clone(), Style::default().fg(COLOR_TEXT)),
                    Span::styled(
                        "_",
                        Style::default()
                            .fg(COLOR_WARN)
                            .add_modifier(Modifier::SLOW_BLINK),
                    ),
                ]));
                lines.push(hint_row("enter  send · esc  back"));
            }
        }
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .render(inner, buf);
    }
}

fn button_row(cursor: u8) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = vec![Span::raw("    ")];
    for (i, choice) in [
        PermissionChoice::Once,
        PermissionChoice::Always,
        PermissionChoice::Reject,
    ]
    .iter()
    .enumerate()
    {
        let selected = cursor as usize == i;
        let fg = if selected {
            Color::Rgb(0x12, 0x15, 0x1c)
        } else {
            COLOR_TEXT_MUTED
        };
        let bg = if selected {
            COLOR_WARN
        } else {
            Color::Rgb(0x21, 0x26, 0x30)
        };
        spans.push(Span::styled(
            format!(" {} ", button_label(*choice)),
            Style::default().fg(fg).bg(bg).add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::raw(" "));
    }
    Line::from(spans)
}

fn hint_row(label: &str) -> Line<'static> {
    Line::from(Span::styled(
        format!("    {label}"),
        Style::default().fg(COLOR_TEXT_MUTED),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn cursor_wraps_right() {
        let mut card = PermissionCard::new("req_1", PermissionAction::Edit, "Edit foo.txt");
        card.handle_key(key(KeyCode::Right));
        card.handle_key(key(KeyCode::Right));
        card.handle_key(key(KeyCode::Right));
        assert_eq!(card.cursor, 0);
    }

    #[test]
    fn cursor_wraps_left() {
        let mut card = PermissionCard::new("req_1", PermissionAction::Edit, "Edit foo.txt");
        card.handle_key(key(KeyCode::Left));
        assert_eq!(card.cursor, 2);
    }

    #[test]
    fn enter_on_once_emits_decision() {
        let mut card = PermissionCard::new("req_1", PermissionAction::Edit, "Edit foo.txt");
        let evt = card.handle_key(key(KeyCode::Enter)).unwrap();
        assert!(matches!(
            evt,
            PermissionDecisionEvent::Decided {
                choice: PermissionChoice::Once,
                ..
            }
        ));
    }

    #[test]
    fn enter_on_always_enters_substage() {
        let mut card = PermissionCard::new("req_1", PermissionAction::Edit, "Edit foo.txt");
        card.handle_key(key(KeyCode::Right));
        let evt = card.handle_key(key(KeyCode::Enter)).unwrap();
        assert!(matches!(evt, PermissionDecisionEvent::EnteredAlwaysStage));
        assert_eq!(card.stage, PermissionStage::ConfirmAlways);
    }

    #[test]
    fn confirm_always_yields_decision() {
        let mut card = PermissionCard::new("req_1", PermissionAction::Edit, "Edit foo.txt");
        card.handle_key(key(KeyCode::Right));
        card.handle_key(key(KeyCode::Enter));
        let evt = card.handle_key(key(KeyCode::Enter)).unwrap();
        assert!(matches!(
            evt,
            PermissionDecisionEvent::Decided {
                choice: PermissionChoice::Always,
                ..
            }
        ));
    }

    #[test]
    fn esc_in_always_returns_to_choose() {
        let mut card = PermissionCard::new("req_1", PermissionAction::Edit, "Edit foo.txt");
        card.handle_key(key(KeyCode::Right));
        card.handle_key(key(KeyCode::Enter));
        let _ = card.handle_key(key(KeyCode::Esc));
        assert_eq!(card.stage, PermissionStage::Choose);
    }

    #[test]
    fn reject_in_root_is_immediate_when_not_subagent() {
        let mut card = PermissionCard::new("req_1", PermissionAction::Edit, "Edit foo.txt");
        card.handle_key(key(KeyCode::Right));
        card.handle_key(key(KeyCode::Right));
        let evt = card.handle_key(key(KeyCode::Enter)).unwrap();
        assert!(matches!(
            evt,
            PermissionDecisionEvent::Decided {
                choice: PermissionChoice::Reject,
                ..
            }
        ));
    }

    #[test]
    fn reject_in_root_opens_substage_for_subagent() {
        let mut card =
            PermissionCard::new("req_1", PermissionAction::Edit, "Edit foo.txt").for_subagent();
        card.handle_key(key(KeyCode::Right));
        card.handle_key(key(KeyCode::Right));
        let evt = card.handle_key(key(KeyCode::Enter)).unwrap();
        assert!(matches!(evt, PermissionDecisionEvent::EnteredRejectStage));
        assert_eq!(card.stage, PermissionStage::Reject);
    }

    #[test]
    fn reject_stage_collects_message() {
        let mut card =
            PermissionCard::new("req_1", PermissionAction::Edit, "Edit foo.txt").for_subagent();
        // Navigate to reject and submit.
        card.handle_key(key(KeyCode::Right));
        card.handle_key(key(KeyCode::Right));
        card.handle_key(key(KeyCode::Enter));
        for c in "nope".chars() {
            card.handle_key(key(KeyCode::Char(c)));
        }
        let evt = card.handle_key(key(KeyCode::Enter)).unwrap();
        match evt {
            PermissionDecisionEvent::Decided { message, .. } => {
                assert_eq!(message.as_deref(), Some("nope"));
            }
            _ => panic!("expected Decided"),
        }
    }

    #[test]
    fn esc_at_root_rejects() {
        let mut card = PermissionCard::new("req_1", PermissionAction::Edit, "Edit foo.txt");
        let evt = card.handle_key(key(KeyCode::Esc)).unwrap();
        assert!(matches!(
            evt,
            PermissionDecisionEvent::Decided {
                choice: PermissionChoice::Reject,
                ..
            }
        ));
    }

    #[test]
    fn snapshot_includes_action_and_stage() {
        let card = PermissionCard::new("req_1", PermissionAction::Edit, "Edit foo.txt");
        let snap = card.snapshot();
        assert!(snap.contains("Edit"));
        assert!(snap.contains("Choose"));
    }

    #[test]
    fn renders_buttons_in_choose_stage() {
        let card = PermissionCard::new("req_1", PermissionAction::Edit, "Edit foo.txt");
        let mut terminal = Terminal::new(TestBackend::new(60, 6)).unwrap();
        terminal.draw(|f| f.render_widget(&card, f.area())).unwrap();
        let buf = terminal.backend().buffer().clone();
        let rendered: String = buf.content.iter().map(|c| c.symbol()).collect();
        assert!(rendered.contains("Permission required"));
        assert!(rendered.contains("Allow once"));
        assert!(rendered.contains("Reject"));
    }
}
