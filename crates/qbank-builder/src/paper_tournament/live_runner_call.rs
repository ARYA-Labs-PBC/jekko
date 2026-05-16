use super::*;
use serde::de::DeserializeOwned;
use serde::Serialize;

impl JnoccioHttpRunner {
    pub(crate) fn call_json_once<T>(
        &self,
        phase: &str,
        index: usize,
        attempt: usize,
        prompt: &str,
        response_schema: serde_json::Value,
    ) -> Result<(T, AgentCallReceipt), JnoccioCallError>
    where
        T: DeserializeOwned + Serialize,
    {
        let call_started = Instant::now();
        self.append_call_started(phase, index, attempt)?;
        let body = json!({
            "model": self.model,
            "messages": [
                {
                    "role": "system",
                    "content": "You are building production QBank evidence from redistributable scientific papers. Return only JSON that satisfies the schema. Never invent route metadata."
                },
                {
                    "role": "user",
                    "content": prompt
                }
            ],
            "temperature": 0.2,
            "max_tokens": self.max_output_tokens,
            "response_format": {
                "type": "json_schema",
                "json_schema": {
                    "name": format!("qbank_{phase}_output"),
                    "strict": true,
                    "schema": response_schema
                }
            }
        });
        let mut request = self.client.post(&self.endpoint).json(&body);
        if let Some(token) = self.bearer_token.as_ref() {
            request = request.bearer_auth(token);
        }
        let response = request.send().map_err(|err| {
            let category = if err.is_timeout() {
                "timeout"
            } else {
                "http_request"
            };
            self.append_after_error(
                phase,
                index,
                attempt,
                &call_started,
                false,
                "not_started",
                "unknown",
                category,
                err.to_string(),
                None,
            );
            JnoccioCallError::retryable(format!("jnoccio {phase} request failed: {err}"))
                .with_category(category)
        })?;
        let status = response.status();
        let text = response.text().map_err(|err| {
            self.append_after_error(
                phase,
                index,
                attempt,
                &call_started,
                false,
                "not_started",
                "unknown",
                "http_response_read",
                err.to_string(),
                None,
            );
            JnoccioCallError::new(format!("jnoccio {phase} response read failed: {err}"))
        })?;
        if !status.is_success() {
            self.append_after_error(
                phase,
                index,
                attempt,
                &call_started,
                false,
                "not_started",
                "unknown",
                "http_status",
                format!("HTTP {}", status.as_u16()),
                None,
            );
            return Err(JnoccioCallError::retryable(format!(
                "jnoccio {phase} returned HTTP {}: {}",
                status.as_u16(),
                text
            ))
            .with_category("route_http"));
        }
        let parsed: serde_json::Value = serde_json::from_str(&text).map_err(|err| {
            self.append_after_error(
                phase,
                index,
                attempt,
                &call_started,
                false,
                "response_json_error",
                "unknown",
                "parse",
                err.to_string(),
                None,
            );
            JnoccioCallError::retryable(format!("jnoccio {phase} response is not JSON: {err}"))
                .with_category("parse_schema")
        })?;
        let content =
            self.extract_assistant_content(&parsed, phase, index, attempt, &call_started)?;
        let route_value = match parsed.get("jnoccio") {
            Some(value) => value,
            None => {
                self.append_after_error(
                    phase,
                    index,
                    attempt,
                    &call_started,
                    false,
                    "response_json_ok",
                    "unknown",
                    "route_metadata",
                    "response missing extra.jnoccio metadata",
                    None,
                );
                return Err(JnoccioCallError::new(format!(
                    "jnoccio {phase} response missing extra.jnoccio metadata"
                ))
                .with_category("route_metadata"));
            }
        };
        let route_metadata = route_metadata_from_jnoccio(route_value).map_err(|err| {
            self.append_after_error(
                phase,
                index,
                attempt,
                &call_started,
                true,
                "response_json_ok",
                route_value
                    .get("structured_schema_status")
                    .and_then(|value| value.as_str())
                    .unwrap_or("unknown"),
                "route_metadata",
                err.clone(),
                None,
            );
            JnoccioCallError::new(err).with_category("route_metadata")
        })?;
        validate_live_route_metadata(phase, &route_metadata).map_err(|err| {
            self.append_after_error(
                phase,
                index,
                attempt,
                &call_started,
                true,
                "response_json_ok",
                route_value
                    .get("structured_schema_status")
                    .and_then(|value| value.as_str())
                    .unwrap_or("unknown"),
                "route_metadata",
                err.clone(),
                Some(&route_metadata),
            );
            JnoccioCallError::new(err).with_category("route_metadata")
        })?;
        self.validate_route_model_policy(phase, &route_metadata)
            .map_err(|err| {
                self.append_after_error(
                    phase,
                    index,
                    attempt,
                    &call_started,
                    true,
                    "response_json_ok",
                    route_value
                        .get("structured_schema_status")
                        .and_then(|value| value.as_str())
                        .unwrap_or("unknown"),
                    "route_model_policy",
                    err.message.clone(),
                    Some(&route_metadata),
                );
                err
            })?;
        let (token_usage, prompt_hash, context_hash) = self.extract_route_context(
            route_value,
            phase,
            index,
            attempt,
            &call_started,
            &route_metadata,
        )?;
        let raw_output_hash = sha256_hex(content.as_bytes());
        let receipt = AgentCallReceipt {
            agent_name: format!("{phase}-{}", index + 1),
            phase: phase.to_string(),
            prompt_hash,
            context_hash,
            raw_output_hash,
            route_metadata: Some(route_metadata.clone()),
            token_usage: Some(token_usage),
        };
        let output = parse_agent_json::<T>(&content).map_err(|err| {
            self.append_after_error(
                phase,
                index,
                attempt,
                &call_started,
                true,
                "agent_json_error",
                route_value
                    .get("structured_schema_status")
                    .and_then(|value| value.as_str())
                    .unwrap_or("unknown"),
                "parse",
                err.to_string(),
                Some(&route_metadata),
            );
            JnoccioCallError::with_receipt(
                format!(
                    "parse jnoccio {phase} output: {err}; content preview: {}",
                    content.chars().take(800).collect::<String>()
                ),
                receipt.clone(),
            )
            .with_category("parse_schema")
        })?;
        self.append_call_succeeded(phase, index, attempt, &call_started, &route_metadata)?;
        Ok((output, receipt))
    }
}
