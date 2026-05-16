use anyhow::{bail, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const DEFAULT_REPOSITORY: &str = "neverhuman/jekko";

pub fn run(dist_root: &Path, version: &str, release: bool, repo: Option<&str>) -> Result<()> {
    let mut archives = Vec::new();
    for package_dir in collect_package_dirs(dist_root)? {
        stage_package_manifest(&package_dir, version)?;
        remove_tui_dir(&package_dir.join("bin/tui"))?;
        if release {
            archives.push(archive_package(&package_dir)?);
        }
    }

    if release {
        upload_archives(dist_root, version, repo, &archives)?;
    }
    Ok(())
}

fn collect_package_dirs(dist_root: &Path) -> Result<Vec<PathBuf>> {
    let mut dirs = Vec::new();
    for entry in fs::read_dir(dist_root).with_context(|| format!("read {}", dist_root.display()))? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let package_dir = entry.path();
        if package_dir.join("bin").is_dir() {
            dirs.push(package_dir);
        }
    }
    dirs.sort();
    Ok(dirs)
}

fn stage_package_manifest(package_dir: &Path, version: &str) -> Result<()> {
    let name = package_dir
        .file_name()
        .and_then(|value| value.to_str())
        .context("package dir missing utf8 name")?;
    let (os, cpu) = package_labels(name)?;
    let package = serde_json::json!({
        "name": name,
        "version": version,
        "os": [os],
        "cpu": [cpu],
    });
    fs::write(
        package_dir.join("package.json"),
        serde_json::to_string_pretty(&package).context("serialise package.json")?,
    )
    .with_context(|| format!("write {}", package_dir.join("package.json").display()))?;
    Ok(())
}

fn package_labels(name: &str) -> Result<(&'static str, &'static str)> {
    let label = name
        .strip_prefix("jekko-")
        .with_context(|| format!("unexpected package name {name}"))?;
    let mut parts = label.split('-');
    let os = match parts.next().context("missing os")? {
        "windows" => "win32",
        "darwin" => "darwin",
        "linux" => "linux",
        other => bail!("unexpected os token {other}"),
    };
    let cpu = parts.next().context("missing cpu")?;
    let cpu = match cpu {
        "x64" => "x64",
        "arm64" => "arm64",
        other => bail!("unexpected cpu token {other}"),
    };
    Ok((os, cpu))
}

fn remove_tui_dir(path: &Path) -> Result<()> {
    if path.exists() {
        fs::remove_dir_all(path).with_context(|| format!("remove {}", path.display()))?;
    }
    Ok(())
}

fn archive_package(package_dir: &Path) -> Result<PathBuf> {
    let name = package_dir
        .file_name()
        .and_then(|value| value.to_str())
        .context("package dir missing utf8 name")?;
    let archive = if name.contains("linux") {
        package_dir
            .parent()
            .context("missing dist parent")?
            .join(format!("{name}.tar.gz"))
    } else {
        package_dir
            .parent()
            .context("missing dist parent")?
            .join(format!("{name}.zip"))
    };

    let status = if name.contains("linux") {
        Command::new("tar")
            .args(["-czf", archive.file_name().unwrap().to_str().unwrap(), "."])
            .current_dir(package_dir.join("bin"))
            .status()
    } else {
        Command::new("zip")
            .args(["-r", archive.file_name().unwrap().to_str().unwrap(), "."])
            .current_dir(package_dir.join("bin"))
            .status()
    }
    .with_context(|| format!("archive {}", package_dir.display()))?;
    if !status.success() {
        bail!("archive command failed with status {status}");
    }
    Ok(archive)
}

fn upload_archives(
    dist_root: &Path,
    version: &str,
    repo: Option<&str>,
    archives: &[PathBuf],
) -> Result<()> {
    let repo = match repo.map(ToOwned::to_owned) {
        Some(value) => value,
        None => match std::env::var("GH_REPO").ok() {
            Some(value) => value,
            None => DEFAULT_REPOSITORY.to_string(),
        },
    };
    let mut cmd = Command::new("gh");
    cmd.args(["release", "upload", &format!("v{version}")]);
    for archive in archives {
        cmd.arg(archive);
    }
    cmd.args(["--clobber", "--repo", &repo])
        .current_dir(dist_root);
    let status = cmd.status().with_context(|| "run gh release upload")?;
    if !status.success() {
        bail!("gh release upload failed with status {status}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collect_package_dirs_finds_binaries() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("jekko-linux-x64/bin")).unwrap();
        fs::create_dir_all(dir.path().join("jekko-windows-x64/bin")).unwrap();
        fs::create_dir_all(dir.path().join("ignored")).unwrap();
        let dirs = collect_package_dirs(dir.path()).unwrap();
        assert_eq!(dirs.len(), 2);
    }

    #[test]
    fn package_labels_maps_windows_to_win32() {
        let (os, cpu) = package_labels("jekko-windows-x64").unwrap();
        assert_eq!(os, "win32");
        assert_eq!(cpu, "x64");
    }
}
