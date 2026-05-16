use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const DEFAULT_LOCAL_VERSION: &str = "0.0.0-local";

pub(super) fn resolve_version() -> String {
    match env::var("JEKKO_VERSION").ok().filter(|v| !v.is_empty()) {
        Some(value) => value,
        None => DEFAULT_LOCAL_VERSION.to_string(),
    }
}

pub(super) fn resolve_target_dir(repo_root: &Path) -> PathBuf {
    if let Some(raw) = env::var_os("CARGO_TARGET_DIR") {
        let candidate = PathBuf::from(&raw);
        if candidate.is_absolute() {
            return candidate;
        }
        return repo_root.join(candidate);
    }
    repo_root.join("target")
}

pub(super) fn sha256_of_file(path: &Path) -> Result<String> {
    let bytes = fs::read(path).with_context(|| format!("read for hashing: {}", path.display()))?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    Ok(hex::encode(hasher.finalize()))
}

pub(super) fn current_host_triple() -> &'static str {
    // The rustc that compiled xtask reports the host triple via std::env::consts;
    // we synthesize it the same way `cargo build` would default.
    match (env::consts::OS, env::consts::ARCH) {
        ("macos", "aarch64") => "aarch64-apple-darwin",
        ("macos", "x86_64") => "x86_64-apple-darwin",
        ("linux", "aarch64") => "aarch64-unknown-linux-gnu",
        ("linux", "x86_64") => "x86_64-unknown-linux-gnu",
        ("windows", "aarch64") => "aarch64-pc-windows-msvc",
        ("windows", "x86_64") => "x86_64-pc-windows-msvc",
        _ => "",
    }
}

pub(super) fn is_command_available(name: &str) -> bool {
    Command::new(name)
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

pub(super) fn installed_rustup_targets() -> Vec<String> {
    let Ok(output) = Command::new("rustup")
        .args(["target", "list", "--installed"])
        .output()
    else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect()
}
