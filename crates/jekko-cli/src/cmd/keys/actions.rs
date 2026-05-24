use std::fs;

use anyhow::{Context, Result};

use super::args::{KeysDeleteArgs, KeysSetArgs, KeysStatusArgs};
use super::storage::{keys_path, read_env_lines, redact, write_env_lines};

pub(super) fn set(user_id: &str, args: &KeysSetArgs) -> Result<()> {
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

pub(super) fn list(user_id: &str) -> Result<()> {
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

pub(super) fn delete(user_id: &str, args: &KeysDeleteArgs) -> Result<()> {
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

pub(super) fn path(user_id: &str) -> Result<()> {
    let p = keys_path(user_id)?;
    println!("{}", p.display());
    Ok(())
}

pub(super) fn init(user_id: &str) -> Result<()> {
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

pub(super) fn status(user_id: &str, args: &KeysStatusArgs) -> Result<()> {
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
