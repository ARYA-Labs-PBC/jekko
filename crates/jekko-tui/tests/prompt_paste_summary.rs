//! Long bracketed pastes collapse into an inline summary chip; the actual
//! content is stashed in a side buffer and re-expanded on submit.

use jekko_tui::prompt::paste::PasteBuffer;
use jekko_tui::prompt::{Prompt, PASTE_BYTE_THRESHOLD, PASTE_LINE_THRESHOLD};

fn long_multiline_paste() -> String {
    let mut s = String::new();
    for i in 0..(PASTE_LINE_THRESHOLD + 2) {
        s.push_str(&format!("line {i}\n"));
    }
    s
}

#[test]
fn paste_threshold_exposes_constants() {
    // Spec lockdown: thresholds must match the documented contract.
    const _: () = assert!(PASTE_LINE_THRESHOLD >= 8);
    const _: () = assert!(PASTE_BYTE_THRESHOLD >= 280);
}

#[test]
fn paste_buffer_should_collapse_long_pastes() {
    let long = long_multiline_paste();
    assert!(PasteBuffer::should_collapse(&long));
}

#[test]
fn paste_buffer_should_skip_short_pastes() {
    assert!(!PasteBuffer::should_collapse("two\nlines"));
}

#[test]
fn paste_buffer_should_collapse_dense_single_line() {
    let dense = "a".repeat(PASTE_BYTE_THRESHOLD + 1);
    assert!(PasteBuffer::should_collapse(&dense));
}

#[test]
fn prompt_collapses_long_paste_to_chip() {
    let mut prompt = Prompt::new();
    let payload = long_multiline_paste();
    prompt.handle_paste(payload.clone());
    let visible = prompt.buffer_string();
    assert!(visible.contains("[paste"));
    assert!(visible.contains("lines"));
    assert_eq!(prompt.paste_buffer().records().len(), 1);
    let expanded = prompt.expanded_buffer();
    assert_eq!(expanded, payload);
}

#[test]
fn prompt_keeps_short_paste_inline() {
    let mut prompt = Prompt::new();
    prompt.handle_paste("short paste".to_string());
    assert_eq!(prompt.buffer_string(), "short paste");
    assert!(prompt.paste_buffer().records().is_empty());
}

#[test]
fn submit_returns_expanded_payload() {
    let mut prompt = Prompt::new();
    let payload = long_multiline_paste();
    prompt.handle_paste(payload.clone());
    let submitted = prompt.submit().expect("submit returns text");
    assert!(submitted.contains("line 0"));
    assert!(submitted.contains(&format!("line {}", PASTE_LINE_THRESHOLD + 1)));
}

#[test]
fn paste_summary_format_includes_line_and_size() {
    let mut buffer = PasteBuffer::new();
    let record = buffer.stash("ten\nlines\nhere\nfor\nthe\nformat\ncheck\nin\nthe\ntest");
    let summary = record.summary();
    assert!(summary.starts_with("[paste #1"));
    assert!(summary.contains("lines"));
}
