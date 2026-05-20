//! `jekko keys` — manage canonical model API keys.
//!
//! Keys live in `~/.jekko/users/<user_id>/llm.env`. The default user dir is
//! `user`, used by every install. Additional dirs (`user_1`, `user_2`, ...)
//! only unlock when jnoccio is unlocked
//! ([`jekko_jnoccio_boot::unlock::is_unlocked`]). When more than one user
//! dir is present, the runtime load-balances across the cross-product of
//! `(provider, user, model)` via [`jekko_provider::key_pool::KeyPool`] +
//! `jekko_runtime::key_balancer::KeyBalancer`.
//!
//! Examples:
//! ```text
//! jekko keys list
//! jekko keys set ANTHROPIC_API_KEY example-anthropic-key-0000000000000000
//! jekko keys --user user_1 set OPENROUTER_API_KEY sk-or-...
//! jekko keys delete OPENAI_API_KEY
//! jekko keys path
//! jekko keys users
//! ```

use std::fs;
use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use clap::{Args, Subcommand};
use jekko_provider::key_pool::{discover_user_dirs, user_dir, users_root, DEFAULT_USER_ID};
use jekko_provider::setup::parse_env_lines;

use crate::cli::GlobalOpts;

/// `jekko keys` args.
#[derive(Args, Debug)]
pub struct KeysArgs {
    /// User dir under `~/.jekko/users/<user>/`. Defaults to `user`. Extra
    /// user dirs require jnoccio unlock.
    #[arg(long, global = true, default_value = DEFAULT_USER_ID)]
    pub user: String,

    #[command(subcommand)]
    pub command: KeysCommand,
}

#[derive(Subcommand, Debug)]
pub enum KeysCommand {
    /// Set a key by name. Use `--value` or read from stdin.
    Set(KeysSetArgs),
    /// List currently configured keys (values redacted).
    List,
    /// Delete a key by name.
    Delete(KeysDeleteArgs),
    /// Print the canonical keys file path for the selected user.
    Path,
    /// Initialise the keys file if it does not exist.
    Init,
    /// Show machine-readable status.
    Status(KeysStatusArgs),
    /// List all detected user dirs and their key counts.
    Users(KeysUsersArgs),
}

#[derive(Args, Debug)]
pub struct KeysSetArgs {
    /// Key name (e.g. `ANTHROPIC_API_KEY`).
    pub name: String,
    /// Key value. When omitted, read from stdin.
    pub value: Option<String>,
}

#[derive(Args, Debug)]
pub struct KeysDeleteArgs {
    /// Key name.
    pub name: String,
}

#[derive(Args, Debug, Default)]
pub struct KeysStatusArgs {
    /// Emit machine-readable JSON.
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug, Default)]
pub struct KeysUsersArgs {
    /// Emit machine-readable JSON.
    #[arg(long)]
    pub json: bool,
}

pub fn run(_global: &GlobalOpts, args: &KeysArgs) -> Result<()> {
    migrate_existing_jekko_env()?;
    enforce_user_gate(&args.user, jekko_jnoccio_boot::unlock::is_unlocked())?;
    match &args.command {
        KeysCommand::Set(opts) => set(&args.user, opts),
        KeysCommand::List => list(&args.user),
        KeysCommand::Delete(opts) => delete(&args.user, opts),
        KeysCommand::Path => path(&args.user),
        KeysCommand::Init => init(&args.user),
        KeysCommand::Status(opts) => status(&args.user, opts),
        KeysCommand::Users(opts) => users(opts),
    }
}

/// Resolve `~/.jekko/users/<user_id>/llm.env`. Errors if `HOME` (and
/// `JEKKO_HOME`) are unavailable.
fn keys_path(user_id: &str) -> Result<PathBuf> {
    let root = users_root().ok_or_else(|| {
        anyhow!("HOME directory is required for `jekko keys`; export HOME and rerun")
    })?;
    Ok(user_dir(&root, user_id).llm_env_path)
}

/// Reject non-default user ids unless jnoccio is unlocked. `unlocked` is
/// injected by callers so tests can drive both paths deterministically.
fn enforce_user_gate(user_id: &str, unlocked: bool) -> Result<()> {
    if user_id == DEFAULT_USER_ID || unlocked {
        return Ok(());
    }
    Err(anyhow!(
        "creating extra users requires jnoccio unlock; got user `{user_id}`"
    ))
}

/// Move an existing single-user `~/.jekko/jekko.env` into
/// `~/.jekko/users/user/llm.env` on the first invocation after upgrade. Leaves
/// `~/.jekko/jekko.env.bak` behind so the original is recoverable. Idempotent:
/// does nothing once `users/` exists or the source file is already gone.
fn migrate_existing_jekko_env() -> Result<()> {
    let Some(users) = users_root() else {
        return Ok(());
    };
    if users.exists() {
        return Ok(());
    }
    let source = match users.parent() {
        Some(jekko) => jekko.join("jekko.env"),
        None => PathBuf::new(),
    };
    if !source.is_file() {
        return Ok(());
    }
    let default = user_dir(&users, DEFAULT_USER_ID);
    fs::create_dir_all(&default.dir)
        .with_context(|| format!("create default user dir at {}", default.dir.display()))?;
    fs::rename(&source, &default.llm_env_path).with_context(|| {
        format!(
            "move {} to {}",
            source.display(),
            default.llm_env_path.display()
        )
    })?;
    let backup = source.with_extension("env.bak");
    fs::copy(&default.llm_env_path, &backup)
        .with_context(|| format!("write previous-key backup at {}", backup.display()))?;
    eprintln!(
        "migrated {} → {} (backup at {})",
        source.display(),
        default.llm_env_path.display(),
        backup.display()
    );
    Ok(())
}

fn read_env_lines(user_id: &str) -> Result<Vec<(String, String)>> {
    let path = keys_path(user_id)?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let text = fs::read_to_string(&path)
        .with_context(|| format!("read keys file at {}", path.display()))?;
    Ok(parse_env_lines(&text))
}

fn write_env_lines(user_id: &str, entries: &[(String, String)]) -> Result<()> {
    let path = keys_path(user_id)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("create keys dir at {}", parent.display()))?;
    }
    let mut out = String::new();
    for (k, v) in entries {
        out.push_str(k);
        out.push('=');
        out.push_str(v);
        out.push('\n');
    }
    fs::write(&path, out).with_context(|| format!("write keys file at {}", path.display()))?;
    Ok(())
}

fn redact(value: &str) -> String {
    let len = value.chars().count();
    if len <= 8 {
        return "*".repeat(len);
    }
    let head: String = value.chars().take(4).collect();
    let tail: String = value.chars().skip(len.saturating_sub(4)).collect();
    format!("{head}...{tail}")
}

fn set(user_id: &str, args: &KeysSetArgs) -> Result<()> {
    let value = match args.value.as_deref() {
        Some(v) => v.to_string(),
        None => {
            use std::io::Read;
            let mut s = String::new();
            std::io::stdin().read_to_string(&mut s)?;
            s.trim().to_string()
        }
    };
    let mut entries = read_env_lines(user_id)?;
    if let Some(existing) = entries.iter_mut().find(|(k, _)| k == &args.name) {
        existing.1 = value;
    } else {
        entries.push((args.name.clone(), value));
    }
    write_env_lines(user_id, &entries)?;
    println!("set {} for user {user_id}", args.name);
    Ok(())
}

fn list(user_id: &str) -> Result<()> {
    let entries = read_env_lines(user_id)?;
    if entries.is_empty() {
        println!("no keys configured for user {user_id}");
        return Ok(());
    }
    for (k, v) in entries {
        println!("{k}={}", redact(&v));
    }
    Ok(())
}

fn delete(user_id: &str, args: &KeysDeleteArgs) -> Result<()> {
    let mut entries = read_env_lines(user_id)?;
    let before = entries.len();
    entries.retain(|(k, _)| k != &args.name);
    if entries.len() == before {
        anyhow::bail!("no such key: {} (user {user_id})", args.name);
    }
    write_env_lines(user_id, &entries)?;
    println!("deleted {} for user {user_id}", args.name);
    Ok(())
}

fn path(user_id: &str) -> Result<()> {
    let p = keys_path(user_id)?;
    println!("{}", p.display());
    Ok(())
}

fn init(user_id: &str) -> Result<()> {
    let p = keys_path(user_id)?;
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("create keys dir at {}", parent.display()))?;
    }
    if !p.exists() {
        let header = format!("# Jekko model keys for user `{user_id}`\n");
        fs::write(&p, header).with_context(|| format!("create keys file at {}", p.display()))?;
    }
    println!("{}", p.display());
    Ok(())
}

fn status(user_id: &str, args: &KeysStatusArgs) -> Result<()> {
    let entries = read_env_lines(user_id)?;
    let names: Vec<&str> = entries.iter().map(|(k, _)| k.as_str()).collect();
    if args.json {
        let blob = serde_json::json!({
            "user": user_id,
            "path": keys_path(user_id)?.display().to_string(),
            "count": names.len(),
            "keys": names,
        });
        println!("{}", serde_json::to_string_pretty(&blob)?);
    } else {
        println!("user: {user_id}");
        println!("path: {}", keys_path(user_id)?.display());
        println!("count: {}", names.len());
        for name in names {
            println!("  {name}");
        }
    }
    Ok(())
}

fn users(args: &KeysUsersArgs) -> Result<()> {
    let unlocked = jekko_jnoccio_boot::unlock::is_unlocked();
    let dirs = discover_user_dirs(unlocked);
    if args.json {
        let rows: Vec<_> = dirs
            .iter()
            .map(|d| {
                let count = fs::read_to_string(&d.llm_env_path)
                    .map(|t| parse_env_lines(&t).len())
                    .unwrap_or(0);
                serde_json::json!({
                    "user": d.user_id,
                    "path": d.llm_env_path.display().to_string(),
                    "exists": d.llm_env_path.is_file(),
                    "keys": count,
                })
            })
            .collect();
        let blob = serde_json::json!({
            "unlocked": unlocked,
            "users": rows,
        });
        println!("{}", serde_json::to_string_pretty(&blob)?);
        return Ok(());
    }
    println!("unlocked: {unlocked}");
    for d in &dirs {
        let count = fs::read_to_string(&d.llm_env_path)
            .map(|t| parse_env_lines(&t).len())
            .unwrap_or(0);
        let exists = if d.llm_env_path.is_file() {
            "ok"
        } else {
            "missing"
        };
        println!(
            "  {user:<10} {count:>3} keys  [{exists}]  {path}",
            user = d.user_id,
            path = d.llm_env_path.display()
        );
    }
    if !unlocked {
        println!("(unlock jnoccio to enable multi-user balancing)");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use tempfile::TempDir;

    // Tests mutate JEKKO_HOME/JNOCCIO_DEVELOPER_KEY — serialize.
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

        // Idempotent: second run is a no-op.
        migrate_existing_jekko_env().unwrap();
        assert!(new_path.is_file());
    }

    #[test]
    fn enforce_user_gate_blocks_non_default_when_locked() {
        assert!(enforce_user_gate("user", false).is_ok());
        let err = enforce_user_gate("user_1", false).unwrap_err().to_string();
        assert!(err.contains("requires jnoccio unlock"));
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
}
