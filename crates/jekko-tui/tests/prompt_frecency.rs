//! Frecency table behavior: bumping updates count and timestamp; top_n
//! returns entries sorted by frecency (count × recency weight).

use std::time::{Duration, Instant};

use jekko_tui::prompt::Frecency;

#[test]
fn bump_increments_count() {
    let mut f = Frecency::new();
    f.bump("/help");
    assert_eq!(f.count_of("/help"), 1);
    f.bump("/help");
    assert_eq!(f.count_of("/help"), 2);
}

#[test]
fn bump_records_timestamp() {
    let mut f = Frecency::new();
    let now = Instant::now();
    f.set_now(now);
    f.bump("/model");
    let ts = f.last_used_of("/model").unwrap();
    assert_eq!(ts, now);
}

#[test]
fn top_n_orders_by_recency_when_counts_tie() {
    let mut f = Frecency::new();
    let now = Instant::now();
    f.set_now(now);
    f.bump("/old");
    f.set_now(now + Duration::from_secs(86_400 * 30)); // 30 days later
    f.bump("/recent");

    let top = f.top_n(2);
    assert_eq!(top[0].id, "/recent");
    assert_eq!(top[1].id, "/old");
}

#[test]
fn top_n_respects_count_when_recency_ties() {
    let mut f = Frecency::new();
    let now = Instant::now();
    f.set_now(now);
    f.bump("/many");
    f.bump("/many");
    f.bump("/many");
    f.bump("/few");

    let top = f.top_n(2);
    assert_eq!(top[0].id, "/many");
    assert_eq!(top[1].id, "/few");
}

#[test]
fn top_n_truncates() {
    let mut f = Frecency::new();
    for ch in 'a'..='e' {
        f.bump(format!("/{ch}"));
    }
    assert_eq!(f.top_n(3).len(), 3);
}

#[test]
fn unknown_entries_have_zero_count() {
    let f = Frecency::new();
    assert_eq!(f.count_of("/missing"), 0);
    assert!(f.last_used_of("/missing").is_none());
}

#[test]
fn empty_top_n_is_empty() {
    let f = Frecency::new();
    assert!(f.top_n(5).is_empty());
}
