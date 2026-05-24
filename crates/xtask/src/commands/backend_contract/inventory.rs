use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::Command;

use anyhow::{bail, Context, Result};

pub(super) fn discover_route_groups(routes_root: &Path) -> Result<Vec<String>> {
    let mut groups = Vec::new();
    if !routes_root.exists() {
        return Ok(groups);
    }

    for entry in
        fs::read_dir(routes_root).with_context(|| format!("read {}", routes_root.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                groups.push(name.to_string());
            }
            continue;
        }
        if path.extension().and_then(|s| s.to_str()) != Some("rs") {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        if stem == "mod" {
            continue;
        }
        groups.push(stem.to_string());
    }

    groups.sort();
    groups.dedup();
    Ok(groups)
}

pub(super) fn discover_cli_commands(cli_src: &Path) -> Result<Vec<String>> {
    let text = fs::read_to_string(cli_src)
        .with_context(|| format!("read CLI source {}", cli_src.display()))?;
    let Some(start) = text.find("enum Command {") else {
        bail!(
            "backend-contract: could not locate `enum Command` in {}",
            cli_src.display()
        );
    };
    let body = &text[start..];
    let mut commands = Vec::new();
    for line in body.lines().skip(1) {
        let trimmed = line.trim();
        if trimmed.starts_with('}') {
            break;
        }
        if trimmed.starts_with("#[") || trimmed.starts_with("//") || trimmed.is_empty() {
            continue;
        }
        let Some(variant) = trimmed
            .split(|c: char| c == '(' || c == ',' || c.is_whitespace())
            .next()
        else {
            continue;
        };
        if variant.is_empty()
            || !variant
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_uppercase())
        {
            continue;
        }
        commands.push(pascal_to_kebab(variant));
    }

    commands.sort();
    commands.dedup();
    Ok(commands)
}

pub(super) fn discover_openapi_methods(repo_root: &Path) -> Result<BTreeMap<String, Vec<String>>> {
    let output = Command::new("cargo")
        .args([
            "run",
            "--locked",
            "--quiet",
            "-p",
            "jekko-server",
            "--bin",
            "openapi-dump",
        ])
        .current_dir(repo_root)
        .output()
        .with_context(|| "spawn `cargo run --locked -p jekko-server --bin openapi-dump`")?;
    if !output.status.success() {
        bail!(
            "backend-contract: openapi-dump failed (exit {}): {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    let doc: serde_json::Value =
        serde_json::from_slice(&output.stdout).context("parse OpenAPI dump")?;
    let Some(paths) = doc.get("paths").and_then(|v| v.as_object()) else {
        bail!("backend-contract: OpenAPI doc has no `paths` object");
    };

    let methods = [
        "get", "post", "put", "patch", "delete", "options", "head", "trace",
    ];
    let mut out = BTreeMap::new();
    for (path, value) in paths {
        let Some(obj) = value.as_object() else {
            continue;
        };
        let mut present = Vec::new();
        for method in methods {
            if obj.contains_key(method) {
                present.push(method.to_string());
            }
        }
        present.sort();
        out.insert(path.clone(), present);
    }
    Ok(out)
}

pub(super) fn pascal_to_kebab(input: &str) -> String {
    let mut out = String::new();
    for (idx, ch) in input.chars().enumerate() {
        if ch.is_ascii_uppercase() {
            if idx != 0 {
                out.push('-');
            }
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push(ch);
        }
    }
    out
}
