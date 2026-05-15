use super::*;

pub(crate) fn first_sentence(text: &str) -> String {
    let trimmed = text.trim();
    let chars = trimmed.char_indices().collect::<Vec<_>>();
    for (position, (index, ch)) in chars.iter().enumerate() {
        if !matches!(ch, '.' | '!' | '?') {
            continue;
        }
        if *ch == '.' {
            let prev = position
                .checked_sub(1)
                .and_then(|prev| chars.get(prev))
                .map(|(_, ch)| *ch);
            let next = chars.get(position + 1).map(|(_, ch)| *ch);
            if prev.is_some_and(|ch| ch.is_ascii_digit())
                && next.is_some_and(|ch| ch.is_ascii_digit())
            {
                continue;
            }
        }
        let candidate = trimmed[..*index].trim();
        if candidate.chars().count() > 24 {
            return candidate.to_string();
        }
    }
    trimmed.to_string()
}

pub(crate) fn answer_from_quote(quote: &str) -> String {
    quote.trim().to_string()
}

pub(crate) fn select_distractors(
    paper: &PaperRecord,
    papers: &[PaperRecord],
    requested: usize,
) -> Vec<String> {
    papers
        .iter()
        .filter(|candidate| candidate.publication_hash != paper.publication_hash)
        .take(requested)
        .map(|candidate| candidate.publication_hash.clone())
        .collect()
}

pub(crate) fn select_hard_distractors(
    paper: &PaperRecord,
    papers: &[PaperRecord],
    quote: &str,
    question: &str,
    requested: usize,
) -> Vec<String> {
    let anchor = content_tokens(&format!("{question} {quote}"));
    let mut scored = papers
        .iter()
        .filter(|candidate| candidate.publication_hash != paper.publication_hash)
        .filter(|candidate| paper_quality_allowed(candidate))
        .map(|candidate| {
            let text = format!(
                "{} {}",
                candidate.title,
                canonical_paper_text(candidate, false).full_text
            );
            let overlap = token_overlap_score(&anchor, &content_tokens(&text));
            let unit_bonus = shared_unit_bonus(quote, &text);
            (overlap + unit_bonus, candidate.publication_hash.clone())
        })
        .collect::<Vec<_>>();
    scored.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
    scored
        .into_iter()
        .take(requested)
        .map(|(_, hash)| hash)
        .collect()
}

fn token_overlap_score(
    anchor: &std::collections::BTreeSet<String>,
    candidate: &std::collections::BTreeSet<String>,
) -> i32 {
    anchor
        .iter()
        .filter(|token| candidate.contains(*token))
        .count() as i32
}

fn shared_unit_bonus(quote: &str, text: &str) -> i32 {
    let quote_lower = quote.to_ascii_lowercase();
    let text_lower = text.to_ascii_lowercase();
    [
        "%", "mg", "mm", "cm", "kg", "wt", "rh", "ci", "fold", "ratio", "mean", "median",
    ]
    .iter()
    .filter(|unit| quote_lower.contains(**unit) && text_lower.contains(**unit))
    .count() as i32
}

pub(crate) fn receipt(
    phase: &str,
    index: usize,
    prompt: &str,
    paper_hash: &str,
) -> AgentCallReceipt {
    let prompt_hash = sha256_hex(prompt.as_bytes());
    let context_hash = sha256_hex(format!("{paper_hash}:{phase}:{index}").as_bytes());
    let raw_output_hash = sha256_hex(format!("{phase}:{index}:{prompt_hash}").as_bytes());
    let usage = TokenUsage {
        prompt_tokens: 1200 + index as u64,
        completion_tokens: 300 + index as u64,
        total_tokens: 1500 + (index as u64 * 2),
    };
    let decisions = vec![ModelDecision {
        model_id: format!("qbank-{phase}-primary"),
        configured_score: 0.91,
        selection_score: 0.93,
        latency_ms: 100 + index as u64,
        status: "completed".to_string(),
        output_hash: Some(raw_output_hash.clone()),
        selected: true,
        token_usage: usage.clone(),
    }];
    let decisions_hash = sha256_hex(&serde_json::to_vec(&decisions).expect("decisions serialize"));
    let route_metadata = RouteMetadata {
        request_id: format!(
            "mock_smoke_{}_{}_{}",
            phase,
            index + 1,
            &raw_output_hash[..12]
        ),
        provider: "mock-smoke".to_string(),
        model: format!("mock-qbank-{phase}-primary"),
        route_mode: Some("mock_smoke".to_string()),
        route_confidence: Some(0.93),
        primary_model_id: Some(format!("mock-qbank-{phase}-primary")),
        backup_model_ids: vec![format!("mock-qbank-{phase}-backup")],
        fusion_model_id: Some("mock-qbank-fusion-router".to_string()),
        winner_model_id: Some(format!("mock-qbank-{phase}-primary")),
        prompt_hash: Some(prompt_hash.clone()),
        context_hash: Some(context_hash.clone()),
        receipts_hash: Some(sha256_hex(
            format!("{phase}:{index}:{paper_hash}:receipt").as_bytes(),
        )),
        token_usage: Some(usage.clone()),
        model_decisions_hash: Some(decisions_hash),
        model_decisions: decisions,
    };
    AgentCallReceipt {
        agent_name: format!("{phase}-{}", index + 1),
        phase: phase.to_string(),
        prompt_hash,
        context_hash,
        raw_output_hash,
        route_metadata: Some(route_metadata),
        token_usage: Some(usage),
    }
}

pub(crate) fn failure(
    phase: &str,
    agent_name: &str,
    error: String,
    receipt: &AgentCallReceipt,
) -> AgentFailure {
    let category = failure_category(phase, &error).to_string();
    let fatal_for_acceptance = fatal_failure_category(&category);
    AgentFailure {
        category,
        phase: phase.to_string(),
        agent_name: agent_name.to_string(),
        error,
        fatal_for_acceptance,
        route_metadata: receipt.route_metadata.clone(),
        raw_output_hash: Some(receipt.raw_output_hash.clone()),
    }
}

pub(crate) fn live_call_failure(
    phase: &str,
    index: usize,
    error: JnoccioCallError,
) -> AgentFailure {
    let receipt = error.receipt.as_ref();
    let category = match error.category {
        Some(value) => value,
        None => failure_category(phase, &error.message).to_string(),
    };
    AgentFailure {
        fatal_for_acceptance: fatal_failure_category(&category),
        category,
        phase: phase.to_string(),
        agent_name: match receipt {
            Some(receipt) => receipt.agent_name.clone(),
            None => format!("{phase}-{}", index + 1),
        },
        error: error.message,
        route_metadata: receipt.and_then(|receipt| receipt.route_metadata.clone()),
        raw_output_hash: receipt.map(|receipt| receipt.raw_output_hash.clone()),
    }
}

pub(crate) fn failure_category<'a>(phase: &str, error: &'a str) -> &'a str {
    let lower = error.to_ascii_lowercase();
    if lower.contains("timeout") {
        "timeout"
    } else if lower.contains("http")
        || lower.contains("request failed")
        || lower.contains("response read")
    {
        "route_http"
    } else if lower.contains("route metadata") || lower.contains("metadata") {
        "route_metadata"
    } else if lower.contains("parse") || lower.contains("json") || lower.contains("schema") {
        "parse_schema"
    } else if lower.contains("support")
        || lower.contains("quote")
        || lower.contains("canonical full text")
    {
        "generator_support"
    } else {
        match phase {
            "generation" | "generator" => "generator_schema",
            "verification" => "verifier_reject",
            "testing" => "tester_schema",
            "grading" => "grader_schema",
            _ => "parse_schema",
        }
    }
}

pub(crate) fn fatal_failure_category(category: &str) -> bool {
    matches!(
        category,
        "source_quality" | "no_quote_candidates" | "blind_too_easy" | "blind_confidence"
    )
}

pub(crate) fn failure_route_label(route: Option<&RouteMetadata>) -> String {
    let Some(route) = route else {
        return String::new();
    };
    let usage = route.token_usage.as_ref().map(|usage| {
        format!(
            ", tokens={}/{}/{}",
            usage.prompt_tokens, usage.completion_tokens, usage.total_tokens
        )
    });
    let usage = match usage {
        Some(value) => value,
        None => String::new(),
    };
    format!(
        " [request_id={}, model={}, route_mode={}, winner={}{}]",
        route.request_id,
        route.model,
        route.route_mode.as_deref().unwrap_or(""),
        route.winner_model_id.as_deref().unwrap_or(""),
        usage
    )
}
