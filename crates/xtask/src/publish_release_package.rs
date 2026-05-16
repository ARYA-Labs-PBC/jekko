use anyhow::{bail, ensure, Context, Result};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn run(package_dir: &Path, tag: &str) -> Result<()> {
    let package_json = package_dir.join("package.json");
    let package_text = fs::read_to_string(&package_json)
        .with_context(|| format!("read {}", package_json.display()))?;
    let package: Value = serde_json::from_str(&package_text)
        .with_context(|| format!("parse {}", package_json.display()))?;
    let (name, version) = package_identity(&package)?;

    if published(name, version)? {
        println!("already published {name}@{version}");
        return Ok(());
    }

    if !cfg!(windows) {
        run_checked(
            Command::new("chmod")
                .args(["-R", "755", "."])
                .current_dir(package_dir),
            &format!("chmod package tree in {}", package_dir.display()),
        )?;
    }

    let tarball = package_tarball(package_dir)?;
    run_checked(
        Command::new("npm")
            .args(["publish", &tarball, "--tag", tag, "--access", "public"])
            .current_dir(package_dir),
        &format!("npm publish for {}", package_dir.display()),
    )?;
    Ok(())
}

pub fn run_all(package_root: &Path, dist_root: &Path, tag: &str) -> Result<()> {
    let root_package_dir = prepare_root_package(package_root, dist_root)?;
    for package_dir in collect_release_package_dirs(dist_root)?
        .into_iter()
        .filter(|dir| dir != &root_package_dir)
    {
        run(&package_dir, tag)?;
    }
    run(&root_package_dir, tag)?;
    Ok(())
}

pub fn prepare_root_package(package_root: &Path, dist_root: &Path) -> Result<PathBuf> {
    let package_json = package_root.join("package.json");
    let package_text = fs::read_to_string(&package_json)
        .with_context(|| format!("read {}", package_json.display()))?;
    let package: Value = serde_json::from_str(&package_text)
        .with_context(|| format!("parse {}", package_json.display()))?;
    let (package_name, _) = package_identity(&package)?;
    let license = package
        .get("license")
        .and_then(Value::as_str)
        .context("package.json missing string license")?;

    let releases = collect_release_package_dirs(dist_root)?;
    let mut binaries = serde_json::Map::new();
    let mut version: Option<String> = None;
    for release_dir in releases.iter() {
        let release_json = fs::read_to_string(release_dir.join("package.json"))
            .with_context(|| format!("read {}", release_dir.join("package.json").display()))?;
        let release_package: Value = serde_json::from_str(&release_json)
            .with_context(|| format!("parse {}", release_dir.join("package.json").display()))?;
        let (name, release_version) = package_identity(&release_package)?;
        binaries.insert(name.to_string(), Value::String(release_version.to_string()));
        match &version {
            Some(existing) => ensure!(
                existing == release_version,
                "release package versions differ: {existing} != {release_version}"
            ),
            None => version = Some(release_version.to_string()),
        }
    }
    let version = version.context("no release packages found in dist")?;

    let root_dist = dist_root.join(package_name);
    if root_dist.exists() {
        fs::remove_dir_all(&root_dist)
            .with_context(|| format!("remove {}", root_dist.display()))?;
    }
    fs::create_dir_all(root_dist.join("bin"))
        .with_context(|| format!("create {}", root_dist.join("bin").display()))?;
    copy_path(&package_root.join("bin"), &root_dist.join("bin"))?;
    copy_path(
        &package_root.join("script/postinstall.mjs"),
        &root_dist.join("postinstall.mjs"),
    )?;
    copy_path(
        &package_root.join("script/jnoccio-install-bundle.mjs"),
        &root_dist.join("jnoccio-install-bundle.mjs"),
    )?;
    copy_path(&package_root.join("script/seed"), &root_dist.join("seed"))?;
    let repo_root = package_root
        .parent()
        .and_then(|parent| parent.parent())
        .context("resolve repo root from package root")?;
    copy_path(&repo_root.join("LICENSE"), &root_dist.join("LICENSE"))?;

    let root_package = serde_json::json!({
        "name": format!("{package_name}-ai"),
        "bin": {
            package_name: format!("./bin/{package_name}"),
        },
        "scripts": {
            "postinstall": "bun ./postinstall.mjs || node ./postinstall.mjs",
        },
        "version": version,
        "license": license,
        "optionalDependencies": binaries,
    });
    fs::write(
        root_dist.join("package.json"),
        serde_json::to_string_pretty(&root_package).context("serialise root package.json")?,
    )
    .with_context(|| format!("write {}", root_dist.join("package.json").display()))?;
    Ok(root_dist)
}

pub fn collect_release_package_dirs(dist_root: &Path) -> Result<Vec<PathBuf>> {
    let mut releases = Vec::new();
    for entry in fs::read_dir(dist_root).with_context(|| format!("read {}", dist_root.display()))? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let package_json = entry.path().join("package.json");
        if package_json.exists() {
            releases.push(entry.path());
        }
    }
    releases.sort();
    Ok(releases)
}

fn package_identity(package: &Value) -> Result<(&str, &str)> {
    let name = package
        .get("name")
        .and_then(Value::as_str)
        .context("package.json missing string name")?;
    let version = package
        .get("version")
        .and_then(Value::as_str)
        .context("package.json missing string version")?;
    Ok((name, version))
}

fn published(name: &str, version: &str) -> Result<bool> {
    let status = Command::new("npm")
        .args(["view", &format!("{name}@{version}"), "version"])
        .status()
        .with_context(|| format!("run npm view {name}@{version} version"))?;
    Ok(status.success())
}

fn package_tarball(package_dir: &Path) -> Result<String> {
    let output = Command::new("npm")
        .args(["pack", "--json"])
        .current_dir(package_dir)
        .output()
        .with_context(|| format!("run npm pack in {}", package_dir.display()))?;
    if !output.status.success() {
        bail!("npm pack failed with status {}", output.status);
    }

    let json: Value =
        serde_json::from_slice(&output.stdout).context("parse npm pack --json output")?;
    let tarball = json
        .as_array()
        .and_then(|items| items.first())
        .and_then(|item| item.get("filename"))
        .and_then(Value::as_str)
        .context("npm pack --json output missing filename")?;
    Ok(tarball.to_string())
}

fn run_checked(cmd: &mut Command, label: &str) -> Result<()> {
    let status = cmd.status().with_context(|| format!("run {label}"))?;
    if !status.success() {
        bail!("{label} failed with status {status}");
    }
    Ok(())
}

fn copy_path(src: &Path, dst: &Path) -> Result<()> {
    if src.is_dir() {
        copy_dir_all(src, dst)
    } else {
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
        }
        fs::copy(src, dst)
            .with_context(|| format!("copy {} → {}", src.display(), dst.display()))?;
        Ok(())
    }
}

fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst).with_context(|| format!("create {}", dst.display()))?;
    for entry in fs::read_dir(src).with_context(|| format!("read {}", src.display()))? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)
                .with_context(|| format!("copy {} → {}", src_path.display(), dst_path.display()))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn package_identity_reads_required_fields() {
        let package = json!({"name":"demo","version":"1.2.3"});
        let (name, version) = package_identity(&package).unwrap();
        assert_eq!(name, "demo");
        assert_eq!(version, "1.2.3");
    }

    #[test]
    fn package_identity_rejects_missing_fields() {
        let package = json!({"name":"demo"});
        assert!(package_identity(&package).is_err());
    }

    #[test]
    fn collect_release_package_dirs_ignores_non_packages() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("a")).unwrap();
        fs::write(dir.path().join("a/package.json"), "{}").unwrap();
        fs::create_dir_all(dir.path().join("b")).unwrap();
        fs::write(dir.path().join("b/package.json"), "{}").unwrap();
        fs::create_dir_all(dir.path().join("ignored")).unwrap();
        let dirs = collect_release_package_dirs(dir.path()).unwrap();
        assert_eq!(dirs, vec![dir.path().join("a"), dir.path().join("b")]);
    }

    #[test]
    fn prepare_root_package_writes_manifest_and_copies_assets() {
        let dir = tempfile::tempdir().unwrap();
        let package_root = dir.path().join("packages/jekko");
        let dist_root = package_root.join("dist");
        fs::create_dir_all(package_root.join("bin")).unwrap();
        fs::create_dir_all(package_root.join("script/seed")).unwrap();
        fs::write(package_root.join("script/postinstall.mjs"), "postinstall").unwrap();
        fs::write(
            package_root.join("script/jnoccio-install-bundle.mjs"),
            "bundle",
        )
        .unwrap();
        fs::write(package_root.join("bin/jekko"), "binary").unwrap();
        fs::write(
            package_root.join("package.json"),
            r#"{"name":"jekko","version":"1.2.3","license":"MIT"}"#,
        )
        .unwrap();
        fs::write(dir.path().join("LICENSE"), "license").unwrap();
        fs::create_dir_all(dist_root.join("release-a")).unwrap();
        fs::write(
            dist_root.join("release-a/package.json"),
            r#"{"name":"release-a","version":"1.2.3"}"#,
        )
        .unwrap();
        let root = prepare_root_package(&package_root, &dist_root).unwrap();
        let manifest = fs::read_to_string(root.join("package.json")).unwrap();
        assert!(manifest.contains("\"name\": \"jekko-ai\""));
        assert!(manifest.contains("\"version\": \"1.2.3\""));
        assert!(root.join("bin/jekko").exists());
        assert!(root.join("LICENSE").exists());
        assert!(root.join("seed").exists());
    }
}
