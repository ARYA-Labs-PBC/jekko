//! `xtask backend-contract` — regression gate for the backend boundary.
//!
//! The gate compares the current Rust backend surface against a fixture
//! captured at the local Rust port boundary (`60521fbf3`). It is intentionally
//! conservative: if the current tree loses any expected route groups, OpenAPI
//! paths/methods, tool ids, or backend CLI commands, the command fails.

use std::path::Path;

use anyhow::Result;

use super::tool_schema_parity::discover_tool_ids;

mod checks;
mod fixture;
mod inventory;

#[cfg(test)]
mod tests;

const FIXTURE_REL: &str = "crates/xtask/fixtures/backend-contract/local-rust-port-60521fbf3.json";
const ROUTES_ROOT: &str = "crates/jekko-server/src/routes";
const CLI_SRC: &str = "crates/jekko-cli/src/cli.rs";
const TOOL_ROOT: &str = "crates/jekko-runtime/src/tool";

pub fn run(repo_root: &Path, strict: bool) -> Result<()> {
    let fixture = fixture::load_fixture(repo_root)?;
    let current_routes = inventory::discover_route_groups(&repo_root.join(ROUTES_ROOT))?;
    let current_openapi = inventory::discover_openapi_methods(repo_root)?;
    let current_tools = discover_tool_ids(&repo_root.join(TOOL_ROOT))?;
    let current_cli = inventory::discover_cli_commands(&repo_root.join(CLI_SRC))?;
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

    checks::check_subset(
        "route groups",
        &current_routes,
        &fixture.route_groups,
        strict,
    )?;
    checks::check_openapi(&current_openapi, &fixture.openapi, strict)?;
    checks::check_subset(
        "tool ids",
        &current_tools.keys().cloned().collect::<Vec<_>>(),
        &fixture.tool_ids,
        strict,
    )?;
    checks::check_subset("CLI commands", &current_cli, &fixture.cli_commands, strict)?;
    checks::check_migration_count(current_migrations, fixture.migration_count, strict)?;

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
