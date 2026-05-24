use std::fs;
use std::sync::Mutex;

use tempfile::TempDir;

use super::actions::set;
use super::args::KeysSetArgs;
use super::storage::{
    enforce_user_gate, keys_path, migrate_existing_jekko_env, read_env_lines, redact,
};

// Tests mutate JEKKO_HOME/JNOCCIO_DEVELOPER_KEY; serialize.
static ENV_LOCK: Mutex<()> = Mutex::new(());

struct EnvGuard {
    prev_home: Option<std::ffi::OsString>,
    prev_dev: Option<std::ffi::OsString>,
    _lock: std::sync::MutexGuard<'static, ()>,
}

impl EnvGuard {
    fn install(home: &std::path::Path, dev_key: Option<&str>) -> Self {
        let lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let prev_home = std::env::var_os("JEKKO_HOME");
        let prev_dev = std::env::var_os("JNOCCIO_DEVELOPER_KEY");
        std::env::set_var("JEKKO_HOME", home);
        match dev_key {
            Some(v) => std::env::set_var("JNOCCIO_DEVELOPER_KEY", v),
            None => std::env::remove_var("JNOCCIO_DEVELOPER_KEY"),
        }
        EnvGuard {
            prev_home,
            prev_dev,
            _lock: lock,
        }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        match &self.prev_home {
            Some(v) => std::env::set_var("JEKKO_HOME", v),
            None => std::env::remove_var("JEKKO_HOME"),
        }
        match &self.prev_dev {
            Some(v) => std::env::set_var("JNOCCIO_DEVELOPER_KEY", v),
            None => std::env::remove_var("JNOCCIO_DEVELOPER_KEY"),
        }
    }
}

#[test]
fn migrates_existing_jekko_env_into_default_user_dir() {
    let tmp = TempDir::new().unwrap();
    let jekko = tmp.path().join(".jekko");
    fs::create_dir_all(&jekko).unwrap();
    fs::write(jekko.join("jekko.env"), "OPENAI_API_KEY=k\n").unwrap();
    let _guard = EnvGuard::install(&jekko, None);

    migrate_existing_jekko_env().unwrap();

    let new_path = jekko.join("users").join("user").join("llm.env");
    assert!(new_path.is_file(), "new keys file missing");
    assert_eq!(fs::read_to_string(&new_path).unwrap(), "OPENAI_API_KEY=k\n");
    let backup = jekko.join("jekko.env.bak");
    assert!(backup.is_file(), "backup missing");
    assert!(
        !jekko.join("jekko.env").exists(),
        "source file should be moved"
    );

    migrate_existing_jekko_env().unwrap();
    assert!(new_path.is_file());
}

#[test]
fn enforce_user_gate_blocks_non_default_when_locked() {
    assert!(enforce_user_gate("user", false).is_ok());
    let err = enforce_user_gate("user_1", false).unwrap_err().to_string();
    assert!(err.contains("requires JNOCCIO_DEVELOPER_KEY developer unlock"));
}

#[test]
fn enforce_user_gate_allows_non_default_when_unlocked() {
    assert!(enforce_user_gate("user_1", true).is_ok());
    assert!(enforce_user_gate("alice", true).is_ok());
}

#[test]
fn keys_path_resolves_under_jekko_home() {
    let tmp = TempDir::new().unwrap();
    let _guard = EnvGuard::install(tmp.path(), None);
    let p = keys_path("user").unwrap();
    assert_eq!(
        p,
        tmp.path()
            .join("users")
            .join("user")
            .join(jekko_provider::key_pool::LLM_ENV_FILENAME)
    );
}

#[test]
fn set_then_list_roundtrips() {
    let tmp = TempDir::new().unwrap();
    let _guard = EnvGuard::install(tmp.path(), None);
    set(
        "user",
        &KeysSetArgs {
            name: "OPENAI_API_KEY".into(),
            value: Some("sk-xyz".into()),
        },
    )
    .unwrap();
    let entries = read_env_lines("user").unwrap();
    assert_eq!(entries, vec![("OPENAI_API_KEY".into(), "sk-xyz".into())]);
}

#[test]
fn redact_short_values_uses_stars() {
    assert_eq!(redact("abc"), "***");
}

#[test]
fn redact_long_values_preserves_edges() {
    assert_eq!(redact("example-anthropic-key-1234567890"), "exam...7890");
}
