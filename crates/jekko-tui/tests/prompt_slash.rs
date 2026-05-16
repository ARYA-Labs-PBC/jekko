//! Slash command popup: typing `/` opens the popup, arrows navigate it, and
//! Enter accepts the selection (instead of submitting).

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use jekko_tui::prompt::slash::{buffer_triggers_slash, builtin_commands};
use jekko_tui::prompt::{Prompt, PromptOutcome};

fn press(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

#[test]
fn typing_slash_opens_popup() {
    let mut prompt = Prompt::new();
    assert!(!prompt.is_slash_open());
    prompt.handle_key(press(KeyCode::Char('/')));
    assert!(prompt.is_slash_open());
}

#[test]
fn slash_popup_closes_when_buffer_loses_prefix() {
    let mut prompt = Prompt::new();
    prompt.handle_key(press(KeyCode::Char('/')));
    assert!(prompt.is_slash_open());
    // Backspace away the `/` and the popup should close.
    prompt.handle_key(press(KeyCode::Backspace));
    assert!(!prompt.is_slash_open());
}

#[test]
fn slash_filters_by_query() {
    let mut prompt = Prompt::new();
    prompt.handle_key(press(KeyCode::Char('/')));
    prompt.handle_key(press(KeyCode::Char('h')));
    let filtered = prompt.slash().filtered();
    assert!(filtered.iter().any(|c| c.id == "help"));
    assert!(filtered.iter().all(|c| c.label.starts_with('h')));
}

#[test]
fn slash_enter_returns_selection_not_submit() {
    let mut prompt = Prompt::new();
    prompt.handle_key(press(KeyCode::Char('/')));
    let outcome = prompt.handle_key(press(KeyCode::Enter));
    match outcome {
        PromptOutcome::SlashSelected(cmd) => {
            assert!(builtin_commands().iter().any(|c| c.id == cmd.id));
        }
        other => panic!("expected SlashSelected, got {other:?}"),
    }
    assert!(!prompt.is_slash_open());
}

#[test]
fn slash_arrow_keys_move_cursor() {
    let mut prompt = Prompt::new();
    prompt.handle_key(press(KeyCode::Char('/')));
    let first = prompt.slash().selected().unwrap().id;
    prompt.handle_key(press(KeyCode::Down));
    let second = prompt.slash().selected().unwrap().id;
    assert_ne!(first, second);
    prompt.handle_key(press(KeyCode::Up));
    let third = prompt.slash().selected().unwrap().id;
    assert_eq!(first, third);
}

#[test]
fn slash_escape_cancels_popup() {
    let mut prompt = Prompt::new();
    prompt.handle_key(press(KeyCode::Char('/')));
    let outcome = prompt.handle_key(press(KeyCode::Esc));
    assert!(matches!(outcome, PromptOutcome::PopupCancelled));
    assert!(!prompt.is_slash_open());
}

#[test]
fn buffer_trigger_helper_is_sensitive_to_whitespace() {
    assert!(buffer_triggers_slash("/help"));
    assert!(!buffer_triggers_slash("hi /world"));
    assert!(!buffer_triggers_slash("/help me"));
}
