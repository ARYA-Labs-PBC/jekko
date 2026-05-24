//! Strict linting for superreasoning ZYAL runbooks.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde_yaml::Value;

mod discover;
mod query;
mod strict;
mod types;

#[cfg(test)]
mod tests;

pub use types::{LintFinding, LintReport};

use discover::{discover_super_runbooks, is_superreasoning_runbook, zyal_yaml_body};
use strict::lint_strict;
use types::finding;

/// Lint every discovered superreasoning runbook.
pub fn lint_all(root: &Path, strict: bool) -> Result<LintReport> {
    let mut report = LintReport {
        schema_version: "zyal.superreasoning.lint.v1".to_string(),
        checked: Vec::new(),
        findings: Vec::new(),
    };
    for path in discover_super_runbooks(root)? {
        let child = lint_file(&path, strict)?;
        report.checked.extend(child.checked);
        report.findings.extend(child.findings);
    }
    Ok(report)
}

/// Lint one runbook.
pub fn lint_file(path: &Path, strict: bool) -> Result<LintReport> {
    let raw = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let body = zyal_yaml_body(&raw).with_context(|| format!("parse {}", path.display()))?;
    let parsed = serde_yaml::from_str::<Value>(&body);
    let yaml = parsed.as_ref().ok();
    let mut report = LintReport {
        schema_version: "zyal.superreasoning.lint.v1".to_string(),
        checked: Vec::new(),
        findings: Vec::new(),
    };
    if !is_superreasoning_runbook(&raw, yaml) {
        return Ok(report);
    }
    report.checked.push(path.to_path_buf());
    if !strict {
        return Ok(report);
    }
    if let Err(error) = parsed {
        finding(
            &mut report.findings,
            path,
            "SUPER000_PARSE",
            &format!("strict superreasoning runbooks must parse as YAML: {error}"),
        );
        return Ok(report);
    }
    lint_strict(
        path,
        yaml.expect("checked parsed above"),
        &mut report.findings,
    );
    Ok(report)
}
