use super::*;

pub(crate) fn retry_jitter_ms(phase: &str, index: usize, attempt: usize) -> u64 {
    let exponent = attempt.min(5) as u32;
    let base = 500_u64.saturating_mul(2_u64.saturating_pow(exponent));
    let hash = sha256_hex(format!("{phase}:{index}:{attempt}").as_bytes());
    let jitter = u64::from_str_radix(&hash[..4], 16).unwrap_or(0) % 750;
    base.saturating_add(jitter).min(30_000)
}

pub(crate) fn route_metadata_from_jnoccio(
    value: &serde_json::Value,
) -> Result<RouteMetadata, String> {
    let mut metadata: RouteMetadata = serde_json::from_value(value.clone())
        .map_err(|err| format!("parse jnoccio route metadata: {err}"))?;
    if metadata.route_confidence.is_none() {
        metadata.route_confidence = value.get("confidence").and_then(|value| value.as_f64());
    }
    Ok(metadata)
}

pub(crate) fn validate_live_route_metadata(
    phase: &str,
    route: &RouteMetadata,
) -> Result<(), String> {
    if route.provider != "jnoccio" {
        return Err(format!("jnoccio {phase} route provider is not jnoccio"));
    }
    if route.request_id.trim().is_empty()
        || route.model.trim().is_empty()
        || route.route_mode.as_deref().unwrap_or("").trim().is_empty()
        || route
            .primary_model_id
            .as_deref()
            .unwrap_or("")
            .trim()
            .is_empty()
        || route
            .winner_model_id
            .as_deref()
            .unwrap_or("")
            .trim()
            .is_empty()
        || route.prompt_hash.as_deref().unwrap_or("").trim().is_empty()
        || route
            .context_hash
            .as_deref()
            .unwrap_or("")
            .trim()
            .is_empty()
        || route
            .receipts_hash
            .as_deref()
            .unwrap_or("")
            .trim()
            .is_empty()
        || route
            .model_decisions_hash
            .as_deref()
            .unwrap_or("")
            .trim()
            .is_empty()
    {
        return Err(format!("jnoccio {phase} route metadata is incomplete"));
    }
    if route.route_mode.as_deref() == Some("fusion")
        && route
            .fusion_model_id
            .as_deref()
            .unwrap_or("")
            .trim()
            .is_empty()
    {
        return Err(format!(
            "jnoccio {phase} fusion route missing fusion_model_id"
        ));
    }
    if route.backup_model_ids.is_empty()
        || route.token_usage.is_none()
        || route.model_decisions.is_empty()
    {
        return Err(format!("jnoccio {phase} route metadata is incomplete"));
    }
    if route.request_id.to_ascii_lowercase().starts_with("mock") {
        return Err(format!("jnoccio {phase} request_id is not live"));
    }
    Ok(())
}

impl JnoccioHttpRunner {
    pub(crate) fn append_call_started(
        &self,
        phase: &str,
        index: usize,
        attempt: usize,
    ) -> Result<(), JnoccioCallError> {
        self.append_progress(&json!({
            "event": "before_call",
            "phase": phase,
            "attempt": attempt,
            "agent_index": index,
            "elapsed_ms": 0,
            "route_metadata_present": false,
            "parse_status": "not_started",
            "schema_status": "not_started",
            "error_category": null
        }))
    }

    pub(crate) fn append_call_succeeded(
        &self,
        phase: &str,
        index: usize,
        attempt: usize,
        call_started: &Instant,
        route_metadata: &RouteMetadata,
    ) -> Result<(), JnoccioCallError> {
        self.append_progress(&json!({
            "event": "after_call",
            "phase": phase,
            "attempt": attempt,
            "agent_index": index,
            "elapsed_ms": call_started.elapsed().as_millis() as u64,
            "route_metadata_present": true,
            "parse_status": "ok",
            "schema_status": "unknown",
            "error_category": null,
            "request_id": route_metadata.request_id,
            "route_mode": route_metadata.route_mode,
            "winner_model_id": route_metadata.winner_model_id,
            "token_usage": route_metadata.token_usage
        }))
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn append_after_error(
        &self,
        phase: &str,
        index: usize,
        attempt: usize,
        call_started: &Instant,
        route_metadata_present: bool,
        parse_status: &str,
        schema_status: &str,
        error_category: &str,
        error: impl Into<String>,
        route_metadata: Option<&RouteMetadata>,
    ) {
        let mut row = json!({
            "event": "after_call",
            "phase": phase,
            "attempt": attempt,
            "agent_index": index,
            "elapsed_ms": call_started.elapsed().as_millis() as u64,
            "route_metadata_present": route_metadata_present,
            "parse_status": parse_status,
            "schema_status": schema_status,
            "error_category": error_category,
            "error": error.into()
        });
        if let (Some(map), Some(route)) = (row.as_object_mut(), route_metadata) {
            map.insert("request_id".to_string(), json!(route.request_id));
            map.insert("route_mode".to_string(), json!(route.route_mode));
            map.insert("winner_model_id".to_string(), json!(route.winner_model_id));
            map.insert("token_usage".to_string(), json!(route.token_usage));
        }
        let _ = self.append_progress(&row);
    }

    pub(crate) fn append_progress(
        &self,
        value: &serde_json::Value,
    ) -> Result<(), JnoccioCallError> {
        let path = &self.progress_jsonl;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|err| {
                JnoccioCallError::non_retryable(format!(
                    "create progress dir {}: {err}",
                    parent.display()
                ))
            })?;
        }
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|err| {
                JnoccioCallError::non_retryable(format!(
                    "open progress jsonl {}: {err}",
                    path.display()
                ))
            })?;
        let mut row = value.clone();
        if let Some(map) = row.as_object_mut() {
            map.insert(
                "run_root".to_string(),
                json!(self.run_root.display().to_string()),
            );
            map.insert("model".to_string(), json!(self.model));
        }
        writeln!(
            file,
            "{}",
            serde_json::to_string(&row).map_err(|err| {
                JnoccioCallError::non_retryable(format!("serialize progress row: {err}"))
            })?
        )
        .map_err(|err| {
            JnoccioCallError::non_retryable(format!(
                "write progress jsonl {}: {err}",
                path.display()
            ))
        })
    }

    pub(crate) fn validate_route_model_policy(
        &self,
        phase: &str,
        route: &RouteMetadata,
    ) -> Result<(), JnoccioCallError> {
        let model = route
            .winner_model_id
            .as_deref()
            .unwrap_or(route.model.as_str());
        let mut deny = self.route_model_deny.clone();
        if self.strict_production {
            deny.push(RouteModelPolicy {
                phase: "generator".to_string(),
                pattern: "mistral-codestral".to_string(),
            });
            deny.push(RouteModelPolicy {
                phase: "testing".to_string(),
                pattern: "mistral-codestral".to_string(),
            });
            deny.push(RouteModelPolicy {
                phase: "verification".to_string(),
                pattern: "mistral-codestral".to_string(),
            });
        }
        if policy_matches(&deny, phase, model) {
            return Err(JnoccioCallError::retryable(format!(
                "route model policy denied {phase} model {model}"
            ))
            .with_category("route_model_policy"));
        }
        let allows = self
            .route_model_allow
            .iter()
            .filter(|policy| policy.phase == phase)
            .cloned()
            .collect::<Vec<_>>();
        if !allows.is_empty() && !policy_matches(&allows, phase, model) {
            return Err(JnoccioCallError::retryable(format!(
                "route model policy did not allow {phase} model {model}"
            ))
            .with_category("route_model_policy"));
        }
        Ok(())
    }
}

fn policy_matches(policies: &[RouteModelPolicy], phase: &str, model: &str) -> bool {
    policies.iter().any(|policy| {
        policy.phase == phase
            && regex::Regex::new(&policy.pattern)
                .map(|regex| regex.is_match(model))
                .unwrap_or(false)
    })
}
