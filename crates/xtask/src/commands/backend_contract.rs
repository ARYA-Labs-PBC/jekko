//! `xtask backend-contract` — regression gate for the backend boundary.
//!
//! The gate compares the current Rust backend surface against a fixture
//! captured at the local Rust port boundary (`60521fbf3`). It is intentionally
//! conservative: if the current tree loses any expected route groups, OpenAPI
//! paths/methods, tool ids, or backend CLI commands, the command fails.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::Command;

use anyhow::{bail, Context, Result};
use serde::Deserialize;

use super::parity_diff::SetDiff;
use super::tool_schema_parity::discover_tool_ids;

const FIXTURE_REL: &str = "crates/xtask/fixtures/backend-contract/local-rust-port-60521fbf3.json";
const ROUTES_ROOT: &str = "crates/jekko-server/src/routes";
const CLI_SRC: &str = "crates/jekko-cli/src/cli.rs";
const TOOL_ROOT: &str = "crates/jekko-runtime/src/tool";

#[derive(Debug, Clone, Deserialize)]
struct BackendContractFixture {
    #[serde(default)]
    pre_port_ts_groups: Vec<String>,
    #[serde(default)]
    rust_port_groups: Vec<String>,
    #[serde(default)]
    expected_rust_extras: Vec<String>,
    #[serde(default)]
    deferred_groups: Vec<String>,
    #[serde(default)]
    route_groups: Vec<String>,
    #[serde(default)]
    openapi: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    tool_ids: Vec<String>,
    #[serde(default)]
    cli_commands: Vec<String>,
    #[serde(default)]
    migration_count: usize,
}

pub fn run(repo_root: &Path, strict: bool) -> Result<()> {
    let fixture = load_fixture(repo_root)?;
    let current_routes = discover_route_groups(&repo_root.join(ROUTES_ROOT))?;
    let current_openapi = discover_openapi_methods(repo_root)?;
    let current_tools = discover_tool_ids(&repo_root.join(TOOL_ROOT))?;
    let current_cli = discover_cli_commands(&repo_root.join(CLI_SRC))?;
    let current_migrations = jekko_store::db::embedded_migration_count();

    println!(
        "backend-contract: fixture {}",
        repo_root.join(FIXTURE_REL).display()
    );
    println!(
        "backend-contract: historical TS groups: {} | Rust port groups: {} | deferred: {} | rust extras: {}",
        fixture.pre_port_ts_groups.len(),
        fixture.rust_port_groups.len(),
        fixture.deferred_groups.len(),
        fixture.expected_rust_extras.len()
    );
    if !fixture.deferred_groups.is_empty() {
        println!(
            "backend-contract: deferred route groups: {}",
            fixture.deferred_groups.join(", ")
        );
    }

    check_subset(
        "route groups",
        &current_routes,
        &fixture.route_groups,
        strict,
    )?;
    check_openapi(&current_openapi, &fixture.openapi, strict)?;
    check_subset(
        "tool ids",
        &current_tools.keys().cloned().collect::<Vec<_>>(),
        &fixture.tool_ids,
        strict,
    )?;
    check_subset("CLI commands", &current_cli, &fixture.cli_commands, strict)?;
    check_migration_count(current_migrations, fixture.migration_count, strict)?;

    println!(
        "backend-contract: ✓ current backend surface covers {} route group(s), {} openapi path(s), {} tool(s), {} cli command(s), {} migration(s)",
        current_routes.len(),
        current_openapi.len(),
        current_tools.len(),
        current_cli.len(),
        current_migrations
    );
    Ok(())
}

fn load_fixture(repo_root: &Path) -> Result<BackendContractFixture> {
    let path = repo_root.join(FIXTURE_REL);
    let text = fs::read_to_string(&path)
        .with_context(|| format!("read backend-contract fixture {}", path.display()))?;
    let fixture: BackendContractFixture = serde_json::from_str(&text)
        .with_context(|| format!("parse backend-contract fixture {}", path.display()))?;
    Ok(fixture)
}

fn discover_route_groups(routes_root: &Path) -> Result<Vec<String>> {
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

fn discover_cli_commands(cli_src: &Path) -> Result<Vec<String>> {
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

fn discover_openapi_methods(repo_root: &Path) -> Result<BTreeMap<String, Vec<String>>> {
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

fn check_subset(label: &str, actual: &[String], expected: &[String], strict: bool) -> Result<()> {
    let diff = SetDiff::compute(actual.to_vec(), expected.to_vec());
    if diff.removed.is_empty() {
        println!(
            "backend-contract: {} ✓ {} expected item(s) covered, {} extra current item(s)",
            label,
            expected.len(),
            diff.added.len()
        );
        return Ok(());
    }

    println!(
        "backend-contract: {} missing {} expected item(s) and has {} extra current item(s)",
        label,
        diff.removed.len(),
        diff.added.len()
    );
    for item in &diff.removed {
        println!("  - {item}");
    }
    for item in &diff.added {
        println!("  + {item}");
    }
    if strict {
        bail!("backend-contract: {label} missing expected item(s)");
    }
    Ok(())
}

fn check_openapi(
    actual: &BTreeMap<String, Vec<String>>,
    expected: &BTreeMap<String, Vec<String>>,
    strict: bool,
) -> Result<()> {
    let mut missing_paths = Vec::new();
    let mut missing_methods = Vec::new();
    for (path, methods) in expected {
        match actual.get(path) {
            Some(current) => {
                let diff = SetDiff::compute(current.clone(), methods.clone());
                if !diff.removed.is_empty() {
                    missing_methods.push((path.clone(), diff.removed));
                }
            }
            None => missing_paths.push(path.clone()),
        }
    }

    if missing_paths.is_empty() && missing_methods.is_empty() {
        println!(
            "backend-contract: OpenAPI ✓ {} expected path(s) covered, {} total current path(s)",
            expected.len(),
            actual.len()
        );
        return Ok(());
    }

    if !missing_paths.is_empty() {
        println!("backend-contract: OpenAPI missing paths:");
        for path in &missing_paths {
            println!("  - {path}");
        }
    }
    if !missing_methods.is_empty() {
        println!("backend-contract: OpenAPI missing methods:");
        for (path, methods) in &missing_methods {
            println!("  - {path}: {}", methods.join(", "));
        }
    }

    if strict {
        bail!("backend-contract: OpenAPI missing expected path(s) or method(s)");
    }
    Ok(())
}

fn check_migration_count(actual: usize, expected: usize, strict: bool) -> Result<()> {
    if actual >= expected {
        println!(
            "backend-contract: migrations ✓ current {} >= expected {}",
            actual, expected
        );
        return Ok(());
    }

    println!(
        "backend-contract: migrations missing {} embedded migration(s) (current {}, expected {})",
        expected - actual,
        actual,
        expected
    );
    if strict {
        bail!("backend-contract: migration count shrank");
    }
    Ok(())
}

fn pascal_to_kebab(input: &str) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pascal_to_kebab_handles_mixed_acronyms() {
        assert_eq!(pascal_to_kebab("McpServer"), "mcp-server");
        assert_eq!(pascal_to_kebab("Db"), "db");
        assert_eq!(pascal_to_kebab("Providers"), "providers");
    }

    #[test]
    fn discover_route_groups_ignores_mod_rs() {
        let tmp = tempfile::tempdir().unwrap();
        let routes = tmp.path().join("routes");
        fs::create_dir_all(routes.join("v2")).unwrap();
        fs::write(routes.join("mod.rs"), "// mod").unwrap();
        fs::write(routes.join("session.rs"), "// session").unwrap();
        fs::write(routes.join("v2").join("session.rs"), "// v2").unwrap();
        let groups = discover_route_groups(&routes).unwrap();
        assert_eq!(groups, vec!["session".to_string(), "v2".to_string()]);
    }

    #[test]
    fn discover_cli_commands_parses_enum_variants() {
        let tmp = tempfile::tempdir().unwrap();
        let cli = tmp.path().join("cli.rs");
        fs::write(
            &cli,
            r#"
pub enum Command {
    Run(cmd::run::RunArgs),
    #[command(name = "mcp-server")]
    McpServer(cmd::mcp_server::McpServerArgs),
    Db(cmd::db::DbArgs),
}
"#,
        )
        .unwrap();
        let commands = discover_cli_commands(&cli).unwrap();
        assert_eq!(
            commands,
            vec![
                "db".to_string(),
                "mcp-server".to_string(),
                "run".to_string(),
            ]
        );
    }
}
