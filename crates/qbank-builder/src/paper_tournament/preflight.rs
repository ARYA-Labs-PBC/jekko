use super::*;

pub(crate) fn configured_jnoccio_model(config: &BuildPaperTournamentConfig) -> String {
    let configured = config
        .jnoccio_model
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    match configured {
        Some(value) => value,
        None => match std::env::var("QBANK_JNOCCIO_MODEL") {
            Ok(value) if !value.trim().is_empty() => value,
            _ => "jnoccio/jnoccio-fusion".to_string(),
        },
    }
}

pub(crate) fn model_matches_gateway_visible_model(
    visible_model: &str,
    requested_model: &str,
) -> bool {
    requested_model == visible_model
        || requested_model
            .strip_prefix("jnoccio/")
            .map(|suffix| suffix == visible_model)
            .unwrap_or(false)
        || visible_model
            .strip_prefix("jnoccio/")
            .map(|suffix| suffix == requested_model)
            .unwrap_or(false)
        || visible_model
            == requested_model
                .rsplit('/')
                .next()
                .unwrap_or(requested_model)
}

pub(crate) fn fetch_required_text(
    client: &reqwest::blocking::Client,
    url: &str,
    label: &str,
) -> Result<String, String> {
    let response = client
        .get(url)
        .send()
        .map_err(|err| format!("{label} request failed: {err}"))?;
    let status = response.status();
    let text = response
        .text()
        .map_err(|err| format!("{label} response read failed: {err}"))?;
    if !status.is_success() {
        return Err(format!("{label} returned HTTP {}: {text}", status.as_u16()));
    }
    Ok(text)
}

pub(crate) fn fetch_optional_json(
    client: &reqwest::blocking::Client,
    url: &str,
) -> Result<Option<serde_json::Value>, String> {
    let response = match client.get(url).send() {
        Ok(response) => response,
        Err(err) => return Err(format!("jnoccio metrics request failed: {err}")),
    };
    let status = response.status();
    let text = response
        .text()
        .map_err(|err| format!("jnoccio metrics response read failed: {err}"))?;
    if status.as_u16() == 404 {
        return Ok(None);
    }
    if !status.is_success() {
        return Err(format!(
            "jnoccio metrics returned HTTP {}: {text}",
            status.as_u16()
        ));
    }
    serde_json::from_str(&text)
        .map(Some)
        .map_err(|err| format!("jnoccio metrics response is not JSON: {err}"))
}

pub(crate) fn summarize_jnoccio_models(status: &serde_json::Value) -> Vec<serde_json::Value> {
    let mut models = BTreeMap::<String, serde_json::Value>::new();
    if let Some(array) = status.get("models").and_then(|value| value.as_array()) {
        for value in array {
            insert_model_summary(&mut models, value);
        }
    }
    collect_model_summaries(status, &mut models);
    models.into_values().collect()
}

pub(crate) fn collect_model_summaries(
    value: &serde_json::Value,
    models: &mut BTreeMap<String, serde_json::Value>,
) {
    match value {
        serde_json::Value::Array(items) => {
            for item in items {
                collect_model_summaries(item, models);
            }
        }
        serde_json::Value::Object(map) => {
            let has_model_identity = ["id", "model_id", "visible_id", "model", "name"]
                .iter()
                .any(|key| map.contains_key(*key));
            let has_capacity = [
                "context_window",
                "context_window_tokens",
                "max_output_tokens",
            ]
            .iter()
            .any(|key| map.contains_key(*key));
            if has_model_identity && has_capacity {
                insert_model_summary(models, value);
            }
            for child in map.values() {
                collect_model_summaries(child, models);
            }
        }
        _ => {}
    }
}

pub(crate) fn insert_model_summary(
    models: &mut BTreeMap<String, serde_json::Value>,
    value: &serde_json::Value,
) {
    let summary = compact_model_summary(value);
    if summary.as_object().map(|object| object.is_empty()) == Some(true) {
        return;
    }
    let key = ["visible_id", "id", "model_id", "name", "model"]
        .iter()
        .filter_map(|field| summary.get(*field).and_then(|value| value.as_str()))
        .next()
        .map(str::to_string);
    let key = match key {
        Some(value) => value,
        None => match serde_json::to_string(&summary) {
            Ok(value) => value,
            Err(_) => format!("model-{}", models.len() + 1),
        },
    };
    models.entry(key).or_insert(summary);
}

pub(crate) fn compact_model_summary(value: &serde_json::Value) -> serde_json::Value {
    let mut object = serde_json::Map::new();
    for key in [
        "id",
        "model_id",
        "visible_id",
        "name",
        "provider",
        "model",
        "display_name",
        "status",
        "enabled",
        "healthy",
        "keyed",
        "context_window",
        "context_window_tokens",
        "max_output_tokens",
        "max_tokens",
        "roles",
        "route_mode",
        "cooldown_until",
        "disabled_reason",
    ] {
        if let Some(field) = value.get(key).filter(|field| !field.is_null()) {
            object.insert(key.to_string(), field.clone());
        }
    }
    serde_json::Value::Object(object)
}

pub(crate) fn route_summary_for_challenge(challenge: &ChallengeRecord) -> serde_json::Value {
    let mut by_model = BTreeMap::<String, usize>::new();
    let mut by_primary = BTreeMap::<String, usize>::new();
    let mut by_winner = BTreeMap::<String, usize>::new();
    let mut by_mode = BTreeMap::<String, usize>::new();
    let mut selected_decisions = BTreeMap::<String, usize>::new();
    let mut decision_counts = BTreeMap::<String, usize>::new();
    let mut max_prompt_tokens = 0u64;
    let mut max_completion_tokens = 0u64;
    let mut max_total_tokens = 0u64;

    for route in &challenge.route_metadata {
        *by_model.entry(route.model.clone()).or_insert(0) += 1;
        if let Some(value) = route.primary_model_id.as_ref() {
            *by_primary.entry(value.clone()).or_insert(0) += 1;
        }
        if let Some(value) = route.winner_model_id.as_ref() {
            *by_winner.entry(value.clone()).or_insert(0) += 1;
        }
        if let Some(value) = route.route_mode.as_ref() {
            *by_mode.entry(value.clone()).or_insert(0) += 1;
        }
        if let Some(usage) = route.token_usage.as_ref() {
            max_prompt_tokens = max_prompt_tokens.max(usage.prompt_tokens);
            max_completion_tokens = max_completion_tokens.max(usage.completion_tokens);
            max_total_tokens = max_total_tokens.max(usage.total_tokens);
        }
        for decision in &route.model_decisions {
            *decision_counts
                .entry(decision.model_id.clone())
                .or_insert(0) += 1;
            if decision.selected {
                *selected_decisions
                    .entry(decision.model_id.clone())
                    .or_insert(0) += 1;
            }
        }
    }

    json!({
        "route_records": challenge.route_metadata.len(),
        "by_model": by_model,
        "by_primary_model": by_primary,
        "by_winner_model": by_winner,
        "by_route_mode": by_mode,
        "model_decisions": decision_counts,
        "selected_model_decisions": selected_decisions,
        "max_prompt_tokens": max_prompt_tokens,
        "max_completion_tokens": max_completion_tokens,
        "max_total_tokens": max_total_tokens
    })
}
