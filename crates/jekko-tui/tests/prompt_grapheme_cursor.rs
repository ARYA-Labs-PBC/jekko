//! The textarea-cursor / grapheme handling: typing into the prompt should
//! produce one cursor position per *grapheme cluster*, not one per byte.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use jekko_tui::prompt::Prompt;

fn press(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

#[test]
fn cursor_advances_one_step_per_ascii_char() {
    let mut prompt = Prompt::new();
    prompt.handle_key(press(KeyCode::Char('a')));
    prompt.handle_key(press(KeyCode::Char('b')));
    prompt.handle_key(press(KeyCode::Char('c')));
    let (_, col) = prompt.textarea().cursor();
    assert_eq!(col, 3);
    assert_eq!(prompt.buffer_string(), "abc");
}

#[test]
fn cursor_advances_one_step_per_cjk_char() {
    let mut prompt = Prompt::new();
    prompt.handle_key(press(KeyCode::Char('你')));
    prompt.handle_key(press(KeyCode::Char('好')));
    let (_, col) = prompt.textarea().cursor();
    // The textarea reports a "char" cursor; both CJK chars count as one step.
    assert_eq!(col, 2);
    assert_eq!(prompt.buffer_string(), "你好");
}

#[test]
fn left_arrow_moves_by_one_codepoint_after_cjk() {
    let mut prompt = Prompt::new();
    prompt.handle_key(press(KeyCode::Char('你')));
    prompt.handle_key(press(KeyCode::Char('好')));
    prompt.handle_key(press(KeyCode::Left));
    let (_, col) = prompt.textarea().cursor();
    assert_eq!(col, 1);
}

#[test]
fn home_resets_column_to_zero() {
    let mut prompt = Prompt::new();
    prompt.handle_key(press(KeyCode::Char('h')));
    prompt.handle_key(press(KeyCode::Char('i')));
    prompt.handle_key(press(KeyCode::Home));
    let (_, col) = prompt.textarea().cursor();
    assert_eq!(col, 0);
}

#[test]
fn newline_via_shift_enter_creates_second_row() {
    let mut prompt = Prompt::new();
    prompt.handle_key(press(KeyCode::Char('a')));
    let shift_enter = KeyEvent::new(KeyCode::Enter, KeyModifiers::SHIFT);
    prompt.handle_key(shift_enter);
    prompt.handle_key(press(KeyCode::Char('b')));
    assert_eq!(
        prompt.textarea().lines(),
        &["a".to_string(), "b".to_string()]
    );
}
