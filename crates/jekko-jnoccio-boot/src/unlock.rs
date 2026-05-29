//! Unlock detection for Jnoccio Fusion.
//!
//! Mirrors the logic in `packages/jekko/src/util/jnoccio-unlock.ts`:
//! `isJnoccioFusionUnlocked` / `isJnoccioFusionConfigured` / `hasPlaintextSignals`.
//!
//! # Unlock hierarchy (left-to-right, fast exit)
//!
//! 1. `JNOCCIO_DEVELOPER_KEY` env var is set and non-empty → unlocked.
//! 2. `~/.env.jnoccio` contains a non-empty `JNOCCIO_DEVELOPER_KEY=...` → unlocked.
//!
//! Plaintext `jnoccio-fusion/` files remain a diagnostic/configuration signal,
//! but plaintext alone never unlocks developer-only runtime paths.
//!
//! Note: checks are read-only and cheap. No crypto is performed here.

use std::fs;
use std::path::{Path, PathBuf};

/// Returns `true` if the developer unlock signal is present, meaning the
/// current machine has explicit access to run local Jnoccio Fusion paths.
pub fn is_unlocked() -> bool {
    developer_key().is_some()
}

/// Returns the developer key when it is present in process env or
/// `~/.env.jnoccio`. This is intentionally the only unlock source.
pub fn developer_key() -> Option<String> {
    if let Ok(value) = std::env::var("JNOCCIO_DEVELOPER_KEY") {
        if !value.trim().is_empty() {
            tracing::debug!("jnoccio unlocked via JNOCCIO_DEVELOPER_KEY env var");
            return Some(value);
        }
    }

    let home = home_dir()?;
    let env_file = home.join(".env.jnoccio");
    let text = fs::read_to_string(&env_file).ok()?;
    let key = developer_key_from_env_text(&text)?;
    tracing::debug!("jnoccio unlocked via {}", env_file.display());
    Some(key)
}

fn developer_key_from_env_text(text: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            return None;
        }
        let trimmed = trimmed.strip_prefix("export ").unwrap_or(trimmed).trim();
        let (name, value) = trimmed.split_once('=')?;
        if name.trim() != "JNOCCIO_DEVELOPER_KEY" {
            return None;
        }
        let value = value.trim();
        let value = value
            .strip_prefix('"')
            .and_then(|v| v.strip_suffix('"'))
            .or_else(|| value.strip_prefix('\'').and_then(|v| v.strip_suffix('\'')))
            .unwrap_or(value)
            .trim();
        (!value.is_empty()).then(|| value.to_string())
    })
}

/// Returns `true` if the encrypted subtree is readable as plaintext. This is
/// useful for diagnostics and setup status, but it does not unlock runtime
/// developer behavior.
pub fn has_plaintext_checkout() -> bool {
    if let Some(root) = find_repo_root() {
        return has_plaintext_signals(&root);
    }
    false
}

/// Find the `jnoccio-fusion/` directory root.
///
/// Resolution is deliberately **cwd-independent** so jekko boots the gateway
/// correctly no matter which folder the user launches it from. The returned
/// path is canonicalized when possible. Order (first hit wins):
///
/// 1. `JEKKO_FUSION_ROOT` env override — either the `jnoccio-fusion/` dir
///    itself or a repo root that contains one.
/// 2. Walk up from the **running executable** (`current_exe`) for a
///    `jnoccio-fusion/` subdir — the dev case, e.g. `<repo>/target/release/jekko`
///    resolves to `<repo>/jnoccio-fusion` regardless of `$PWD`.
/// 3. `$XDG_CONFIG_HOME/jekko/jnoccio-fusion` (default
///    `$HOME/.config/jekko/jnoccio-fusion`) — the installed bundle layout.
/// 4. Walk up from `$PWD` (legacy dev convenience) — kept last so it never
///    shadows the stable anchors above.
///
/// Returns `None` only when every source is missing.
pub fn find_jnoccio_fusion_root() -> Option<PathBuf> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    find_jnoccio_fusion_root_from(&cwd)
}

/// Find the `jnoccio-fusion/` directory root, using `start` as the final
/// (legacy) walk-up base instead of `$PWD`. The stable anchors (env var,
/// executable location, installed bundle) still take precedence over `start`
/// so explicit-base callers (e.g. the runtime auto-boot path) are equally
/// robust to the launch directory.
pub fn find_jnoccio_fusion_root_from(start: &Path) -> Option<PathBuf> {
    // 1. Explicit override.
    if let Some(root) = fusion_root_from_env() {
        tracing::debug!(root = %root.display(), "jnoccio-fusion root from JEKKO_FUSION_ROOT");
        return Some(root);
    }
    // 2. The jekko binary's own location (independent of cwd).
    if let Some(root) = fusion_root_from_exe() {
        tracing::debug!(root = %root.display(), "jnoccio-fusion root from executable path");
        return Some(root);
    }
    // 3. Installed bundle layout.
    if let Some(root) = installed_bundle_root() {
        tracing::debug!(root = %root.display(), "jnoccio-fusion root from installed bundle");
        return Some(root);
    }
    // 4. Legacy: walk up from the provided start path.
    if let Some(root) = fusion_root_in_ancestors(start) {
        tracing::debug!(root = %root.display(), "jnoccio-fusion root from ancestor walk");
        return Some(root);
    }
    None
}

/// Resolve `JEKKO_FUSION_ROOT`, accepting either the `jnoccio-fusion/` dir
/// itself or a parent directory that contains one.
fn fusion_root_from_env() -> Option<PathBuf> {
    let raw = std::env::var_os("JEKKO_FUSION_ROOT")?;
    if raw.is_empty() {
        return None;
    }
    fusion_dir_from_candidate(&PathBuf::from(raw))
}

/// Walk up from the running executable looking for a sibling `jnoccio-fusion/`.
fn fusion_root_from_exe() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let start = exe.parent()?;
    fusion_root_in_ancestors(start)
}

/// The installed bundle root (`$XDG_CONFIG_HOME/jekko/jnoccio-fusion`), if it
/// exists as a directory.
fn installed_bundle_root() -> Option<PathBuf> {
    let bundle = xdg_config_home()?.join("jekko").join("jnoccio-fusion");
    bundle.is_dir().then(|| canonicalize_or(&bundle))
}

/// Walk `start` and its ancestors (≤10 levels) for a `jnoccio-fusion/` subdir,
/// returning that subdir (the fusion root), canonicalized when possible.
fn fusion_root_in_ancestors(start: &Path) -> Option<PathBuf> {
    let repo = find_repo_root_from(start)?;
    let candidate = repo.join("jnoccio-fusion");
    candidate.is_dir().then(|| canonicalize_or(&candidate))
}

/// Map a `JEKKO_FUSION_ROOT` candidate to a concrete `jnoccio-fusion/` dir:
/// accept the directory directly when it is already named `jnoccio-fusion`, or
/// descend into a `jnoccio-fusion/` child when the candidate is a repo root.
fn fusion_dir_from_candidate(path: &Path) -> Option<PathBuf> {
    if !path.is_dir() {
        return None;
    }
    if path.file_name().and_then(|name| name.to_str()) == Some("jnoccio-fusion") {
        return Some(canonicalize_or(path));
    }
    let nested = path.join("jnoccio-fusion");
    nested.is_dir().then(|| canonicalize_or(&nested))
}

/// Canonicalize a path, falling back to the input on error so callers always
/// get a usable (if non-canonical) path.
fn canonicalize_or(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// Resolve `$XDG_CONFIG_HOME`, defaulting to `$HOME/.config` (XDG base-dir
/// spec default). Returns `None` only if neither variable is set.
fn xdg_config_home() -> Option<PathBuf> {
    if let Ok(value) = std::env::var("XDG_CONFIG_HOME") {
        if !value.is_empty() {
            return Some(PathBuf::from(value));
        }
    }
    home_dir().map(|h| h.join(".config"))
}

/// Walk `$PWD` (then its ancestors) looking for a `jnoccio-fusion/` subdirectory.
/// Returns the **repo root** (parent of `jnoccio-fusion/`), not the subtree itself.
pub fn find_repo_root() -> Option<PathBuf> {
    let cwd = std::env::current_dir().ok()?;
    find_repo_root_from(&cwd)
}

/// Find the repository root from an explicit starting path.
pub fn find_repo_root_from(start: &Path) -> Option<PathBuf> {
    for ancestor in start.ancestors().take(10) {
        if ancestor.join("jnoccio-fusion").is_dir() {
            return Some(ancestor.to_path_buf());
        }
    }
    None
}

/// Checks whether the `jnoccio-fusion/` subtree inside `repo_root` has been
/// git-crypt unlocked (i.e. the files are readable plaintext, not encrypted binary).
///
/// Mirrors `hasPlaintextSignals` from `jnoccio-unlock.ts`.
pub fn has_plaintext_signals(repo_root: &Path) -> bool {
    let fusion_root = repo_root.join("jnoccio-fusion");

    // Check 1: Cargo.toml must mention the expected package name.
    let cargo_path = fusion_root.join("Cargo.toml");
    let Ok(cargo_text) = fs::read_to_string(&cargo_path) else {
        return false;
    };
    if !cargo_text.contains("[package]") || !cargo_text.contains("name = \"jnoccio-fusion\"") {
        return false;
    }

    // Check 2: config/server.json must have the provider/model fields readable.
    let config_path = fusion_root.join("config").join("server.json");
    let Ok(config_text) = fs::read_to_string(&config_path) else {
        return false;
    };
    config_text.contains("\"jnoccio\"")
        && (config_text.contains("\"jnoccio-fusion\"")
            || config_text.contains("\"jnoccio/jnoccio-fusion\""))
}

/// Returns the `.env.jnoccio` path inside the `jnoccio-fusion/` subtree if it
/// exists (a fully configured + unlocked install also has this file with API
/// keys written by `jekko jnoccio unlock`).
pub fn jnoccio_env_path(repo_root: &Path) -> PathBuf {
    repo_root.join("jnoccio-fusion").join(".env.jnoccio")
}

/// Returns `true` if the repo is readable AND the `.env.jnoccio` file exists
/// (meaning `jekko jnoccio unlock` was previously completed successfully).
pub fn is_configured(repo_root: &Path) -> bool {
    has_plaintext_signals(repo_root) && jnoccio_env_path(repo_root).exists()
}

fn home_dir() -> Option<PathBuf> {
    std::env::var("HOME")
        .ok()
        .map(PathBuf::from)
        .filter(|p| p.is_dir())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::Mutex;
    use tempfile::TempDir;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    struct EnvGuard {
        prev_home: Option<std::ffi::OsString>,
        prev_dev: Option<std::ffi::OsString>,
        prev_cwd: PathBuf,
        _lock: std::sync::MutexGuard<'static, ()>,
    }

    impl EnvGuard {
        fn install(home: &Path, cwd: Option<&Path>, dev_key: Option<&str>) -> Self {
            let lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
            let prev_home = std::env::var_os("HOME");
            let prev_dev = std::env::var_os("JNOCCIO_DEVELOPER_KEY");
            let prev_cwd = std::env::current_dir().unwrap();
            std::env::set_var("HOME", home);
            match dev_key {
                Some(v) => std::env::set_var("JNOCCIO_DEVELOPER_KEY", v),
                None => std::env::remove_var("JNOCCIO_DEVELOPER_KEY"),
            }
            if let Some(cwd) = cwd {
                std::env::set_current_dir(cwd).unwrap();
            }
            Self {
                prev_home,
                prev_dev,
                prev_cwd,
                _lock: lock,
            }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            std::env::set_current_dir(&self.prev_cwd).unwrap();
            match &self.prev_home {
                Some(v) => std::env::set_var("HOME", v),
                None => std::env::remove_var("HOME"),
            }
            match &self.prev_dev {
                Some(v) => std::env::set_var("JNOCCIO_DEVELOPER_KEY", v),
                None => std::env::remove_var("JNOCCIO_DEVELOPER_KEY"),
            }
        }
    }

    fn make_plaintext_signals(root: &Path) {
        let fusion = root.join("jnoccio-fusion");
        fs::create_dir_all(fusion.join("config")).unwrap();
        fs::write(
            fusion.join("Cargo.toml"),
            "[package]\nname = \"jnoccio-fusion\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        fs::write(
            fusion.join("config").join("server.json"),
            r#"{"provider":"jnoccio","model":"jnoccio-fusion"}"#,
        )
        .unwrap();
    }

    #[test]
    fn detects_plaintext_signals() {
        let tmp = TempDir::new().unwrap();
        make_plaintext_signals(tmp.path());
        assert!(has_plaintext_signals(tmp.path()));
    }

    #[test]
    fn detects_namespaced_plaintext_model_signal() {
        let tmp = TempDir::new().unwrap();
        make_plaintext_signals(tmp.path());
        fs::write(
            tmp.path()
                .join("jnoccio-fusion")
                .join("config")
                .join("server.json"),
            r#"{"provider":"jnoccio","model":"jnoccio/jnoccio-fusion"}"#,
        )
        .unwrap();
        assert!(has_plaintext_signals(tmp.path()));
    }

    #[test]
    fn rejects_encrypted_signals() {
        let tmp = TempDir::new().unwrap();
        let fusion = tmp.path().join("jnoccio-fusion");
        fs::create_dir_all(fusion.join("config")).unwrap();
        // Encrypted binary content (git-crypt format — not valid UTF-8 / JSON)
        fs::write(fusion.join("Cargo.toml"), b"\x00GITCRYPT\x00\x02encrypted").unwrap();
        assert!(!has_plaintext_signals(tmp.path()));
    }

    #[test]
    fn is_configured_requires_env_file() {
        let tmp = TempDir::new().unwrap();
        make_plaintext_signals(tmp.path());
        // Plaintext but no .env.jnoccio yet
        assert!(!is_configured(tmp.path()));

        // Write the env file
        fs::write(
            tmp.path().join("jnoccio-fusion").join(".env.jnoccio"),
            "JNOCCIO_DEFAULT_API_KEY=test\n",
        )
        .unwrap();
        assert!(is_configured(tmp.path()));
    }

    #[test]
    fn is_unlocked_requires_developer_key_even_with_plaintext_signals() {
        let home = TempDir::new().unwrap();
        let repo = TempDir::new().unwrap();
        make_plaintext_signals(repo.path());
        let _guard = EnvGuard::install(home.path(), Some(repo.path()), None);

        assert!(has_plaintext_checkout());
        assert!(!is_unlocked());
    }

    #[test]
    fn env_file_existence_alone_does_not_unlock() {
        let home = TempDir::new().unwrap();
        fs::write(home.path().join(".env.jnoccio"), "# local jnoccio file\n").unwrap();
        let _guard = EnvGuard::install(home.path(), None, None);

        assert!(!is_unlocked());
    }

    #[test]
    fn env_file_with_developer_key_unlocks() {
        let home = TempDir::new().unwrap();
        fs::write(
            home.path().join(".env.jnoccio"),
            "JNOCCIO_DEVELOPER_KEY=file-secret\n",
        )
        .unwrap();
        let _guard = EnvGuard::install(home.path(), None, None);

        assert_eq!(developer_key().as_deref(), Some("file-secret"));
        assert!(is_unlocked());
    }

    #[test]
    fn process_env_developer_key_unlocks() {
        let home = TempDir::new().unwrap();
        let _guard = EnvGuard::install(home.path(), None, Some("process-secret"));

        assert_eq!(developer_key().as_deref(), Some("process-secret"));
        assert!(is_unlocked());
    }

    // ── Fusion-root discovery (cwd-independent) ─────────────────────────────

    #[test]
    fn candidate_accepts_direct_fusion_dir() {
        let tmp = TempDir::new().unwrap();
        let fusion = tmp.path().join("jnoccio-fusion");
        fs::create_dir_all(&fusion).unwrap();
        let got = fusion_dir_from_candidate(&fusion).unwrap();
        assert_eq!(got, fs::canonicalize(&fusion).unwrap());
    }

    #[test]
    fn candidate_descends_into_fusion_child() {
        let tmp = TempDir::new().unwrap();
        let fusion = tmp.path().join("jnoccio-fusion");
        fs::create_dir_all(&fusion).unwrap();
        let got = fusion_dir_from_candidate(tmp.path()).unwrap();
        assert_eq!(got, fs::canonicalize(&fusion).unwrap());
    }

    #[test]
    fn candidate_none_when_absent() {
        let tmp = TempDir::new().unwrap();
        assert!(fusion_dir_from_candidate(tmp.path()).is_none());
    }

    #[test]
    fn ancestors_walk_finds_fusion_root() {
        let tmp = TempDir::new().unwrap();
        let fusion = tmp.path().join("jnoccio-fusion");
        let nested = tmp.path().join("a").join("b").join("c");
        fs::create_dir_all(&fusion).unwrap();
        fs::create_dir_all(&nested).unwrap();
        let got = fusion_root_in_ancestors(&nested).unwrap();
        assert_eq!(got, fs::canonicalize(&fusion).unwrap());
    }

    #[test]
    fn env_override_resolves_repo_root_and_fusion_dir() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let tmp = TempDir::new().unwrap();
        let fusion = tmp.path().join("jnoccio-fusion");
        fs::create_dir_all(&fusion).unwrap();
        let prev = std::env::var_os("JEKKO_FUSION_ROOT");

        // Point at the repo root (parent of jnoccio-fusion/).
        std::env::set_var("JEKKO_FUSION_ROOT", tmp.path());
        let from_repo = fusion_root_from_env();
        // Point directly at the jnoccio-fusion/ dir.
        std::env::set_var("JEKKO_FUSION_ROOT", &fusion);
        let from_dir = fusion_root_from_env();

        match prev {
            Some(v) => std::env::set_var("JEKKO_FUSION_ROOT", v),
            None => std::env::remove_var("JEKKO_FUSION_ROOT"),
        }

        let expected = fs::canonicalize(&fusion).unwrap();
        assert_eq!(from_repo.unwrap(), expected);
        assert_eq!(from_dir.unwrap(), expected);
    }

    #[test]
    fn installed_bundle_resolves_under_xdg_default() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let home = TempDir::new().unwrap();
        let bundle = home
            .path()
            .join(".config")
            .join("jekko")
            .join("jnoccio-fusion");
        fs::create_dir_all(&bundle).unwrap();

        let prev_home = std::env::var_os("HOME");
        let prev_xdg = std::env::var_os("XDG_CONFIG_HOME");
        std::env::set_var("HOME", home.path());
        std::env::remove_var("XDG_CONFIG_HOME");

        let got = installed_bundle_root();

        match prev_home {
            Some(v) => std::env::set_var("HOME", v),
            None => std::env::remove_var("HOME"),
        }
        match prev_xdg {
            Some(v) => std::env::set_var("XDG_CONFIG_HOME", v),
            None => std::env::remove_var("XDG_CONFIG_HOME"),
        }

        assert_eq!(got.unwrap(), fs::canonicalize(&bundle).unwrap());
    }
}
