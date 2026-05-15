use crate::corpus::real_papers::model::{ModelDecision, RouteMetadata};
use crate::qbank_hash::sha256_hex;

pub(super) fn validate_route(label: &str, route: &RouteMetadata, errors: &mut Vec<String>) {
    if route.request_id.trim().is_empty() {
        errors.push(format!("{label} missing request_id"));
    }
    if looks_synthetic_request_id(&route.request_id) {
        errors.push(format!("{label} request_id looks synthetic"));
    }
    if route.provider.trim().is_empty() || route.model.trim().is_empty() {
        errors.push(format!("{label} missing provider/model"));
    }
    if route.route_mode.as_deref().unwrap_or("").trim().is_empty() {
        errors.push(format!("{label} missing route_mode"));
    }
    if route
        .route_confidence
        .map(|value| !(0.0..=1.0).contains(&value))
        .unwrap_or(true)
    {
        errors.push(format!("{label} missing route_confidence"));
    }
    if route
        .primary_model_id
        .as_deref()
        .unwrap_or("")
        .trim()
        .is_empty()
    {
        errors.push(format!("{label} missing primary_model_id"));
    }
    if route.backup_model_ids.is_empty() {
        errors.push(format!("{label} missing backup_model_ids"));
    }
    if route.route_mode.as_deref() == Some("fusion")
        && route
            .fusion_model_id
            .as_deref()
            .unwrap_or("")
            .trim()
            .is_empty()
    {
        errors.push(format!("{label} fusion route missing fusion_model_id"));
    }
    if route
        .winner_model_id
        .as_deref()
        .unwrap_or("")
        .trim()
        .is_empty()
    {
        errors.push(format!("{label} missing winner_model_id"));
    }
    if route.prompt_hash.as_deref().unwrap_or("").trim().is_empty() {
        errors.push(format!("{label} missing prompt_hash"));
    }
    if route
        .context_hash
        .as_deref()
        .unwrap_or("")
        .trim()
        .is_empty()
    {
        errors.push(format!("{label} missing context_hash"));
    }
    if route
        .receipts_hash
        .as_deref()
        .unwrap_or("")
        .trim()
        .is_empty()
    {
        errors.push(format!("{label} missing receipts_hash"));
    }
    if route
        .model_decisions_hash
        .as_deref()
        .unwrap_or("")
        .trim()
        .is_empty()
    {
        errors.push(format!("{label} missing model_decisions_hash"));
    }
    if route.model_decisions.is_empty() {
        errors.push(format!("{label} missing model_decisions"));
    } else {
        validate_model_decisions(
            label,
            &route.model_decisions,
            route.model_decisions_hash.as_deref(),
            errors,
        );
    }
    match route.token_usage.as_ref() {
        Some(usage) => validate_token_usage(
            &format!("{label}.token_usage"),
            usage.prompt_tokens,
            usage.completion_tokens,
            usage.total_tokens,
            errors,
        ),
        None => errors.push(format!("{label} missing token_usage")),
    }
}

pub(super) fn validate_token_usage(
    label: &str,
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
    errors: &mut Vec<String>,
) {
    if prompt_tokens == 0 || completion_tokens == 0 || total_tokens == 0 {
        errors.push(format!("{label} missing token usage"));
    }
    if total_tokens != prompt_tokens + completion_tokens {
        errors.push(format!(
            "{label} total_tokens does not match prompt + completion"
        ));
    }
}

pub(super) fn validate_model_decisions(
    label: &str,
    decisions: &[ModelDecision],
    expected_hash: Option<&str>,
    errors: &mut Vec<String>,
) {
    let mut selected = 0usize;
    for (index, decision) in decisions.iter().enumerate() {
        if decision.model_id.trim().is_empty() {
            errors.push(format!("{label}.model_decisions[{index}] missing model_id"));
        }
        if !(0.0..=1.0).contains(&decision.configured_score) {
            errors.push(format!(
                "{label}.model_decisions[{index}] invalid configured_score"
            ));
        }
        if !(0.0..=1.0).contains(&decision.selection_score) {
            errors.push(format!(
                "{label}.model_decisions[{index}] invalid selection_score"
            ));
        }
        if decision.status.trim().is_empty() {
            errors.push(format!("{label}.model_decisions[{index}] missing status"));
        }
        if decision
            .output_hash
            .as_deref()
            .unwrap_or("")
            .trim()
            .is_empty()
        {
            errors.push(format!(
                "{label}.model_decisions[{index}] missing output_hash"
            ));
        }
        if decision.token_usage.prompt_tokens == 0
            || decision.token_usage.completion_tokens == 0
            || decision.token_usage.total_tokens == 0
        {
            errors.push(format!(
                "{label}.model_decisions[{index}] missing token usage"
            ));
        }
        if decision.token_usage.total_tokens
            != decision.token_usage.prompt_tokens + decision.token_usage.completion_tokens
        {
            errors.push(format!(
                "{label}.model_decisions[{index}] token usage total mismatch"
            ));
        }
        if decision.latency_ms == 0 {
            errors.push(format!("{label}.model_decisions[{index}] missing latency"));
        }
        if decision.selected {
            selected += 1;
        }
    }
    if selected == 0 {
        errors.push(format!("{label} has no selected model decision"));
    }
    if selected > 1 {
        errors.push(format!("{label} has multiple selected model decisions"));
    }
    if let Some(expected_hash) = expected_hash {
        match serde_json::to_vec(decisions) {
            Ok(json) => {
                let computed = sha256_hex(&json);
                if computed != expected_hash {
                    errors.push(format!("{label} model_decisions_hash mismatch"));
                }
            }
            Err(err) => errors.push(format!(
                "{label} failed to serialize model_decisions: {err}"
            )),
        }
    }
}

pub(super) fn looks_synthetic_request_id(request_id: &str) -> bool {
    let lower = request_id.trim().to_ascii_lowercase();
    lower.is_empty()
        || lower.starts_with("request-")
        || lower.starts_with("fixture-")
        || lower.starts_with("mock")
        || lower.starts_with("deterministic-")
        || lower.starts_with("seed-")
        || lower.starts_with("test-")
}
