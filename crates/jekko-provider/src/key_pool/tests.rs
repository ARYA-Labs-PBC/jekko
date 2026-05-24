use std::fs;
use std::path::Path;
use std::time::Duration;

use tempfile::TempDir;

use super::*;

fn write_env(root: &Path, user: &str, contents: &str) {
    let dir = root.join(user);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join(LLM_ENV_FILENAME), contents).unwrap();
}

#[test]
fn locked_returns_only_default_dir() {
    let tmp = TempDir::new().unwrap();
    write_env(tmp.path(), "user", "OPENAI_API_KEY=k0\n");
    write_env(tmp.path(), "user_1", "OPENAI_API_KEY=k1\n");
    write_env(tmp.path(), "user_2", "OPENAI_API_KEY=k2\n");
    let dirs = discover_in(tmp.path(), false);
    assert_eq!(dirs.len(), 1);
    assert_eq!(dirs[0].user_id, "user");
}

#[test]
fn unlocked_includes_extras_sorted_after_default() {
    let tmp = TempDir::new().unwrap();
    write_env(tmp.path(), "user_2", "OPENAI_API_KEY=k2\n");
    write_env(tmp.path(), "user_1", "OPENAI_API_KEY=k1\n");
    write_env(tmp.path(), "alice", "OPENAI_API_KEY=ka\n");
    let dirs = discover_in(tmp.path(), true);
    let ids: Vec<_> = dirs.iter().map(|d| d.user_id.clone()).collect();
    assert_eq!(ids, vec!["user", "alice", "user_1", "user_2"]);
}

#[test]
fn unlocked_skips_dirs_without_llm_env() {
    let tmp = TempDir::new().unwrap();
    fs::create_dir_all(tmp.path().join("empty")).unwrap();
    write_env(tmp.path(), "user_1", "OPENAI_API_KEY=k\n");
    let ids: Vec<_> = discover_in(tmp.path(), true)
        .iter()
        .map(|d| d.user_id.clone())
        .collect();
    assert_eq!(ids, vec!["user", "user_1"]);
}

#[test]
fn candidates_match_first_nonblank_env_name_per_provider() {
    let tmp = TempDir::new().unwrap();
    // Google has three env_names in priority order; only the second is set.
    write_env(tmp.path(), "user", "GEMINI_API_KEY=g1\n");
    write_env(tmp.path(), "user_1", "GOOGLE_API_KEY=g2\n");

    let mut pool =
        KeyPool::with_root(tmp.path().to_path_buf(), true).with_ttl(Duration::from_secs(0));
    let cands = pool.candidates("google");
    let ids: Vec<_> = cands.iter().map(|c| c.user_id.clone()).collect();
    assert_eq!(ids, vec!["user", "user_1"]);
    assert_eq!(cands[0].env_name, "GEMINI_API_KEY");
    assert_eq!(cands[0].key, "g1");
    assert_eq!(cands[1].env_name, "GOOGLE_API_KEY");
    assert_eq!(cands[1].key, "g2");
}

#[test]
fn candidates_skip_blank_values() {
    let tmp = TempDir::new().unwrap();
    write_env(tmp.path(), "user", "OPENAI_API_KEY=\n");
    write_env(tmp.path(), "user_1", "OPENAI_API_KEY=k1\n");
    let mut pool =
        KeyPool::with_root(tmp.path().to_path_buf(), true).with_ttl(Duration::from_secs(0));
    let cands = pool.candidates("openai");
    assert_eq!(cands.len(), 1);
    assert_eq!(cands[0].user_id, "user_1");
}

#[test]
fn newly_added_user_dir_is_picked_up_on_rescan() {
    let tmp = TempDir::new().unwrap();
    write_env(tmp.path(), "user", "OPENAI_API_KEY=k0\n");
    let mut pool =
        KeyPool::with_root(tmp.path().to_path_buf(), true).with_ttl(Duration::from_secs(0));
    assert_eq!(pool.candidates("openai").len(), 1);

    write_env(tmp.path(), "user_2", "OPENAI_API_KEY=k2\n");
    let users: Vec<_> = pool
        .candidates("openai")
        .into_iter()
        .map(|c| c.user_id)
        .collect();
    assert_eq!(users, vec!["user", "user_2"]);
}

#[test]
fn ttl_caches_within_window() {
    let tmp = TempDir::new().unwrap();
    write_env(tmp.path(), "user", "OPENAI_API_KEY=k0\n");
    let mut pool =
        KeyPool::with_root(tmp.path().to_path_buf(), true).with_ttl(Duration::from_secs(60));
    let first = pool.candidates("openai").len();
    write_env(tmp.path(), "user_1", "OPENAI_API_KEY=k1\n");
    // Within TTL we should still see only the originally-discovered dirs.
    assert_eq!(pool.candidates("openai").len(), first);
}

#[test]
fn unknown_provider_returns_no_candidates() {
    let tmp = TempDir::new().unwrap();
    write_env(tmp.path(), "user", "OPENAI_API_KEY=k\n");
    let mut pool =
        KeyPool::with_root(tmp.path().to_path_buf(), false).with_ttl(Duration::from_secs(0));
    assert!(pool.candidates("nope-not-real").is_empty());
}
