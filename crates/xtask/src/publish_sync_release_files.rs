use anyhow::{Context, Result};
use regex::Regex;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

pub fn run(repo_root: &Path, version: &str) -> Result<()> {
    let mut changed = Vec::new();

    for path in collect_package_jsons(repo_root)? {
        let original = fs::read_to_string(&path)
            .with_context(|| format!("read package manifest {}", path.display()))?;
        let updated = rewrite_package_json(&original, version)?;
        if updated != original {
            fs::write(&path, updated)
                .with_context(|| format!("write package manifest {}", path.display()))?;
            changed.push(path);
        }
    }

    let extension_toml = repo_root.join("packages/extensions/zed/extension.toml");
    if extension_toml.exists() {
        let original = fs::read_to_string(&extension_toml)
            .with_context(|| format!("read {}", extension_toml.display()))?;
        let updated = rewrite_extension_toml(&original, version)?;
        if updated != original {
            fs::write(&extension_toml, updated)
                .with_context(|| format!("write {}", extension_toml.display()))?;
            changed.push(extension_toml);
        }
    }

    println!(
        "publish-sync-release-files: updated {} file(s)",
        changed.len()
    );
    for path in changed {
        println!("publish-sync-release-files: updated {}", path.display());
    }

    Ok(())
}

fn collect_package_jsons(repo_root: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    walk_package_jsons(repo_root, &mut out)?;
    out.sort();
    Ok(out)
}

fn walk_package_jsons(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    let mut entries = fs::read_dir(dir)
        .with_context(|| format!("read directory {}", dir.display()))?
        .collect::<Result<Vec<_>, _>>()
        .with_context(|| format!("collect directory entries for {}", dir.display()))?;
    entries.sort_by_key(|entry| entry.file_name());

    for entry in entries {
        let path = entry.path();
        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();
        if entry.file_type()?.is_dir() {
            if matches!(
                file_name.as_ref(),
                ".git" | "dist" | "node_modules" | "target"
            ) {
                continue;
            }
            walk_package_jsons(&path, out)?;
            continue;
        }

        if file_name.as_ref() == "package.json" {
            out.push(path);
        }
    }

    Ok(())
}

fn rewrite_package_json(original: &str, version: &str) -> Result<String> {
    let mut value: Value = serde_json::from_str(original).context("parse package.json")?;
    if let Some(map) = value.as_object_mut() {
        if matches!(map.get("version"), Some(Value::String(_))) {
            map.insert("version".to_string(), Value::String(version.to_string()));
        }
    }
    serde_json::to_string_pretty(&value).context("serialise package.json")
}

fn rewrite_extension_toml(original: &str, version: &str) -> Result<String> {
    let version_re = Regex::new(r#"(?m)^version = "[^"]+""#).expect("valid version regex");
    let updated = if version_re.is_match(original) {
        version_re
            .replace(original, format!("version = \"{version}\""))
            .into_owned()
    } else {
        original.to_string()
    };

    let release_re = Regex::new(r#"releases/download/v[^/]+/"#).expect("valid release url regex");
    Ok(release_re
        .replace_all(&updated, format!("releases/download/v{version}/"))
        .into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rewrite_package_json_updates_version_only() {
        let original = r#"{ "name": "demo", "version": "1.2.3", "private": true }"#;
        let updated = rewrite_package_json(original, "9.9.9").unwrap();
        let parsed: Value = serde_json::from_str(&updated).unwrap();
        assert_eq!(parsed["version"], "9.9.9");
        assert_eq!(parsed["name"], "demo");
        assert_eq!(parsed["private"], true);
    }

    #[test]
    fn rewrite_extension_toml_updates_version_and_release_urls() {
        let original = r#"
version = "1.2.3"
download = "https://github.com/neverhuman/jekko/releases/download/v1.2.3/jekko.zip"
"#;
        let updated = rewrite_extension_toml(original, "9.9.9").unwrap();
        assert!(updated.contains("version = \"9.9.9\""));
        assert!(updated.contains("releases/download/v9.9.9/jekko.zip"));
    }

    #[test]
    fn collect_package_jsons_skips_dist_and_node_modules() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("a/node_modules/x")).unwrap();
        fs::create_dir_all(dir.path().join("b/dist")).unwrap();
        fs::create_dir_all(dir.path().join("c/nested")).unwrap();
        fs::write(dir.path().join("package.json"), "{}").unwrap();
        fs::write(dir.path().join("a/package.json"), "{}").unwrap();
        fs::write(dir.path().join("a/node_modules/x/package.json"), "{}").unwrap();
        fs::write(dir.path().join("b/dist/package.json"), "{}").unwrap();
        fs::write(dir.path().join("c/nested/package.json"), "{}").unwrap();

        let found = collect_package_jsons(dir.path()).unwrap();
        let relative: Vec<String> = found
            .iter()
            .map(|path| path.strip_prefix(dir.path()).unwrap().display().to_string())
            .collect();

        assert_eq!(
            relative,
            vec![
                "a/package.json".to_string(),
                "c/nested/package.json".to_string(),
                "package.json".to_string(),
            ]
        );
    }

    #[test]
    fn run_updates_repo_manifests_in_place() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("packages/extensions/zed")).unwrap();
        fs::write(
            dir.path().join("package.json"),
            r#"{ "name": "demo", "version": "1.2.3" }"#,
        )
        .unwrap();
        fs::write(
            dir.path().join("packages/extensions/zed/extension.toml"),
            r#"
version = "1.2.3"
download = "https://github.com/neverhuman/jekko/releases/download/v1.2.3/jekko.zip"
"#,
        )
        .unwrap();

        run(dir.path(), "9.9.9").unwrap();

        let pkg: Value =
            serde_json::from_str(&fs::read_to_string(dir.path().join("package.json")).unwrap())
                .unwrap();
        assert_eq!(pkg["version"], "9.9.9");
        let ext =
            fs::read_to_string(dir.path().join("packages/extensions/zed/extension.toml")).unwrap();
        assert!(ext.contains("version = \"9.9.9\""));
        assert!(ext.contains("releases/download/v9.9.9/jekko.zip"));
    }
}
