use crate::RecallResult;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Leak {
    pub channel: &'static str,
    pub needle: String,
}

pub fn scan_recall(result: &RecallResult, forbidden: &[String]) -> Vec<Leak> {
    let mut leaks = Vec::new();
    scan_text("answer", &result.answer, forbidden, &mut leaks);
    for citation in &result.citations {
        scan_text("citations", &citation.citation, forbidden, &mut leaks);
        scan_text("citation_uri", &citation.source_uri, forbidden, &mut leaks);
        if let Some(quote) = &citation.quote {
            scan_text("quote", quote, forbidden, &mut leaks);
        }
    }
    for id in &result.used_ids {
        scan_text("used_ids", id, forbidden, &mut leaks);
    }
    for id in &result.excluded_ids {
        scan_text("excluded_ids", id, forbidden, &mut leaks);
    }
    for redaction in &result.redactions {
        scan_text("redactions", &redaction.reason, forbidden, &mut leaks);
    }
    for call in &result.skill_calls {
        scan_text("skill_calls", &call.args_hash, forbidden, &mut leaks);
    }
    for omission in &result.omitted {
        scan_text("omitted", &omission.reason, forbidden, &mut leaks);
    }
    leaks
}

fn scan_text(channel: &'static str, text: &str, forbidden: &[String], leaks: &mut Vec<Leak>) {
    for raw in forbidden {
        for needle in variants(raw) {
            if !needle.is_empty() && text.contains(&needle) {
                leaks.push(Leak { channel, needle });
            }
        }
    }
}

fn variants(raw: &str) -> Vec<String> {
    let mut out = vec![raw.to_string(), raw.chars().rev().collect::<String>()];
    if raw.len() >= 8 {
        out.push(raw[..4].to_string());
        out.push(raw[raw.len() - 4..].to_string());
    }
    out
}
