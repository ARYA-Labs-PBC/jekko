use anyhow::{bail, Context, Result};
use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::Command;

pub fn run(package_dir: &Path, tag: &str) -> Result<()> {
    let package_json = package_dir.join("package.json");
    let original = fs::read_to_string(&package_json)
        .with_context(|| format!("read {}", package_json.display()))?;
    let mut package: Value = serde_json::from_str(&original)
        .with_context(|| format!("parse {}", package_json.display()))?;
    let (name, version) = package_identity(&package)?;

    if published(name, version)? {
        println!("already published {name}@{version}");
        return Ok(());
    }

    let exports = package
        .get("exports")
        .cloned()
        .context("package.json missing exports object")?;
    package["exports"] = transform_exports(&exports);

    let updated = serde_json::to_string_pretty(&package).context("serialise package.json")?;
    fs::write(&package_json, &updated)
        .with_context(|| format!("write {}", package_json.display()))?;

    let result = publish_package(package_dir, tag);
    let restore = fs::write(&package_json, original)
        .with_context(|| format!("restore {}", package_json.display()));
    match (result, restore) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(err), Ok(())) => Err(err),
        (Ok(()), Err(err)) => Err(err),
        (Err(err), Err(restore_err)) => Err(err.context(format!(
            "also failed to restore {}: {restore_err:#}",
            package_json.display()
        ))),
    }
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

fn publish_package(package_dir: &Path, tag: &str) -> Result<()> {
    let pack_output = Command::new("npm")
        .args(["pack", "--json"])
        .current_dir(package_dir)
        .output()
        .with_context(|| format!("run npm pack in {}", package_dir.display()))?;
    if !pack_output.status.success() {
        bail!("npm pack failed with status {}", pack_output.status);
    }

    let json: Value =
        serde_json::from_slice(&pack_output.stdout).context("parse npm pack --json output")?;
    let tarball = json
        .as_array()
        .and_then(|items| items.first())
        .and_then(|item| item.get("filename"))
        .and_then(Value::as_str)
        .context("npm pack --json output missing filename")?;

    let status = Command::new("npm")
        .args(["publish", tarball, "--tag", tag, "--access", "public"])
        .current_dir(package_dir)
        .status()
        .with_context(|| format!("run npm publish for {}", package_dir.display()))?;
    if !status.success() {
        bail!("npm publish failed with status {status}");
    }
    Ok(())
}

fn transform_exports(exports: &Value) -> Value {
    match exports {
        Value::String(value) => {
            let file = value.replace("./src/", "./dist/").replace(".ts", "");
            serde_json::json!({
                "import": format!("{file}.js"),
                "types": format!("{file}.d.ts"),
            })
        }
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(key, value)| (key.clone(), transform_exports(value)))
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.iter().map(transform_exports).collect()),
        other => other.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn transform_exports_rewrites_nested_strings() {
        let input = json!({
            ".": "./src/index.ts",
            "./nested": {
                "import": "./src/nested.ts"
            },
            "./keep": true
        });
        let output = transform_exports(&input);
        assert_eq!(
            output["."],
            json!({"import":"./dist/index.js","types":"./dist/index.d.ts"})
        );
        assert_eq!(
            output["./nested"]["import"],
            json!({"import":"./dist/nested.js","types":"./dist/nested.d.ts"})
        );
        assert_eq!(output["./keep"], true);
    }

    #[test]
    fn package_identity_reads_required_fields() {
        let package = json!({"name":"demo","version":"1.2.3","exports":{}});
        let (name, version) = package_identity(&package).unwrap();
        assert_eq!(name, "demo");
        assert_eq!(version, "1.2.3");
    }
}
