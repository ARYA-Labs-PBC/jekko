//! Mock-LLM short-circuit for the provider agent executor.
//!
//! Houses the [`MOCK_LLM_ENV`] / [`MOCK_RESPONSE_ENV`] env-var contract plus
//! the deterministic [`mock_assistant_stream`] / [`mock_agent_turn_result`]
//! helpers used by PTY and integration tests. Sits above adapter selection
//! and credential lookup so it never reaches a real provider.

use futures_util::stream;
use jekko_provider::stream::{ProviderEvent, ProviderEventKind};
use jekko_provider::ProviderStream;
use serde_json::Value;

use super::super::types::{AgentTurnRequest, AgentTurnResult};

/// Environment variable that switches the provider executor into a
/// deterministic mock mode for PTY/integration tests.
///
/// When set to `"1"`, [`crate::agent::ProviderAgentExecutor::execute`]
/// short-circuits before adapter selection / credential lookup and returns
/// a fixed assistant response sourced from [`MOCK_RESPONSE_ENV`].
pub const MOCK_LLM_ENV: &str = "JEKKO_TUI_TEST_MOCK_LLM";

/// Environment variable holding the mock assistant payload.
///
/// The value may be either a plain string or a JSON object with a
/// `response` field (e.g. `{"response":"...","delayMs":25}`). When unset,
/// the executor falls back to a generic placeholder string.
pub const MOCK_RESPONSE_ENV: &str = "JEKKO_TUI_TEST_MOCK_RESPONSE";

/// Default mock assistant payload used when [`MOCK_RESPONSE_ENV`] is unset.
pub const MOCK_RESPONSE_DEFAULT: &str = "mocked assistant reply";

/// Extract the mock assistant text from [`MOCK_RESPONSE_ENV`].
///
/// Accepts either a plain string or a JSON object whose `response` field
/// holds the text. Defaults to [`MOCK_RESPONSE_DEFAULT`] when unset or
/// unparseable.
pub fn mock_assistant_text() -> String {
    let raw = std::env::var(MOCK_RESPONSE_ENV).unwrap_or_default();
    if raw.is_empty() {
        return MOCK_RESPONSE_DEFAULT.to_string();
    }
    if let Ok(Value::Object(map)) = serde_json::from_str::<Value>(&raw) {
        if let Some(text) = map.get("response").and_then(Value::as_str) {
            return text.to_string();
        }
    }
    raw
}

/// Build a deterministic [`ProviderStream`] from a fixed assistant text.
///
/// Emits `StreamStart` → `TextDelta` → `StreamEnd`, mirroring the shape
/// the real adapters yield. Used by the [`MOCK_LLM_ENV`] short-circuit
/// and by unit tests asserting the mock contract.
pub fn mock_assistant_stream() -> ProviderStream {
    let text = mock_assistant_text();
    let events = vec![
        Ok(ProviderEvent::new(ProviderEventKind::StreamStart {
            model: None,
        })),
        Ok(ProviderEvent::new(ProviderEventKind::TextDelta { text })),
        Ok(ProviderEvent::new(ProviderEventKind::StreamEnd {
            stop_reason: Some("stop".into()),
        })),
    ];
    Box::pin(stream::iter(events)) as ProviderStream
}

/// Build a deterministic [`AgentTurnResult`] from a fixed assistant text.
///
/// Used by [`MOCK_LLM_ENV`] to skip the entire provider/adapter stack
/// when the runtime is being exercised by PTY tests.
pub(super) fn mock_agent_turn_result(request: &AgentTurnRequest) -> AgentTurnResult {
    let provider_id = request
        .provider
        .clone()
        .unwrap_or_else(|| "mock".to_string());
    let model_id = request
        .model
        .clone()
        .unwrap_or_else(|| "mock-model".to_string());
    AgentTurnResult {
        provider_id,
        model_id,
        assistant_text: mock_assistant_text(),
        reasoning_text: String::new(),
        tool_calls: Vec::new(),
        credential_source_policy: None,
        selected_credential_user_id: None,
        credential_user_id: None,
        router_metadata: None,
    }
}

/// Returns true when the runtime should short-circuit into the mock LLM.
pub fn mock_llm_enabled() -> bool {
    std::env::var(MOCK_LLM_ENV).as_deref() == Ok("1")
}
