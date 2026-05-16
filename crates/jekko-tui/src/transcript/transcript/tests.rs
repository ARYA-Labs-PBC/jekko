use super::*;
use crate::transcript::cards::{
    AssistantCard, AssistantPart, AssistantPartKind, ReasoningCard, SystemCard, SystemKind,
    ToolCard, ToolStatus, UserCard,
};
use std::time::{Duration, Instant};

fn sample_user(text: &str) -> UserCard {
    UserCard::new(text.to_string()).with_timestamp_label("12:34")
}

#[test]
fn new_transcript_is_empty_and_sticky() {
    let t = Transcript::new();
    assert!(t.is_empty());
    assert_eq!(t.len(), 0);
    assert!(t.is_sticky_bottom());
    assert_eq!(t.scroll_offset(), 0);
}

#[test]
fn push_user_grows_buffer() {
    let mut t = Transcript::new();
    t.push_user(sample_user("hello"));
    t.push_user(sample_user("world"));
    assert_eq!(t.len(), 2);
    assert!(matches!(t.entries()[0], TranscriptEntry::User(_)));
}

#[test]
fn scroll_up_disengages_sticky_bottom() {
    let mut t = Transcript::new();
    t.set_viewport_rows(4);
    for _ in 0..10 {
        t.push_user(sample_user("line"));
    }
    // Sticky-bottom holds offset at max.
    let max = t.max_offset();
    assert!(max > 0, "expected scrollable content, got {max}");
    t.scroll_up(2);
    assert!(!t.is_sticky_bottom());
}

#[test]
fn scroll_down_to_bottom_reengages_sticky() {
    let mut t = Transcript::new();
    t.set_viewport_rows(4);
    for _ in 0..6 {
        t.push_user(sample_user("line"));
    }
    t.scroll_up(99);
    assert!(!t.is_sticky_bottom());
    t.scroll_down(99);
    assert!(t.is_sticky_bottom());
    assert_eq!(t.scroll_offset(), t.max_offset());
}

#[test]
fn page_up_uses_viewport() {
    let mut t = Transcript::new();
    t.set_viewport_rows(5);
    for _ in 0..10 {
        t.push_user(sample_user("x"));
    }
    let before = t.scroll_offset();
    t.page_up();
    assert!(t.scroll_offset() < before);
}

#[test]
fn top_and_bottom_jump() {
    let mut t = Transcript::new();
    t.set_viewport_rows(2);
    for _ in 0..6 {
        t.push_user(sample_user("x"));
    }
    t.top();
    assert_eq!(t.scroll_offset(), 0);
    assert!(!t.is_sticky_bottom());
    t.bottom();
    assert_eq!(t.scroll_offset(), t.max_offset());
    assert!(t.is_sticky_bottom());
}

#[test]
fn acceleration_grows_within_window() {
    let mut accel = ScrollAcceleration::default();
    let t0 = Instant::now();
    let v1 = accel.tick_at(t0);
    let v2 = accel.tick_at(t0 + Duration::from_millis(20));
    let v3 = accel.tick_at(t0 + Duration::from_millis(40));
    assert_eq!(v1, 1);
    assert_eq!(v2, 2);
    assert_eq!(v3, 3);
}

#[test]
fn acceleration_resets_after_window() {
    let mut accel = ScrollAcceleration::default();
    let t0 = Instant::now();
    accel.tick_at(t0);
    accel.tick_at(t0 + Duration::from_millis(20));
    let resumed = accel.tick_at(t0 + Duration::from_secs(2));
    assert_eq!(resumed, 1);
}

#[test]
fn accelerated_scroll_changes_offset() {
    let mut t = Transcript::new();
    t.set_viewport_rows(2);
    for _ in 0..20 {
        t.push_user(sample_user("x"));
    }
    let max = t.max_offset();
    t.scroll_up(max);
    assert_eq!(t.scroll_offset(), 0);
    let v = t.accelerated_scroll(ScrollIntent::Down);
    assert_eq!(v, 1);
    assert!(t.scroll_offset() >= 1);
}

#[test]
fn pop_returns_last_entry() {
    let mut t = Transcript::new();
    t.push_user(sample_user("first"));
    t.push_user(sample_user("second"));
    let last = t.pop();
    assert!(matches!(last, Some(TranscriptEntry::User(_))));
    assert_eq!(t.len(), 1);
}

#[test]
fn replace_last_swaps_card() {
    let mut t = Transcript::new();
    t.push_user(sample_user("first"));
    let assistant = AssistantCard::new(vec![AssistantPart::new(
        AssistantPartKind::Text,
        "hi".into(),
    )]);
    let prev = t.replace_last(TranscriptEntry::Assistant(assistant));
    assert!(matches!(prev, Some(TranscriptEntry::User(_))));
    assert!(matches!(t.entries()[0], TranscriptEntry::Assistant(_)));
}

#[test]
fn system_card_appends() {
    let mut t = Transcript::new();
    t.push_system(SystemCard::new("daemon online", SystemKind::Info));
    assert_eq!(t.entries()[0].kind_label(), "system");
}

#[test]
fn tool_card_estimated_rows_includes_diff() {
    let card = ToolCard::new("tool_1", "shell").with_status(ToolStatus::Running);
    let rows = card.estimated_rows();
    assert!(rows >= 2);
}

#[test]
fn reasoning_card_can_collapse() {
    let mut card = ReasoningCard::new("muttering");
    assert!(!card.collapsed);
    card.toggle_collapsed();
    assert!(card.collapsed);
}

#[test]
fn snapshot_includes_kinds() {
    let mut t = Transcript::new();
    t.push_user(sample_user("hi"));
    t.push_system(SystemCard::new("info", SystemKind::Info));
    let snap = t.snapshot();
    assert!(snap.contains("user"));
    assert!(snap.contains("system"));
}

#[test]
fn clear_resets_state() {
    let mut t = Transcript::new();
    t.set_viewport_rows(2);
    t.push_user(sample_user("x"));
    t.scroll_up(99);
    t.clear();
    assert!(t.is_empty());
    assert!(t.is_sticky_bottom());
    assert_eq!(t.scroll_offset(), 0);
}
