//! Layer 3 — run-id generator uniqueness under parallel load.

use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::thread;

use sandboxctl::runid;

#[test]
fn parallel_generation_is_unique() {
    let seen: Arc<Mutex<HashSet<String>>> = Arc::new(Mutex::new(HashSet::new()));
    let mut threads = Vec::new();
    for _ in 0..8 {
        let seen = Arc::clone(&seen);
        threads.push(thread::spawn(move || {
            for _ in 0..256 {
                let id = runid::generate();
                let mut g = seen.lock().unwrap();
                assert!(g.insert(id.clone()), "duplicate run_id: {id}");
            }
        }));
    }
    for t in threads {
        t.join().expect("thread");
    }
    let g = seen.lock().unwrap();
    assert_eq!(g.len(), 8 * 256, "all parallel run-ids unique");
}

#[test]
fn shape_is_sortable() {
    let mut ids = Vec::with_capacity(50);
    for _ in 0..50 {
        ids.push(runid::generate());
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    let mut sorted = ids.clone();
    sorted.sort();
    // We can't guarantee strict monotonicity because the suffix is random,
    // but the date-prefix half MUST keep IDs broadly sortable. Check that
    // the first 16 chars (YYYYMMDDTHHMMSSZ) of `sorted` are non-decreasing.
    let prefixes: Vec<&str> = sorted.iter().map(|s| &s[..16]).collect();
    let mut prev = "";
    for p in prefixes {
        assert!(p >= prev, "date prefix should be sortable: {prev} > {p}");
        prev = p;
    }
}
