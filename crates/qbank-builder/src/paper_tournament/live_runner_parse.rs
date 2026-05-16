use super::*;

impl JnoccioHttpRunner {
    pub(crate) fn extract_assistant_content(
        &self,
        parsed: &serde_json::Value,
        phase: &str,
        index: usize,
        attempt: usize,
        call_started: &Instant,
    ) -> Result<String, JnoccioCallError> {
        let message = match parsed
            .get("choices")
            .and_then(|value| value.as_array())
            .and_then(|choices| choices.first())
            .and_then(|choice| choice.get("message"))
        {
            Some(message) => message,
            None => {
                self.append_after_error(
                    phase,
                    index,
                    attempt,
                    call_started,
                    false,
                    "response_json_ok",
                    "unknown",
                    "response_shape",
                    "response missing assistant message",
                    None,
                );
                return Err(JnoccioCallError::new(format!(
                    "jnoccio {phase} response missing assistant message"
                ))
                .with_category("parse_schema"));
            }
        };
        if let Some(value) = message.get("content").and_then(|content| content.as_str()) {
            return Ok(value.to_string());
        }
        if let Some(value) = message
            .get("reasoning_text")
            .and_then(|value| value.as_str())
        {
            return Ok(value.to_string());
        }
        if let Some(value) = message
            .get("reasoning_content")
            .and_then(|value| value.as_str())
        {
            return Ok(value.to_string());
        }
        if let Some(value) = message.get("reasoning").and_then(|value| value.as_str()) {
            return Ok(value.to_string());
        }
        self.append_after_error(
            phase,
            index,
            attempt,
            call_started,
            false,
            "response_json_ok",
            "unknown",
            "response_shape",
            "response missing assistant content",
            None,
        );
        let preview = match serde_json::to_string(message) {
            Ok(value) => value.chars().take(800).collect::<String>(),
            Err(_) => String::new(),
        };
        Err(JnoccioCallError::new(format!(
            "jnoccio {phase} response missing assistant content: {}",
            preview
        ))
        .with_category("parse_schema"))
    }

    pub(crate) fn extract_route_context(
        &self,
        route_value: &serde_json::Value,
        phase: &str,
        index: usize,
        attempt: usize,
        call_started: &Instant,
        route_metadata: &RouteMetadata,
    ) -> Result<(TokenUsage, String, String), JnoccioCallError> {
        let token_usage = match route_metadata.token_usage.clone() {
            Some(value) => value,
            None => {
                self.append_after_error(
                    phase,
                    index,
                    attempt,
                    call_started,
                    true,
                    "response_json_ok",
                    route_value
                        .get("structured_schema_status")
                        .and_then(|value| value.as_str())
                        .unwrap_or("unknown"),
                    "route_metadata",
                    "metadata missing token_usage",
                    Some(route_metadata),
                );
                return Err(JnoccioCallError::new(format!(
                    "jnoccio {phase} metadata missing token_usage"
                ))
                .with_category("route_metadata"));
            }
        };
        let prompt_hash = match route_metadata.prompt_hash.clone() {
            Some(value) => value,
            None => {
                self.append_after_error(
                    phase,
                    index,
                    attempt,
                    call_started,
                    true,
                    "response_json_ok",
                    route_value
                        .get("structured_schema_status")
                        .and_then(|value| value.as_str())
                        .unwrap_or("unknown"),
                    "route_metadata",
                    "metadata missing prompt_hash",
                    Some(route_metadata),
                );
                return Err(JnoccioCallError::new(format!(
                    "jnoccio {phase} metadata missing prompt_hash"
                ))
                .with_category("route_metadata"));
            }
        };
        let context_hash = match route_metadata.context_hash.clone() {
            Some(value) => value,
            None => {
                self.append_after_error(
                    phase,
                    index,
                    attempt,
                    call_started,
                    true,
                    "response_json_ok",
                    route_value
                        .get("structured_schema_status")
                        .and_then(|value| value.as_str())
                        .unwrap_or("unknown"),
                    "route_metadata",
                    "metadata missing context_hash",
                    Some(route_metadata),
                );
                return Err(JnoccioCallError::new(format!(
                    "jnoccio {phase} metadata missing context_hash"
                ))
                .with_category("route_metadata"));
            }
        };
        Ok((token_usage, prompt_hash, context_hash))
    }
}
