//! Integration tests for the composite `Prompt` widget — rendering, Ctrl+C,
//! Ctrl+V, history navigation gated on buffer emptiness, and snapshots.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use jekko_tui::prompt::{Prompt, PromptOutcome};
use ratatui::backend::TestBackend;
use ratatui::Terminal;

fn press(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn press_with(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
    KeyEvent::new(code, modifiers)
}

#[test]
fn renders_without_panicking() {
    let mut prompt = Prompt::new();
    prompt.set_model_label("claude-opus-4-7");
    prompt.handle_key(press(KeyCode::Char('h')));
    prompt.handle_key(press(KeyCode::Char('i')));
    let backend = TestBackend::new(60, 6);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|f| f.render_widget(&prompt, f.area()))
        .unwrap();
    let text = terminal.backend().to_string();
    assert!(text.contains("hi"));
    assert!(text.contains("claude-opus"));
}

#[test]
fn ctrl_c_returns_clear_requested_and_empties_buffer() {
    let mut prompt = Prompt::new();
    prompt.handle_key(press(KeyCode::Char('x')));
    assert!(!prompt.buffer_string().is_empty());
    let outcome = prompt.handle_key(press_with(KeyCode::Char('c'), KeyModifiers::CONTROL));
    assert!(matches!(outcome, PromptOutcome::ClearRequested));
    assert!(prompt.buffer_string().is_empty());
}

#[test]
fn ctrl_v_returns_paste_requested() {
    let mut prompt = Prompt::new();
    let outcome = prompt.handle_key(press_with(KeyCode::Char('v'), KeyModifiers::CONTROL));
    assert!(matches!(outcome, PromptOutcome::PasteRequested));
}

#[test]
fn enter_returns_submit_outcome() {
    let mut prompt = Prompt::new();
    prompt.handle_key(press(KeyCode::Char('x')));
    let outcome = prompt.handle_key(press(KeyCode::Enter));
    assert!(matches!(outcome, PromptOutcome::Submit));
    let payload = prompt.submit();
    assert_eq!(payload.as_deref(), Some("x"));
}

#[test]
fn ctrl_a_jumps_to_line_home_and_ctrl_e_to_line_end() {
    let mut prompt = Prompt::new();
    for ch in "hello".chars() {
        prompt.handle_key(press(KeyCode::Char(ch)));
    }
    prompt.handle_key(press_with(KeyCode::Char('a'), KeyModifiers::CONTROL));
    assert_eq!(prompt.textarea().cursor().1, 0);
    prompt.handle_key(press_with(KeyCode::Char('e'), KeyModifiers::CONTROL));
    assert_eq!(prompt.textarea().cursor().1, 5);
}

#[test]
fn up_navigates_history_when_buffer_empty() {
    let mut prompt = Prompt::new();
    prompt.handle_key(press(KeyCode::Char('a')));
    prompt.handle_key(press(KeyCode::Enter));
    prompt.submit();
    prompt.handle_key(press(KeyCode::Char('b')));
    prompt.handle_key(press(KeyCode::Enter));
    prompt.submit();

    // Buffer is now empty — Up should recall "b".
    prompt.handle_key(press(KeyCode::Up));
    assert_eq!(prompt.buffer_string(), "b");
    prompt.handle_key(press(KeyCode::Up));
    assert_eq!(prompt.buffer_string(), "a");
    prompt.handle_key(press(KeyCode::Down));
    assert_eq!(prompt.buffer_string(), "b");
    prompt.handle_key(press(KeyCode::Down));
    assert_eq!(prompt.buffer_string(), "");
}

#[test]
fn snapshot_reflects_state() {
    let mut prompt = Prompt::new();
    prompt.handle_key(press(KeyCode::Char('h')));
    prompt.handle_key(press(KeyCode::Char('i')));
    let snap = prompt.snapshot();
    assert_eq!(snap.visible, "hi");
    assert_eq!(snap.expanded, "hi");
    assert!(!snap.slash_open);
    assert!(!snap.mention_open);
}

#[test]
fn submit_empty_buffer_returns_none() {
    let mut prompt = Prompt::new();
    assert!(prompt.submit().is_none());
}

#[test]
fn save_and_restore_stash_roundtrip() {
    let mut prompt = Prompt::new();
    prompt.handle_key(press(KeyCode::Char('h')));
    prompt.handle_key(press(KeyCode::Char('i')));
    prompt.save_stash("home");
    prompt.clear();
    assert!(prompt.buffer_string().is_empty());
    assert!(prompt.restore_stash("home"));
    assert_eq!(prompt.buffer_string(), "hi");
}
