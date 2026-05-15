use crate::sha256_hex;
use crate::{ModelDecision, ModelTrial, RouteMetadata};

pub(crate) fn validate_model_trial(
    field: &str,
    index: usize,
    trial: &ModelTrial,
    errors: &mut Vec<String>,
) {
    if trial.agent_id.trim().is_empty() {
        errors.push(format!("{field}[{index}] missing agent_id"));
    }
    if trial.prompt_hash.trim().is_empty() {
        errors.push(format!("{field}[{index}] missing prompt hash"));
    }
    if trial.context_hash.trim().is_empty() {
        errors.push(format!("{field}[{index}] missing context hash"));
    }
    if !(0.0..=1.0).contains(&trial.confidence) {
        errors.push(format!("{field}[{index}] confidence outside [0,1]"));
    }
    validate_route_metadata(
        &format!("{field}[{index}].route_metadata"),
        &trial.route_metadata,
        errors,
    );
    validate_token_usage(
        &format!("{field}[{index}].token_usage"),
        trial.token_usage.prompt_tokens,
        trial.token_usage.completion_tokens,
        trial.token_usage.total_tokens,
        errors,
    );
}

pub(crate) fn validate_route_metadata(
    label: &str,
    route: &RouteMetadata,
    errors: &mut Vec<String>,
) {
    if route.request_id.trim().is_empty() {
        errors.push(format!("{label} missing request_id"));
    }
    if looks_synthetic_request_id(&route.request_id) {
        errors.push(format!("{label} request_id looks synthetic"));
    }
    if route.provider.trim().is_empty() {
        errors.push(format!("{label} missing provider"));
    }
    if route.model.trim().is_empty() {
        errors.push(format!("{label} missing model"));
    }
    if route
        .route_mode
        .as_deref()
        .map(str::trim)
        .map(str::is_empty)
        .unwrap_or(false)
    {
        errors.push(format!("{label} has empty route_mode"));
    }
    if let Some(confidence) = route.route_confidence {
        if !(0.0..=1.0).contains(&confidence) {
            errors.push(format!("{label} route_confidence outside [0,1]"));
        }
    } else {
        errors.push(format!("{label} missing route_confidence"));
    }
    if route.primary_model_id.is_none() {
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
    if route.winner_model_id.is_none() {
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
    if route.token_usage.is_none() {
        errors.push(format!("{label} missing token_usage"));
    } else if let Some(usage) = route.token_usage.as_ref() {
        validate_token_usage(
            &format!("{label}.token_usage"),
            usage.prompt_tokens,
            usage.completion_tokens,
            usage.total_tokens,
            errors,
        );
    }
}

pub(crate) fn validate_token_usage(
    label: &str,
    prompt_tokens: u64,
    completion_tokens: u64,
    total_tokens: u64,
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

pub(crate) fn validate_model_decisions(
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
        if !decision.configured_score.is_finite() || decision.configured_score < 0.0 {
            errors.push(format!(
                "{label}.model_decisions[{index}] invalid configured_score"
            ));
        }
        if !decision.selection_score.is_finite() || decision.selection_score < 0.0 {
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
        let computed = match serde_json::to_vec(decisions) {
            Ok(json) => sha256_hex(&json),
            Err(err) => {
                errors.push(format!(
                    "{label} failed to serialize model_decisions: {err}"
                ));
                return;
            }
        };
        if computed != expected_hash {
            errors.push(format!("{label} model_decisions_hash mismatch"));
        }
    }
}

pub(crate) fn looks_synthetic_request_id(request_id: &str) -> bool {
    let request_id = request_id.trim();
    if request_id.is_empty() {
        return true;
    }
    let lower = request_id.to_ascii_lowercase();
    lower.starts_with("request-")
        || lower.starts_with("fixture-")
        || lower.starts_with("mock")
        || lower.starts_with("deterministic-")
        || lower.starts_with("seed-")
        || lower.starts_with("test-")
}
