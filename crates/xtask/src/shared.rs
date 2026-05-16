use anyhow::{bail, Context, Result};
use jekko_core::github::{parse_github_event_context, GitHubEventContext};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;

const DEFAULT_TARGET_DIR: &str = "target";

pub(crate) fn repo_root() -> Result<PathBuf> {
    std::env::current_dir().context("reading current directory")
}

pub(crate) fn migrations_json(root: &Path) -> Result<()> {
    let migrations = crate::migrations::collect(root)?;
    println!(
        "{}",
        serde_json::to_string(&migrations).context("serialise migrations")?
    );
    Ok(())
}

pub(crate) fn host_binary_path() -> Result<String> {
    let target_dir = match std::env::var("CARGO_TARGET_DIR") {
        Ok(value) => value,
        Err(_) => DEFAULT_TARGET_DIR.to_string(),
    };
    let exe = if cfg!(windows) { "jekko.exe" } else { "jekko" };
    let root = repo_root()?;
    let path = root.join(target_dir).join("debug").join(exe);
    Ok(path.display().to_string())
}

pub(crate) fn package_manager_version() -> Result<()> {
    let root = repo_root()?;
    let package_json = fs::read_to_string(root.join("package.json"))
        .context("read root package.json for package manager version")?;
    let parsed: serde_json::Value =
        serde_json::from_str(&package_json).context("parse root package.json")?;
    let package_manager = parsed
        .get("packageManager")
        .and_then(serde_json::Value::as_str)
        .context("package.json is missing packageManager")?;
    let Some(version) = package_manager.strip_prefix("bun@") else {
        bail!("packageManager must be bun@<version>, got {package_manager}");
    };
    println!("{version}");
    Ok(())
}

pub(crate) fn json_field(path: PathBuf, field: String) -> Result<()> {
    let text =
        fs::read_to_string(&path).with_context(|| format!("read JSON file {}", path.display()))?;
    let value: serde_json::Value = serde_json::from_str(&text).context("parse JSON file")?;
    let Some(found) = json_lookup(&value, &field) else {
        bail!("missing JSON field: {field}");
    };
    match found {
        serde_json::Value::String(value) => println!("{value}"),
        serde_json::Value::Number(value) => println!("{value}"),
        serde_json::Value::Bool(value) => println!("{value}"),
        serde_json::Value::Null => println!(),
        other => println!("{other}"),
    }
    Ok(())
}

pub(crate) fn json_lookup<'a>(
    value: &'a serde_json::Value,
    field: &str,
) -> Option<&'a serde_json::Value> {
    let mut current = value;
    for part in field.split('.') {
        current = current.as_object()?.get(part)?;
    }
    Some(current)
}

pub(crate) fn github_event(field: String) -> Result<()> {
    let context = current_github_event_context()?;
    let value = context
        .field(&field)
        .with_context(|| format!("unsupported or missing GitHub event field: {field}"))?;
    println!("{value}");
    Ok(())
}

pub(crate) fn current_github_event_context() -> Result<GitHubEventContext> {
    let event_path = std::env::var_os("GITHUB_EVENT_PATH")
        .map(PathBuf::from)
        .context("reading GITHUB_EVENT_PATH")?;
    let event_name = std::env::var("GITHUB_EVENT_NAME").context("reading GITHUB_EVENT_NAME")?;
    let repository = std::env::var("GITHUB_REPOSITORY").context("reading GITHUB_REPOSITORY")?;
    let actor = std::env::var("GITHUB_ACTOR").ok();
    let payload_text = fs::read_to_string(&event_path)
        .with_context(|| format!("read GitHub event payload {}", event_path.display()))?;
    let payload: serde_json::Value =
        serde_json::from_str(&payload_text).context("parse GitHub event payload JSON")?;
    parse_github_event_context(&event_name, &repository, actor.as_deref(), &payload)
        .context("parse GitHub event context")
}

pub(crate) fn home_dir() -> Result<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .context("reading HOME")
}

pub(crate) fn schema_check() -> Result<()> {
    let root = repo_root()?;
    let spec_path = root.join("docs").join("ZYAL").join("SPEC.md");
    let spec = fs::read_to_string(&spec_path)
        .with_context(|| format!("read ZYAL spec markdown {}", spec_path.display()))?;
    for needle in [
        "# ZYAL Spec",
        "## Metadata",
        "## Top-Level Blocks",
        "_Generated from `packages/jekko/src/agent-script/schema-spec.ts`._",
    ] {
        if !spec.contains(needle) {
            bail!("ZYAL spec markdown is missing expected marker: {needle}");
        }
    }
    println!("checked {}", spec_path.display());
    Ok(())
}

pub(crate) fn read_env_file(path: &Path) -> Result<HashMap<String, String>> {
    let text = fs::read_to_string(path).with_context(|| format!("read env file {path:?}"))?;
    let allowed = live_prod_allowed_keys();
    let mut env = HashMap::new();
    for line in text.lines() {
        let Some((key, value)) = parse_env_line(line, &allowed) else {
            continue;
        };
        let value = strip_quotes(&value);
        let value = if key.ends_with("_PATH") || key.ends_with("_FILE") {
            expand_home(&value)?
        } else {
            value
        };
        env.insert(key, value);
    }
    Ok(env)
}

pub(crate) fn existing_env_keys(text: &str) -> HashSet<String> {
    let allowed = live_prod_allowed_keys();
    text.lines()
        .filter_map(|line| parse_env_line(line, &allowed).map(|(key, _)| key))
        .collect()
}

pub(crate) fn candidate_home_env_files(home: &Path) -> Result<Vec<PathBuf>> {
    let mut entries = Vec::new();
    for entry in fs::read_dir(home).with_context(|| format!("read home directory {home:?}"))? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|v| v.to_str()) else {
            continue;
        };
        if name == ".env" || name.ends_with(".env") {
            entries.push(path);
        }
    }
    entries.sort();
    Ok(entries)
}

pub(crate) fn live_prod_allowed_keys() -> HashSet<&'static str> {
    [
        "JEKKO_API_KEY",
        "JEKKO_LIVE_MODEL",
        "JEKKO_TUI_LIVE_PROD",
        "JNOCCIO_DEFAULT_API_KEY",
        "JNOCCIO_DEFAULT_BASE_URL",
        "JNOCCIO_TUIWRIGHT_E2E",
        "JNOCCIO_TUI_TEST",
        "JNOCCIO_UNLOCK_SECRET_PATH",
    ]
    .into_iter()
    .collect()
}

pub(crate) fn parse_env_line(line: &str, allowed: &HashSet<&str>) -> Option<(String, String)> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }
    let body = trimmed.strip_prefix("export ").unwrap_or(trimmed);
    let (key, raw_value) = body.split_once('=')?;
    if key.is_empty()
        || !key
            .chars()
            .next()
            .is_some_and(|c| c == '_' || c.is_ascii_alphabetic())
        || !key.chars().all(|c| c == '_' || c.is_ascii_alphanumeric())
        || !allowed.contains(key)
    {
        return None;
    }
    Some((key.to_string(), raw_value.trim().to_string()))
}

pub(crate) fn strip_quotes(value: &str) -> String {
    let trimmed = value.trim();
    if (trimmed.starts_with('"') && trimmed.ends_with('"'))
        || (trimmed.starts_with('\'') && trimmed.ends_with('\''))
    {
        trimmed[1..trimmed.len().saturating_sub(1)].to_string()
    } else {
        trimmed.to_string()
    }
}

pub(crate) fn expand_home(value: &str) -> Result<String> {
    let home = home_dir()?;
    if value == "~" {
        return Ok(home.display().to_string());
    }
    if let Some(rest) = value.strip_prefix("~/") {
        return Ok(home.join(rest).display().to_string());
    }
    if let Some(rest) = value.strip_prefix("$HOME/") {
        return Ok(home.join(rest).display().to_string());
    }
    if let Some(rest) = value.strip_prefix("${HOME}/") {
        return Ok(home.join(rest).display().to_string());
    }
    Ok(value.to_string())
}

pub(crate) fn run_cargo_test(
    name: &str,
    args: &[&str],
    env: &HashMap<String, String>,
) -> Result<()> {
    println!("Running {name}");
    let mut cmd = ProcessCommand::new("cargo");
    cmd.args([
        "test",
        "--manifest-path",
        "crates/tuiwright-jekko-unlock/Cargo.toml",
    ]);
    cmd.args(args);
    cmd.current_dir(repo_root()?);
    cmd.stdout(std::process::Stdio::inherit());
    cmd.stderr(std::process::Stdio::inherit());
    for (key, value) in std::env::vars() {
        cmd.env(key, value);
    }
    for (key, value) in env {
        cmd.env(key, value);
    }
    let status = cmd.status().context("running cargo test")?;
    if !status.success() {
        bail!("{name} failed with exit {status}");
    }
    Ok(())
}
