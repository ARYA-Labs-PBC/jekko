use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::state::{PagerMode, PagerState};

/// What the caller should do in response to a key. The pager itself never
/// touches the clipboard or exits the host TUI; it just signals intent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PagerAction {
    /// Key consumed, nothing further to do.
    None,
    /// User pressed `q` / `Esc` in browse mode.
    Exit,
    /// User pressed `y` / `Y`; caller should pipe the payload through OSC52.
    Yank(String),
}

/// Drive the pager with a key event. Behaviour depends on the active
/// [`PagerMode`]; `viewport_height` is the row count available for the body.
pub fn handle_key(state: &mut PagerState, key: KeyEvent, viewport_height: usize) -> PagerAction {
    match state.mode {
        PagerMode::Search => handle_key_search(state, key),
        PagerMode::Browse | PagerMode::Highlight => {
            handle_key_browse_or_highlight(state, key, viewport_height)
        }
    }
}

fn handle_key_search(state: &mut PagerState, key: KeyEvent) -> PagerAction {
    match key.code {
        KeyCode::Enter => {
            state.commit_search();
            PagerAction::None
        }
        KeyCode::Esc => {
            state.cancel_search();
            PagerAction::None
        }
        KeyCode::Backspace => {
            state.pop_search_char();
            PagerAction::None
        }
        KeyCode::Char(c) => {
            let blocked = key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SUPER);
            if !blocked {
                state.push_search_char(c);
            }
            PagerAction::None
        }
        _ => PagerAction::None,
    }
}

fn handle_key_browse_or_highlight(
    state: &mut PagerState,
    key: KeyEvent,
    viewport_height: usize,
) -> PagerAction {
    match key.code {
        KeyCode::Up => {
            state.scroll_up(1);
            PagerAction::None
        }
        KeyCode::Down => {
            state.scroll_down(1);
            PagerAction::None
        }
        KeyCode::PageUp => {
            state.page_up(viewport_height);
            PagerAction::None
        }
        KeyCode::PageDown => {
            state.page_down(viewport_height);
            PagerAction::None
        }
        KeyCode::Home => {
            state.home();
            PagerAction::None
        }
        KeyCode::End => {
            state.end(viewport_height);
            PagerAction::None
        }
        KeyCode::Char('/') => {
            state.start_search();
            PagerAction::None
        }
        KeyCode::Char('n') if !key.modifiers.contains(KeyModifiers::SHIFT) => {
            state.next_match(viewport_height);
            PagerAction::None
        }
        KeyCode::Char('N') => {
            state.prev_match(viewport_height);
            PagerAction::None
        }
        KeyCode::Char('y') => yank_current_line(state),
        KeyCode::Char('Y') => yank_visible_lines(state, viewport_height),
        KeyCode::Char('q') | KeyCode::Esc => PagerAction::Exit,
        _ => PagerAction::None,
    }
}

fn yank_current_line(state: &PagerState) -> PagerAction {
    let line_idx = state
        .selected_match()
        .map(|m| m.line)
        .unwrap_or(state.scroll);
    match state.lines.get(line_idx) {
        Some(line) => PagerAction::Yank(line.clone()),
        None => PagerAction::None,
    }
}

fn yank_visible_lines(state: &PagerState, viewport_height: usize) -> PagerAction {
    let h = viewport_height.max(1);
    let end = state.scroll.saturating_add(h).min(state.lines.len());
    let slice = &state.lines[state.scroll..end];
    PagerAction::Yank(slice.join("\n"))
}
