use std::fs;
use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use jekko_provider::key_pool::{user_dir, users_root, DEFAULT_USER_ID};
use jekko_provider::setup::parse_env_lines;

/// Resolve `~/.jekko/users/<user_id>/llm.env`.
pub(super) fn keys_path(user_id: &str) -> Result<PathBuf> {
    let root = users_root().ok_or_else(|| {
        anyhow!("HOME directory is required for `jekko keys`; export HOME and rerun")
    })?;
    Ok(user_dir(&root, user_id).llm_env_path)
}

/// Reject non-default user ids unless Jnoccio developer unlock is present.
pub(super) fn enforce_user_gate(user_id: &str, unlocked: bool) -> Result<()> {
    if user_id == DEFAULT_USER_ID || unlocked {
        return Ok(());
    }
    Err(anyhow!(
        "creating extra users requires JNOCCIO_DEVELOPER_KEY developer unlock; got user `{user_id}`"
    ))
}

/// Move an existing single-user `~/.jekko/jekko.env` into
/// `~/.jekko/users/user/llm.env` on the first invocation after upgrade.
pub(super) fn migrate_existing_jekko_env() -> Result<()> {
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

pub(super) fn read_env_lines(user_id: &str) -> Result<Vec<(String, String)>> {
    let path = keys_path(user_id)?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let text = fs::read_to_string(&path)
        .with_context(|| format!("read keys file at {}", path.display()))?;
    Ok(parse_env_lines(&text))
}

pub(super) fn write_env_lines(user_id: &str, entries: &[(String, String)]) -> Result<()> {
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

pub(super) fn redact(value: &str) -> String {
    let len = value.chars().count();
    if len <= 8 {
        return "*".repeat(len);
    }
    let head: String = value.chars().take(4).collect();
    let tail: String = value.chars().skip(len.saturating_sub(4)).collect();
    format!("{head}...{tail}")
}
