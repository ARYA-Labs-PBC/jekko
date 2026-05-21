use super::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

struct ScopedDir {
    path: PathBuf,
}

impl ScopedDir {
    fn new(label: &'static str) -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let pid = std::process::id();
        let counter = COUNTER.fetch_add(1, Ordering::SeqCst);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let path =
            std::env::temp_dir().join(format!("jekko-xtask-test-{label}-{pid}-{counter}-{nanos}"));
        fs::create_dir_all(&path).unwrap();
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for ScopedDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[test]
fn byte_diff_matches_identical_inputs() {
    let (diff, total) = byte_diff(b"hello world", b"hello world");
    assert_eq!(diff, 0);
    assert_eq!(total, 11);
}

#[test]
fn byte_diff_counts_inline_substitutions() {
    let (diff, total) = byte_diff(b"abcd", b"aXcY");
    assert_eq!(diff, 2);
    assert_eq!(total, 4);
}

#[test]
fn byte_diff_counts_length_deltas() {
    let (diff, total) = byte_diff(b"abcd", b"abcdEXTRA");
    assert_eq!(diff, 5);
    assert_eq!(total, 9);
}

#[test]
fn byte_diff_handles_empty_inputs() {
    assert_eq!(byte_diff(b"", b""), (0, 0));
    assert_eq!(byte_diff(b"foo", b""), (3, 3));
    assert_eq!(byte_diff(b"", b"bar"), (3, 3));
}

#[test]
fn mismatch_percent_uses_max_length_denominator() {
    assert_eq!(mismatch_percent(0, 0), 0.0);
    assert!((mismatch_percent(2, 4) - 50.0).abs() < 1e-9);
    assert!((mismatch_percent(5, 9) - (5.0 / 9.0 * 100.0)).abs() < 1e-9);
}

fn write_capture(root: &Path, screen: &str, file_stem: &str, body: &str) -> PathBuf {
    let dir = root.join(screen);
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("{file_stem}.txt"));
    fs::write(&path, body).unwrap();
    path
}

#[test]
fn collect_capture_keys_walks_screen_subdirs() {
    let dir = ScopedDir::new("walker");
    write_capture(dir.path(), "home", "80x24", "hello");
    write_capture(dir.path(), "home", "200x60", "wide");
    write_capture(
        dir.path(),
        "home",
        "80x24-boot-timeout",
        "timeout diagnostic",
    );
    write_capture(dir.path(), "command-dialog", "80x24", "cmd");
    fs::write(dir.path().join("home/80x24.png"), b"\x89PNG").unwrap();

    let map = collect_capture_keys(dir.path()).unwrap();
    let keys: Vec<&String> = map.keys().collect();
    assert_eq!(
        keys,
        vec!["command-dialog/80x24", "home/200x60", "home/80x24",]
    );
}

#[test]
fn collect_capture_keys_returns_empty_when_root_missing() {
    let dir = ScopedDir::new("missing");
    let missing = dir.path().join("nope");
    let map = collect_capture_keys(&missing).unwrap();
    assert!(map.is_empty());
}

#[test]
fn baseline_diff_rows_classifies_ok_diff_missing() {
    let parent = ScopedDir::new("classify");
    let baseline = parent.path().join("baseline");
    let rust = parent.path().join("rust");

    write_capture(&baseline, "home", "80x24", "frame");
    write_capture(&rust, "home", "80x24", "frame");

    write_capture(&baseline, "home", "200x60", "alpha-beta-gamma");
    write_capture(&rust, "home", "200x60", "alpha-XXXX-gamma");

    write_capture(&baseline, "shell", "80x24", "only-baseline");
    write_capture(&rust, "splash", "80x24", "only-rust");

    let rows = baseline_diff_rows(&baseline, &rust).unwrap();
    let by_key: std::collections::HashMap<String, &BaselineDiffRow> =
        rows.iter().map(|r| (r.key.clone(), r)).collect();

    let ok = by_key["home/80x24"];
    assert_eq!(ok.status, BaselineDiffStatus::Ok);
    assert_eq!(ok.bytes_diff, 0);
    assert!(ok.baseline_present && ok.rust_present);

    let diff = by_key["home/200x60"];
    assert_eq!(diff.status, BaselineDiffStatus::Diff);
    assert_eq!(diff.bytes_diff, 4);
    assert_eq!(diff.bytes_total, 16);
    assert!((diff.mismatch_pct - 25.0).abs() < 1e-9);

    let missing_rust = by_key["shell/80x24"];
    assert_eq!(missing_rust.status, BaselineDiffStatus::Missing);
    assert!(missing_rust.baseline_present);
    assert!(!missing_rust.rust_present);

    let missing_baseline = by_key["splash/80x24"];
    assert_eq!(missing_baseline.status, BaselineDiffStatus::Missing);
    assert!(!missing_baseline.baseline_present);
    assert!(missing_baseline.rust_present);
}

#[test]
fn baseline_diff_passes_threshold_when_under() {
    let parent = ScopedDir::new("under");
    let baseline = parent.path().join("baseline");
    let rust = parent.path().join("rust");
    write_capture(&baseline, "home", "80x24", "aaaaaaaaaa");
    write_capture(&rust, "home", "80x24", "aaaaaaaaab");
    baseline_diff(&baseline, &rust, BaselineDiffFormat::Text, Some(25.0)).unwrap();
}

#[test]
fn baseline_diff_fails_threshold_when_over() {
    let parent = ScopedDir::new("over");
    let baseline = parent.path().join("baseline");
    let rust = parent.path().join("rust");
    write_capture(&baseline, "home", "80x24", "aaaaaaaaaa");
    write_capture(&rust, "home", "80x24", "bbbbbbbbbb");
    let err = baseline_diff(&baseline, &rust, BaselineDiffFormat::Text, Some(5.0)).unwrap_err();
    assert!(format!("{err:#}").contains("exceed mismatch threshold"));
}

#[test]
fn baseline_diff_treats_missing_pair_as_full_mismatch() {
    let parent = ScopedDir::new("orphan");
    let baseline = parent.path().join("baseline");
    let rust = parent.path().join("rust");
    write_capture(&baseline, "home", "80x24", "stuff");
    let rows = baseline_diff_rows(&baseline, &rust).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].status, BaselineDiffStatus::Missing);
    assert!((rows[0].mismatch_pct - 100.0).abs() < 1e-9);
}
