use std::collections::BTreeSet;

use crate::RecallResult;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ProvenanceFinding {
    pub missing_required: usize,
    pub irrelevant_used: usize,
    pub citation_issues: usize,
}

pub fn check(result: &RecallResult, required: &[String], allowed: &[String]) -> ProvenanceFinding {
    let used: BTreeSet<&str> = result.used_ids.iter().map(String::as_str).collect();
    let allowed: BTreeSet<&str> = allowed.iter().map(String::as_str).collect();
    let missing_required = required
        .iter()
        .filter(|id| !used.contains(id.as_str()))
        .count();
    let irrelevant_used = used
        .iter()
        .filter(|id| !allowed.is_empty() && !allowed.contains(**id))
        .count();
    let citation_issues = result
        .citations
        .iter()
        .filter(|citation| citation.source_uri.trim().is_empty())
        .count();
    ProvenanceFinding {
        missing_required,
        irrelevant_used,
        citation_issues,
    }
}
