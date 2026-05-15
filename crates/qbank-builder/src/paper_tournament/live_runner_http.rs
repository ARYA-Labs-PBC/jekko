use super::*;
use serde::de::DeserializeOwned;
use serde::Serialize;

impl JnoccioHttpRunner {
    pub(crate) fn new(config: &BuildPaperTournamentConfig) -> Result<Self, String> {
        let base_url = config
            .jnoccio_base_url
            .as_deref()
            .ok_or("--agent-runner jnoccio requires --jnoccio-base-url")?
            .trim()
            .trim_end_matches('/');
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(
                config.jnoccio_request_timeout_seconds.max(1),
            ))
            .build()
            .map_err(|err| format!("build jnoccio http client: {err}"))?;
        Ok(Self {
            client,
            endpoint: format!("{base_url}/v1/chat/completions"),
            model: configured_jnoccio_model(config),
            max_output_tokens: config.jnoccio_max_output_tokens,
            phase_retries: config.phase_retries,
            bearer_token: std::env::var("JNOCCIO_BEARER_TOKEN")
                .ok()
                .filter(|value| !value.trim().is_empty()),
            progress_jsonl: progress_jsonl_path(config),
            run_root: config.run_root.clone(),
            strict_production: config.strict_production,
            route_model_deny: config.route_model_deny.clone(),
            route_model_allow: config.route_model_allow.clone(),
        })
    }

    pub(crate) fn call_json<T>(
        &self,
        phase: &str,
        index: usize,
        prompt: &str,
        response_schema: serde_json::Value,
    ) -> Result<(T, AgentCallReceipt), JnoccioCallError>
    where
        T: DeserializeOwned + Serialize,
    {
        let mut last_error = None;
        for attempt in 0..=self.phase_retries {
            match self.call_json_once::<T>(phase, index, attempt, prompt, response_schema.clone()) {
                Ok(result) => return Ok(result),
                Err(err) => {
                    if !err.retryable {
                        return Err(err);
                    }
                    if attempt < self.phase_retries {
                        let jitter_ms = retry_jitter_ms(phase, index, attempt);
                        std::thread::sleep(Duration::from_millis(jitter_ms));
                    }
                    last_error = Some(if attempt == 0 {
                        err
                    } else {
                        err.with_context(format!("retry_attempt={attempt}"))
                    });
                }
            }
        }
        match last_error {
            Some(err) => Err(err),
            None => Err(JnoccioCallError::new(format!(
                "jnoccio {phase} failed before request dispatch"
            ))),
        }
    }
}
