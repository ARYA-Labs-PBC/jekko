//! Shared parser for `agent/repo-score.json`.
//!
//! The repo-score document is produced by jankurai audits and consumed by the
//! CLI, TUI, and runner-facing status surfaces. This module keeps the parsing
//! logic in one pure place so callers do not all reimplement their own JSON
//! scraping.

use serde::Deserialize;

/// Parsed summary of a jankurai score file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JankuraiSummary {
    /// Final score after caps.
    pub score: u64,
    /// Raw score before caps.
    pub raw_score: u64,
    /// Number of caps applied.
    pub caps_count: usize,
    /// Cap identifiers or labels.
    pub caps: Vec<String>,
    /// Count of hard findings.
    pub hard_findings: u64,
    /// Count of soft findings.
    pub soft_findings: u64,
    /// Blocker labels from the audit decision.
    pub blockers: Vec<String>,
    /// Claimed conformance level.
    pub claimed_conformance_level: Option<String>,
    /// Observed conformance level.
    pub observed_conformance_level: Option<String>,
    /// Whether the worktree was dirty when the report was generated.
    pub dirty_worktree: bool,
    /// Top-level conformance decision, when present.
    pub conformance_decision: Option<String>,
    /// Number of findings in the report, when present.
    pub findings_count: usize,
}

/// Parse a `repo-score.json` payload into a compact typed summary.
pub fn parse_jankurai_score_json(text: &str) -> Result<JankuraiSummary, serde_json::Error> {
    let parsed: RepoScore = serde_json::from_str(text)?;
    Ok(JankuraiSummary {
        score: parsed.score.unwrap_or(0.0).round() as u64,
        raw_score: parsed.raw_score.unwrap_or(0.0).round() as u64,
        caps_count: parsed.caps_applied.len(),
        caps: parsed.caps_applied,
        hard_findings: parsed
            .decision
            .as_ref()
            .map(|d| d.hard_findings)
            .unwrap_or(0),
        soft_findings: parsed
            .decision
            .as_ref()
            .map(|d| d.soft_findings)
            .unwrap_or(0),
        blockers: parsed.conformance_blockers,
        claimed_conformance_level: parsed.claimed_conformance_level,
        observed_conformance_level: parsed.observed_conformance_level,
        dirty_worktree: parsed.dirty_worktree,
        conformance_decision: parsed.conformance_decision,
        findings_count: parsed.findings.len(),
    })
}

#[derive(Debug, Deserialize)]
struct RepoScore {
    #[serde(default)]
    score: Option<f64>,
    #[serde(default, alias = "raw")]
    raw_score: Option<f64>,
    #[serde(default)]
    caps_applied: Vec<String>,
    #[serde(default)]
    findings: Vec<serde_json::Value>,
    #[serde(default)]
    decision: Option<Decision>,
    #[serde(default)]
    conformance_blockers: Vec<String>,
    #[serde(default)]
    claimed_conformance_level: Option<String>,
    #[serde(default)]
    observed_conformance_level: Option<String>,
    #[serde(default)]
    dirty_worktree: bool,
    #[serde(default)]
    conformance_decision: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Decision {
    #[serde(default)]
    hard_findings: u64,
    #[serde(default)]
    soft_findings: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_summary_fields() {
        let json = r#"{
            "score": 87,
            "raw_score": 91,
            "caps_applied": ["a", "b"],
            "findings": [1, 2, 3],
            "decision": {"hard_findings": 4, "soft_findings": 1},
            "conformance_blockers": ["x"],
            "claimed_conformance_level": "HL3",
            "observed_conformance_level": "HL2",
            "dirty_worktree": true,
            "conformance_decision": "advisory"
        }"#;
        let summary = parse_jankurai_score_json(json).unwrap();
        assert_eq!(summary.score, 87);
        assert_eq!(summary.raw_score, 91);
        assert_eq!(summary.caps_count, 2);
        assert_eq!(summary.findings_count, 3);
        assert_eq!(summary.hard_findings, 4);
        assert_eq!(summary.soft_findings, 1);
        assert!(summary.dirty_worktree);
        assert_eq!(summary.claimed_conformance_level.as_deref(), Some("HL3"));
        assert_eq!(summary.observed_conformance_level.as_deref(), Some("HL2"));
    }

    #[test]
    fn tolerates_missing_optional_fields() {
        let summary = parse_jankurai_score_json("{}").unwrap();
        assert_eq!(summary.score, 0);
        assert_eq!(summary.raw_score, 0);
        assert_eq!(summary.caps_count, 0);
        assert_eq!(summary.findings_count, 0);
        assert_eq!(summary.blockers.len(), 0);
    }
}
