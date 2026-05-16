//! `@`-mention popup: typing `@fo` filters caller-supplied candidates.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use jekko_tui::prompt::{MentionCandidate, Prompt, PromptOutcome};

fn press(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn sample_candidates() -> Vec<MentionCandidate> {
    vec![
        MentionCandidate::new("src/foo.rs"),
        MentionCandidate::new("src/bar.rs"),
        MentionCandidate::new("docs/folio.md"),
        MentionCandidate::new("notes/readme.md"),
    ]
}

#[test]
fn typing_at_opens_popup() {
    let mut prompt = Prompt::new();
    prompt.set_mention_candidates(sample_candidates());
    assert!(!prompt.is_mention_open());
    prompt.handle_key(press(KeyCode::Char('@')));
    assert!(prompt.is_mention_open());
}

#[test]
fn typing_fo_after_at_filters_candidates() {
    let mut prompt = Prompt::new();
    prompt.set_mention_candidates(sample_candidates());
    prompt.handle_key(press(KeyCode::Char('@')));
    prompt.handle_key(press(KeyCode::Char('f')));
    prompt.handle_key(press(KeyCode::Char('o')));
    let filtered = prompt.mention().filtered();
    let labels: Vec<String> = filtered.iter().map(MentionCandidate::display).collect();
    assert!(
        labels
            .iter()
            .any(|p| p.contains("foo.rs") || p.contains("folio.md")),
        "expected foo.rs or folio.md, got {labels:?}",
    );
    // bar.rs and readme.md should not be matched by "fo".
    assert!(labels.iter().all(|p| !p.contains("bar.rs")));
}

#[test]
fn mention_enter_returns_selection() {
    let mut prompt = Prompt::new();
    prompt.set_mention_candidates(sample_candidates());
    prompt.handle_key(press(KeyCode::Char('@')));
    prompt.handle_key(press(KeyCode::Char('f')));
    prompt.handle_key(press(KeyCode::Char('o')));
    let outcome = prompt.handle_key(press(KeyCode::Enter));
    match outcome {
        PromptOutcome::MentionSelected(c) => {
            assert!(c.display().contains("foo") || c.display().contains("folio"));
        }
        other => panic!("expected MentionSelected, got {other:?}"),
    }
    assert!(!prompt.is_mention_open());
}

#[test]
fn whitespace_after_at_closes_popup() {
    let mut prompt = Prompt::new();
    prompt.set_mention_candidates(sample_candidates());
    prompt.handle_key(press(KeyCode::Char('@')));
    assert!(prompt.is_mention_open());
    prompt.handle_key(press(KeyCode::Char(' ')));
    assert!(!prompt.is_mention_open());
}

#[test]
fn empty_query_returns_all_candidates() {
    let mut prompt = Prompt::new();
    prompt.set_mention_candidates(sample_candidates());
    prompt.handle_key(press(KeyCode::Char('@')));
    let filtered = prompt.mention().filtered();
    assert_eq!(filtered.len(), 4);
}
