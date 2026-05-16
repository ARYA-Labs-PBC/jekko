//! `xtask package` — build, strip, hash, and stage the `jekko` binary.
//!
//! 1. Build the release artifact (`cargo build -p jekko-cli --release --locked`).
//! 2. Strip symbols if `strip(1)` is available on `$PATH`.
//! 3. Compute the SHA-256 of the resulting binary.
//! 4. Copy the binary into `dist/jekko-<os>-<arch>/bin/jekko` and write
//!    `dist/jekko-<os>-<arch>/checksum.txt`.
//! 5. Print the staged path and hash.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};

/// Result of the staging step, returned for downstream callers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageReport {
    pub dist_dir: PathBuf,
    pub binary_path: PathBuf,
    pub checksum_path: PathBuf,
    pub sha256_hex: String,
    pub stripped: bool,
    pub target_triple: Option<String>,
    pub baseline: bool,
}

/// Compute the canonical platform/arch label used for the dist dir.
/// Example: `darwin-arm64`, `linux-x64`, `windows-arm64`.
pub fn dist_label() -> String {
    dist_label_for(None, false)
}

fn dist_label_for(target_triple: Option<&str>, baseline: bool) -> String {
    let Some(target_triple) = target_triple else {
        return dist_label_host_with_baseline(baseline);
    };

    let mut parts = target_triple.split('-');
    let arch = parts.next().unwrap_or("unknown");
    let _vendor = parts.next();
    let os = parts.next().unwrap_or("unknown");

    let os = match os {
        "apple" => "darwin",
        "pc" => "windows",
        "unknown" => dist_label_host_os(),
        other => other,
    };
    let arch = match arch {
        "x86_64" => "x64",
        "aarch64" => "arm64",
        other => other,
    };
    dist_label_apply_baseline(format!("{os}-{arch}"), baseline)
}

fn dist_label_host_with_baseline(baseline: bool) -> String {
    let os = match std::env::consts::OS {
        "macos" => "darwin",
        "linux" => "linux",
        "windows" => "windows",
        other => other,
    };
    let arch = match std::env::consts::ARCH {
        "x86_64" => "x86_64",
        "aarch64" => "arm64",
        other => other,
    };
    dist_label_apply_baseline(format!("{os}-{arch}"), baseline)
}

fn dist_label_apply_baseline(label: String, baseline: bool) -> String {
    if baseline {
        format!("{label}-baseline")
    } else {
        label
    }
}

/// Resolve the cargo target directory for `repo_root`. Honors
/// `CARGO_TARGET_DIR` only when it points inside `repo_root` (so unit
/// tests that fake the repo can override the resolution by simply not
/// having a matching prefix).
fn resolve_target_dir(repo_root: &Path) -> PathBuf {
    if let Some(raw) = std::env::var_os("CARGO_TARGET_DIR") {
        let candidate = PathBuf::from(&raw);
        let absolute = if candidate.is_absolute() {
            candidate
        } else {
            repo_root.join(candidate)
        };
        if absolute.starts_with(repo_root) {
            return absolute;
        }
    }
    repo_root.join("target")
}

/// Run the full package pipeline. `skip_build = true` reuses whatever
/// binary already exists in `target/release` (useful in tests).
pub fn run(
    repo_root: &Path,
    skip_build: bool,
    target_triple: Option<&str>,
    baseline: bool,
    dist_root: &Path,
) -> Result<PackageReport> {
    if !skip_build {
        build_release(repo_root, target_triple, baseline)?;
    }

    let target_dir = resolve_target_dir(repo_root);
    let exe_name = exe_name_for(target_triple);
    let built = target_dir
        .join(release_dir_name(target_triple))
        .join(exe_name);
    if !built.exists() {
        bail!(
            "expected release artifact at {} (did the build succeed?)",
            built.display()
        );
    }

    let stripped = strip_if_available(&built)?;
    let sha256_hex = sha256_of_file(&built)?;

    let label = match target_triple {
        Some(target_triple) => dist_label_for(Some(target_triple), baseline),
        None => dist_label_apply_baseline(dist_label(), baseline),
    };
    let dist_dir = resolve_dist_root(repo_root, dist_root).join(format!("jekko-{label}"));
    let bin_dir = dist_dir.join("bin");
    fs::create_dir_all(&bin_dir)
        .with_context(|| format!("create staging dir {}", bin_dir.display()))?;
    let staged = bin_dir.join(exe_name);
    fs::copy(&built, &staged)
        .with_context(|| format!("copy {} → {}", built.display(), staged.display()))?;

    let checksum_path = dist_dir.join("checksum.txt");
    let checksum_body = format!("{}  bin/{}\n", sha256_hex, exe_name);
    fs::write(&checksum_path, &checksum_body)
        .with_context(|| format!("write checksum {}", checksum_path.display()))?;

    println!("package: staged {}", staged.display());
    println!("package: sha256 {}", sha256_hex);
    println!("package: checksum {}", checksum_path.display());

    Ok(PackageReport {
        dist_dir,
        binary_path: staged,
        checksum_path,
        sha256_hex,
        stripped,
        target_triple: target_triple.map(ToOwned::to_owned),
        baseline,
    })
}

fn resolve_dist_root(repo_root: &Path, dist_root: &Path) -> PathBuf {
    if dist_root.is_absolute() {
        dist_root.to_path_buf()
    } else {
        repo_root.join(dist_root)
    }
}

fn build_release(repo_root: &Path, target_triple: Option<&str>, baseline: bool) -> Result<()> {
    let mut cmd = Command::new("cargo");
    cmd.args(["build", "-p", "jekko-cli", "--release", "--locked"]);
    if let Some(target_triple) = target_triple {
        cmd.args(["--target", target_triple]);
    }
    if baseline {
        cmd.env("RUSTFLAGS", "-C target-feature=-avx2");
    }
    let status = cmd
        .current_dir(repo_root)
        .status()
        .with_context(|| "spawn `cargo build -p jekko-cli --release --locked`")?;
    if !status.success() {
        bail!("cargo build failed with exit {status}");
    }
    Ok(())
}

/// Attempt to strip the binary in-place. Returns `Ok(true)` if `strip`
/// ran cleanly; `Ok(false)` if the tool is missing or refused. Other
/// failure modes propagate.
fn strip_if_available(path: &Path) -> Result<bool> {
    let probe = Command::new("strip").arg("--version").output();
    if probe.is_err() {
        return Ok(false);
    }
    let status = Command::new("strip")
        .arg(path)
        .status()
        .with_context(|| format!("run strip on {}", path.display()))?;
    Ok(status.success())
}

fn sha256_of_file(path: &Path) -> Result<String> {
    let bytes = fs::read(path).with_context(|| format!("read for hashing: {}", path.display()))?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let digest = hasher.finalize();
    Ok(hex::encode(digest))
}

const DEFAULT_RELEASE_DIR: &str = "release";

fn release_dir_name(target_triple: Option<&str>) -> String {
    match target_triple {
        Some(t) => format!("{t}/release"),
        None => DEFAULT_RELEASE_DIR.to_string(),
    }
}

fn exe_name_for(target_triple: Option<&str>) -> &'static str {
    if target_triple
        .map(|target| target.contains("windows"))
        .unwrap_or(cfg!(windows))
    {
        "jekko.exe"
    } else {
        "jekko"
    }
}

fn dist_label_host_os() -> &'static str {
    match std::env::consts::OS {
        "macos" => "darwin",
        "linux" => "linux",
        "windows" => "windows",
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dist_label_is_non_empty() {
        let label = dist_label();
        assert!(!label.is_empty());
        assert!(label.contains('-'));
    }

    #[test]
    fn sha256_of_file_matches_known_vector() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("file");
        fs::write(&path, b"hello").unwrap();
        // `printf 'hello' | sha256sum` →
        // 2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824
        let hex = sha256_of_file(&path).unwrap();
        assert_eq!(
            hex,
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
    }

    #[test]
    fn run_with_no_artifact_errors() {
        let dir = tempfile::tempdir().unwrap();
        // Pre-create the cargo target dir but no jekko binary.
        fs::create_dir_all(dir.path().join("target/release")).unwrap();
        let err = run(dir.path(), true, None, false, dir.path()).unwrap_err();
        let msg = format!("{err:#}");
        assert!(msg.contains("expected release artifact"));
    }

    #[test]
    fn run_skip_build_stages_existing_binary() {
        let dir = tempfile::tempdir().unwrap();
        let release_dir = dir.path().join("target/release");
        fs::create_dir_all(&release_dir).unwrap();
        let exe_name = if cfg!(windows) { "jekko.exe" } else { "jekko" };
        let fake_binary = release_dir.join(exe_name);
        fs::write(&fake_binary, b"fake-binary-contents").unwrap();
        let report = run(dir.path(), true, None, false, dir.path()).unwrap();
        assert!(report.binary_path.exists());
        assert_eq!(
            fs::read(&report.binary_path).unwrap(),
            b"fake-binary-contents"
        );
        let checksum_text = fs::read_to_string(&report.checksum_path).unwrap();
        assert!(checksum_text.contains(&report.sha256_hex));
    }

    #[test]
    fn run_skip_build_respects_custom_dist_root() {
        let dir = tempfile::tempdir().unwrap();
        let release_dir = dir.path().join("target/release");
        fs::create_dir_all(&release_dir).unwrap();
        let exe_name = if cfg!(windows) { "jekko.exe" } else { "jekko" };
        let fake_binary = release_dir.join(exe_name);
        fs::write(&fake_binary, b"fake-binary-contents").unwrap();
        let dist_root = dir.path().join("packages/jekko/dist");
        let report = run(dir.path(), true, None, false, &dist_root).unwrap();
        assert!(report.dist_dir.starts_with(&dist_root));
        assert!(report.binary_path.starts_with(&dist_root));
        assert!(report.checksum_path.starts_with(&dist_root));
    }

    #[test]
    fn dist_label_for_target_triples_is_normalized() {
        assert_eq!(
            dist_label_for(Some("x86_64-unknown-linux-gnu"), false),
            "linux-x64"
        );
        assert_eq!(
            dist_label_for(Some("aarch64-apple-darwin"), false),
            "darwin-arm64"
        );
        assert_eq!(
            dist_label_for(Some("x86_64-pc-windows-msvc"), false),
            "windows-x64"
        );
        assert_eq!(
            dist_label_for(Some("x86_64-unknown-linux-gnu"), true),
            "linux-x64-baseline"
        );
    }
}
