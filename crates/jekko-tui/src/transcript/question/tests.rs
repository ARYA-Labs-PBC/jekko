use super::*;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::Terminal;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn build_card() -> QuestionCard {
    QuestionCard::new(
        "q_1",
        "Pick one",
        vec![
            QuestionChoice::new("alpha"),
            QuestionChoice::new("beta"),
            QuestionChoice::new("gamma"),
        ],
    )
}

#[test]
fn cursor_moves_down_with_arrow() {
    let mut card = build_card();
    card.handle_key(key(KeyCode::Down));
    assert_eq!(card.cursor, 1);
}

#[test]
fn cursor_wraps() {
    let mut card = build_card();
    card.handle_key(key(KeyCode::Up));
    assert_eq!(card.cursor, 3); // wraps over custom slot
}

#[test]
fn single_enter_submits() {
    let mut card = build_card();
    let evt = card.handle_key(key(KeyCode::Enter)).unwrap();
    assert!(
        matches!(evt, QuestionEvent::Submitted { ref answers } if answers == &vec!["alpha".to_string()])
    );
}

#[test]
fn multi_enter_toggles() {
    let mut card = build_card().with_mode(QuestionMode::Multi);
    assert!(card.handle_key(key(KeyCode::Enter)).is_none());
    assert_eq!(card.picked.len(), 1);
    assert!(card.handle_key(key(KeyCode::Enter)).is_none());
    assert_eq!(card.picked.len(), 0);
}

#[test]
fn digit_jumps_and_submits_single() {
    let mut card = build_card();
    let evt = card.handle_key(key(KeyCode::Char('2'))).unwrap();
    assert!(
        matches!(evt, QuestionEvent::Submitted { ref answers } if answers == &vec!["beta".to_string()])
    );
}

#[test]
fn esc_rejects() {
    let mut card = build_card();
    let evt = card.handle_key(key(KeyCode::Esc)).unwrap();
    assert!(matches!(evt, QuestionEvent::Rejected));
}

#[test]
fn custom_slot_enters_editing() {
    let mut card = build_card();
    card.cursor = 3;
    let evt = card.handle_key(key(KeyCode::Enter)).unwrap();
    assert!(matches!(
        evt,
        QuestionEvent::EditingChanged { editing: true }
    ));
    assert!(card.editing_custom);
}

#[test]
fn editing_accumulates_text() {
    let mut card = build_card();
    card.cursor = 3;
    card.handle_key(key(KeyCode::Enter));
    for c in "hi".chars() {
        card.handle_key(key(KeyCode::Char(c)));
    }
    assert_eq!(card.custom_text, "hi");
}

#[test]
fn editing_enter_submits_single() {
    let mut card = build_card();
    card.cursor = 3;
    card.handle_key(key(KeyCode::Enter));
    for c in "custom".chars() {
        card.handle_key(key(KeyCode::Char(c)));
    }
    let evt = card.handle_key(key(KeyCode::Enter)).unwrap();
    assert!(
        matches!(evt, QuestionEvent::Submitted { ref answers } if answers == &vec!["custom".to_string()])
    );
}

#[test]
fn editing_esc_aborts() {
    let mut card = build_card();
    card.cursor = 3;
    card.handle_key(key(KeyCode::Enter));
    let evt = card.handle_key(key(KeyCode::Esc)).unwrap();
    assert!(matches!(
        evt,
        QuestionEvent::EditingChanged { editing: false }
    ));
    assert!(!card.editing_custom);
}

#[test]
fn renders_options() {
    let card = build_card();
    let mut terminal = Terminal::new(TestBackend::new(80, 12)).unwrap();
    terminal.draw(|f| f.render_widget(&card, f.area())).unwrap();
    let buf = terminal.backend().buffer().clone();
    let rendered: String = buf.content.iter().map(|c| c.symbol()).collect();
    assert!(rendered.contains("Pick one"));
    assert!(rendered.contains("alpha"));
    assert!(rendered.contains("beta"));
}

#[test]
fn snapshot_contains_prompt() {
    let card = build_card();
    let snap = card.snapshot();
    assert!(snap.contains("Pick one"));
    assert!(snap.contains("Single"));
}

#[test]
fn submit_returns_picked_in_order() {
    let mut card = build_card().with_mode(QuestionMode::Multi);
    card.cursor = 0;
    card.handle_key(key(KeyCode::Enter));
    card.cursor = 2;
    card.handle_key(key(KeyCode::Enter));
    let evt = card.submit();
    match evt {
        QuestionEvent::Submitted { answers } => {
            assert_eq!(answers, vec!["alpha".to_string(), "gamma".to_string()]);
        }
        _ => panic!("expected Submitted"),
    }
}
