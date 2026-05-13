//! Parses `agent/repo-score.json` into a flat `Vec<Finding>`. The runner uses
//! the classification to:
//!   1. Build the path-overlap DAG (`dag::build`).
//!   2. Route caps + high/critical findings to the incubator lane.
//!   3. Pack independent findings into parallel waves.
//!
//! The repo-score schema is owned by the jankurai CLI; we mirror only the
//! fields needed for routing. Unknown keys are ignored on purpose so a newer
//! jankurai release stays backward-compatible at runtime.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Severity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

impl Severity {
    fn parse(raw: &str) -> Severity {
        match raw.to_ascii_lowercase().as_str() {
            "critical" => Severity::Critical,
            "high" => Severity::High,
            "medium" | "med" => Severity::Medium,
            "low" => Severity::Low,
            _ => Severity::Info,
        }
    }

    /// Severities that the jankurai gate fails on by default.
    pub fn is_hard(self) -> bool {
        matches!(self, Severity::Critical | Severity::High)
    }
}

#[derive(Debug, Clone)]
pub struct Finding {
    /// Rule id from the audit, e.g. `HLT-001-DEAD-MARKER`.
    pub rule_id: String,
    /// Stable fingerprint so the runner can dedupe across iterations.
    pub fingerprint: String,
    pub severity: Severity,
    /// Files this finding touches. Used for path-overlap edges.
    pub paths: Vec<String>,
    /// `Some(cap_id)` when this finding is the consequence of a cap rather
    /// than a per-file rule. Caps short-circuit to the incubator lane.
    pub cap: Option<String>,
}

impl Finding {
    pub fn is_cap(&self) -> bool {
        self.cap.is_some()
    }
}

#[derive(Debug, Clone)]
pub struct ClassifyResult {
    pub findings: Vec<Finding>,
    pub caps_total: usize,
    pub hard_total: usize,
    pub soft_total: usize,
    pub score: f64,
}

pub fn classify(repo_root: &Path) -> Result<ClassifyResult> {
    let path = repo_root.join("agent/repo-score.json");
    let text = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    classify_text(&text)
}

pub fn classify_text(text: &str) -> Result<ClassifyResult> {
    let parsed: RepoScore = serde_json::from_str(text).context("parse agent/repo-score.json")?;

    let raw_findings: Vec<RawFinding> = match parsed.findings {
        Some(list) => list,
        None => Vec::new(),
    };

    let mut findings: Vec<Finding> = raw_findings
        .into_iter()
        .map(|f| {
            let paths = collect_paths(&f);
            let severity = match f.severity.as_deref() {
                Some(s) => Severity::parse(s),
                None => Severity::Info,
            };
            Finding {
                rule_id: f.rule_id.unwrap_or(String::new()),
                fingerprint: f.fingerprint.unwrap_or(String::new()),
                severity,
                paths,
                cap: None,
            }
        })
        .collect();

    // Caps live in a sibling array; each cap becomes a synthetic Finding so
    // the dispatcher routes it through the same lanes as a rule-finding.
    if let Some(caps) = parsed.caps_applied {
        for cap in caps {
            let cap_id_label = match cap.id.as_deref() {
                Some(id) => id.to_string(),
                None => "unknown".to_string(),
            };
            let affects = match cap.affects {
                Some(list) => list,
                None => Vec::new(),
            };
            let cap_id = match cap.id {
                Some(id) => id,
                None => String::new(),
            };
            findings.push(Finding {
                rule_id: format!("cap:{}", cap_id_label),
                fingerprint: format!("cap:{}", cap_id_label),
                severity: Severity::Critical,
                paths: affects,
                cap: Some(cap_id),
            });
        }
    }

    let caps_total = findings.iter().filter(|f| f.is_cap()).count();
    let hard_total = findings.iter().filter(|f| f.severity.is_hard() && !f.is_cap()).count();
    let soft_total = findings.len().saturating_sub(caps_total + hard_total);

    let score = match parsed.score {
        Some(n) => n,
        None => 0.0,
    };

    Ok(ClassifyResult {
        findings,
        caps_total,
        hard_total,
        soft_total,
        score,
    })
}

fn collect_paths(raw: &RawFinding) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(p) = &raw.path {
        out.push(p.clone());
    }
    if let Some(p) = &raw.file {
        out.push(p.clone());
    }
    if let Some(list) = &raw.paths {
        out.extend(list.iter().cloned());
    }
    if let Some(list) = &raw.affected_files {
        out.extend(list.iter().cloned());
    }
    out.sort();
    out.dedup();
    out
}

#[derive(Debug, Deserialize)]
struct RepoScore {
    #[serde(default)]
    score: Option<f64>,
    #[serde(default)]
    findings: Option<Vec<RawFinding>>,
    #[serde(default)]
    caps_applied: Option<Vec<RawCap>>,
}

#[derive(Debug, Deserialize)]
struct RawFinding {
    #[serde(default, alias = "id", alias = "rule")]
    rule_id: Option<String>,
    #[serde(default)]
    fingerprint: Option<String>,
    #[serde(default)]
    severity: Option<String>,
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    file: Option<String>,
    #[serde(default)]
    paths: Option<Vec<String>>,
    #[serde(default)]
    affected_files: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct RawCap {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    affects: Option<Vec<String>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_empty_findings() {
        let json = r#"{"score": 95.0, "findings": []}"#;
        let result = classify_text(json).expect("parse");
        assert!(result.findings.is_empty());
        assert_eq!(result.caps_total, 0);
        assert_eq!(result.hard_total, 0);
        assert_eq!(result.soft_total, 0);
        assert!((result.score - 95.0).abs() < f64::EPSILON);
    }

    #[test]
    fn parses_mixed_severities_into_hard_soft_totals() {
        let json = r#"{
            "score": 60.0,
            "findings": [
                {"rule_id": "HLT-001", "fingerprint": "fp1", "severity": "critical", "path": "src/a.rs"},
                {"rule_id": "HLT-002", "fingerprint": "fp2", "severity": "high",     "path": "src/b.rs"},
                {"rule_id": "HLT-003", "fingerprint": "fp3", "severity": "medium",   "path": "src/c.rs"},
                {"rule_id": "HLT-004", "fingerprint": "fp4", "severity": "low",      "path": "src/d.rs"}
            ]
        }"#;
        let result = classify_text(json).expect("parse");
        assert_eq!(result.hard_total, 2);
        assert_eq!(result.soft_total, 2);
        assert_eq!(result.caps_total, 0);
    }

    #[test]
    fn caps_become_synthetic_critical_findings() {
        let json = r#"{
            "findings": [],
            "caps_applied": [
                {"id": "no-security-lane-on-high-risk-repo", "affects": ["agent/proof-lanes.toml"]}
            ]
        }"#;
        let result = classify_text(json).expect("parse");
        assert_eq!(result.caps_total, 1);
        assert_eq!(result.hard_total, 0);
        let cap = result.findings.iter().find(|f| f.is_cap()).expect("cap finding");
        assert_eq!(cap.severity, Severity::Critical);
        assert_eq!(cap.paths, vec!["agent/proof-lanes.toml"]);
    }

    #[test]
    fn collects_paths_from_multiple_fields() {
        let json = r#"{
            "findings": [
                {"rule_id": "X", "severity": "low", "paths": ["a", "b"], "affected_files": ["b", "c"]}
            ]
        }"#;
        let result = classify_text(json).expect("parse");
        assert_eq!(result.findings[0].paths, vec!["a", "b", "c"]);
    }

    #[test]
    fn severity_parser_is_case_insensitive() {
        assert_eq!(Severity::parse("CRITICAL"), Severity::Critical);
        assert_eq!(Severity::parse("High"), Severity::High);
        assert_eq!(Severity::parse("nonsense"), Severity::Info);
    }
}
