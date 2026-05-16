//! In-memory prompt history navigation.

use jekko_tui::prompt::PromptHistory;

#[test]
fn nav_up_cycles_backward_through_entries() {
    let mut history = PromptHistory::default();
    history.push("first");
    history.push("second");
    history.push("third");

    assert_eq!(history.nav_up(), Some("third"));
    assert_eq!(history.nav_up(), Some("second"));
    assert_eq!(history.nav_up(), Some("first"));
    // Further nav_up stays clamped at the oldest entry.
    assert_eq!(history.nav_up(), Some("first"));
}

#[test]
fn nav_down_walks_back_toward_live_buffer() {
    let mut history = PromptHistory::default();
    history.push("a");
    history.push("b");
    history.push("c");

    history.nav_up();
    history.nav_up();
    history.nav_up();
    // We're now at "a"; walk back forward.
    assert_eq!(history.nav_down(), Some("b"));
    assert_eq!(history.nav_down(), Some("c"));
    assert_eq!(history.nav_down(), None); // live buffer
    assert_eq!(history.nav_down(), None); // remains at live buffer
}

#[test]
fn clear_drops_every_entry_and_resets_cursor() {
    let mut history = PromptHistory::default();
    history.push("a");
    history.push("b");
    history.nav_up();
    history.clear();
    assert!(history.is_empty());
    assert_eq!(history.current(), None);
    assert_eq!(history.nav_up(), None);
}

#[test]
fn pushing_a_new_entry_resets_cursor() {
    let mut history = PromptHistory::default();
    history.push("old");
    history.nav_up();
    assert_eq!(history.current(), Some("old"));
    history.push("new");
    assert_eq!(history.current(), None);
}

#[test]
fn empty_entries_are_ignored() {
    let mut history = PromptHistory::default();
    history.push("");
    history.push("real");
    assert_eq!(history.len(), 1);
}

#[test]
fn capacity_evicts_oldest_first() {
    let mut history = PromptHistory::with_capacity(2);
    history.push("a");
    history.push("b");
    history.push("c");
    let entries: Vec<&str> = history.iter().collect();
    assert_eq!(entries, vec!["b", "c"]);
}
