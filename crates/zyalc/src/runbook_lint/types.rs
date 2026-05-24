use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};
use serde::Serialize;

/// One lint finding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LintFinding {
    pub path: PathBuf,
    pub code: String,
    pub severity: String,
    pub message: String,
}

/// Lint report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LintReport {
    pub schema_version: String,
    pub checked: Vec<PathBuf>,
    pub findings: Vec<LintFinding>,
}

impl LintReport {
    /// Return an error if any findings are present.
    pub fn error_if_findings(&self) -> Result<()> {
        if self.findings.is_empty() {
            Ok(())
        } else {
            Err(anyhow!(
                "strict superreasoning lint found {} issue(s)",
                self.findings.len()
            ))
        }
    }
}

pub(super) fn finding(findings: &mut Vec<LintFinding>, path: &Path, code: &str, message: &str) {
    findings.push(LintFinding {
        path: path.to_path_buf(),
        code: code.to_string(),
        severity: "error".to_string(),
        message: message.to_string(),
    });
}
