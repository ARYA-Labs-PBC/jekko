use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::Deserialize;

use super::FIXTURE_REL;

#[derive(Debug, Clone, Deserialize)]
pub(super) struct BackendContractFixture {
    #[serde(default)]
    pub(super) pre_port_ts_groups: Vec<String>,
    #[serde(default)]
    pub(super) rust_port_groups: Vec<String>,
    #[serde(default)]
    pub(super) expected_rust_extras: Vec<String>,
    #[serde(default)]
    pub(super) deferred_groups: Vec<String>,
    #[serde(default)]
    pub(super) route_groups: Vec<String>,
    #[serde(default)]
    pub(super) openapi: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    pub(super) tool_ids: Vec<String>,
    #[serde(default)]
    pub(super) cli_commands: Vec<String>,
    #[serde(default)]
    pub(super) migration_count: usize,
}

pub(super) fn load_fixture(repo_root: &Path) -> Result<BackendContractFixture> {
    let path = repo_root.join(FIXTURE_REL);
    let text = fs::read_to_string(&path)
        .with_context(|| format!("read backend-contract fixture {}", path.display()))?;
    let fixture: BackendContractFixture = serde_json::from_str(&text)
        .with_context(|| format!("parse backend-contract fixture {}", path.display()))?;
    Ok(fixture)
}
