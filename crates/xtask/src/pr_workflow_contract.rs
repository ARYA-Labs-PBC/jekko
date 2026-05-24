use anyhow::{Context, Result};
use serde_yaml::Value;
use std::fs;

use crate::shared::repo_root;

mod assertions;
mod job;
mod validate;

#[cfg(test)]
mod tests;

pub(super) const WORKFLOW_REL: &str = ".github/workflows/pr-standards.yml";
pub(super) const EXPECTED_WORKFLOW_NAME: &str = "pr-standards";
pub(super) const EXPECTED_TRIGGER_TYPES: &[&str] = &["opened", "edited", "synchronize"];
pub(super) const CHECKOUT_USE: &str = "actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683";
pub(super) const TOOLCHAIN_USE: &str =
    "dtolnay/rust-toolchain@29eef336d9b2848a0b548edc03f92a220660cdb8";

pub fn run() -> Result<()> {
    let root = repo_root()?;
    let path = root.join(WORKFLOW_REL);
    let text = fs::read_to_string(&path)
        .with_context(|| format!("read workflow contract {}", path.display()))?;
    let workflow: Value =
        serde_yaml::from_str(&text).with_context(|| format!("parse {}", path.display()))?;
    validate::validate_workflow(&workflow)?;
    println!("pr-workflow-contract: checked {}", path.display());
    Ok(())
}
