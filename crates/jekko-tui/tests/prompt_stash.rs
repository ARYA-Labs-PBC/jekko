//! Per-route draft stash round-trips.

use jekko_tui::prompt::{PromptStash, RouteKey};

#[test]
fn save_and_restore_round_trips_text() {
    let mut stash = PromptStash::new();
    stash.save(RouteKey::new("home"), "draft on home");
    assert_eq!(stash.peek("home"), Some("draft on home"));
    assert_eq!(stash.restore("home"), Some("draft on home".to_string()));
    // restore is consuming
    assert_eq!(stash.peek("home"), None);
}

#[test]
fn empty_save_removes_existing_entry() {
    let mut stash = PromptStash::new();
    stash.save("home", "draft");
    stash.save("home", "");
    assert!(stash.peek("home").is_none());
}

#[test]
fn multiple_routes_are_independent() {
    let mut stash = PromptStash::new();
    stash.save("home", "h");
    stash.save("shell", "s");
    stash.save(RouteKey::new("session:abc"), "ss");
    assert_eq!(stash.len(), 3);
    assert_eq!(stash.peek("home"), Some("h"));
    assert_eq!(stash.peek("shell"), Some("s"));
    assert_eq!(stash.peek("session:abc"), Some("ss"));
}

#[test]
fn clear_drops_every_entry() {
    let mut stash = PromptStash::new();
    stash.save("a", "x");
    stash.save("b", "y");
    stash.clear();
    assert!(stash.is_empty());
}

#[test]
fn route_key_from_str_and_string_match() {
    let key_a: RouteKey = "abc".into();
    let key_b: RouteKey = "abc".to_string().into();
    assert_eq!(key_a, key_b);
    assert_eq!(key_a.as_str(), "abc");
}
