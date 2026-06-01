//! Deterministic local provider for offline agent-workflow tests.
//!
//! `dummy_agent_llm` never reads credentials, opens sockets, sleeps, or uses
//! randomness. It replays strict scenario fixtures from
//! `dummy_agent_llm_scenarios.json` into the same [`ProviderAdapter`] stream
//! shape as the network-backed adapters, which lets runtime tests exercise
//! text, tool-call, and error paths without spending tokens.

use std::collections::BTreeSet;
use std::sync::OnceLock;

use async_trait::async_trait;
use futures_util::stream;
use serde::Deserialize;
use serde_json::Value;
use tokio_util::sync::CancellationToken;

use crate::adapter::{ProviderAdapter, ProviderRequest, ProviderStream};
use crate::error::{ProviderError, ProviderResult};
use crate::stream::{ProviderCapabilities, ProviderEvent, ProviderEventKind};

/// Provider id used to select the deterministic local adapter.
pub const DUMMY_AGENT_LLM_PROVIDER_ID: &str = "dummy_agent_llm";

/// Default scenario/model id used when no model is explicitly supplied.
pub const DUMMY_AGENT_LLM_DEFAULT_MODEL: &str = "default";

const SCENARIOS_JSON: &str = include_str!("dummy_agent_llm_scenarios.json");

/// Deterministic local `dummy_agent_llm` provider adapter.
#[derive(Debug, Clone, Default)]
pub struct DummyAgentLlmAdapter;

impl DummyAgentLlmAdapter {
    /// Construct a new deterministic local adapter.
    pub fn new() -> Self {
        Self
    }

    /// Return all built-in scenarios after strict fixture parsing and
    /// validation.
    pub fn scenarios(&self) -> ProviderResult<&'static [DummyScenario]> {
        scenarios()
    }

    /// Resolve the scenario id that would be used for `req`.
    pub fn scenario_id_for_request(req: &ProviderRequest) -> String {
        scenario_id_for_request(req)
    }
}

#[async_trait]
impl ProviderAdapter for DummyAgentLlmAdapter {
    async fn stream(
        &self,
        req: ProviderRequest,
        abort: CancellationToken,
    ) -> ProviderResult<ProviderStream> {
        if abort.is_cancelled() {
            return Err(ProviderError::Aborted);
        }
        let scenario_id = scenario_id_for_request(&req);
        let scenario = scenarios()?
            .iter()
            .find(|scenario| scenario.id == scenario_id)
            .ok_or_else(|| {
                ProviderError::ProviderEvent(format!(
                    "unknown dummy_agent_llm scenario `{scenario_id}`"
                ))
            })?;
        let frames = scenario.frames_for_request(&req);
        let events = frames
            .iter()
            .map(|frame| frame.to_stream_item(scenario))
            .collect::<Vec<_>>();
        Ok(Box::pin(stream::iter(events)) as ProviderStream)
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            streaming: true,
            cache_control: false,
            tool_streaming: true,
        }
    }
}

fn scenario_id_for_request(req: &ProviderRequest) -> String {
    for key in ["scenario", "dummy_agent_llm_scenario"] {
        if let Some(value) = req.options.get(key).and_then(Value::as_str) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                return strip_provider_prefix(trimmed).to_string();
            }
        }
    }
    if let Some(value) = req
        .options
        .get(DUMMY_AGENT_LLM_PROVIDER_ID)
        .and_then(|value| value.get("scenario"))
        .and_then(Value::as_str)
    {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return strip_provider_prefix(trimmed).to_string();
        }
    }
    if let Some(value) = req.headers.get("x-jekko-dummy-scenario") {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return strip_provider_prefix(trimmed).to_string();
        }
    }
    let api_model = strip_provider_prefix(req.api_model_id.trim());
    if !api_model.is_empty() {
        return api_model.to_string();
    }
    let model = strip_provider_prefix(req.model.trim());
    if !model.is_empty() {
        return model.to_string();
    }
    DUMMY_AGENT_LLM_DEFAULT_MODEL.to_string()
}

fn strip_provider_prefix(value: &str) -> &str {
    value
        .strip_prefix("dummy_agent_llm/")
        .unwrap_or(value)
        .trim()
}

fn scenarios() -> ProviderResult<&'static [DummyScenario]> {
    static SCENARIOS: OnceLock<Result<Vec<DummyScenario>, String>> = OnceLock::new();
    match SCENARIOS.get_or_init(load_scenarios) {
        Ok(scenarios) => Ok(scenarios.as_slice()),
        Err(message) => Err(ProviderError::Json(message.clone())),
    }
}

fn load_scenarios() -> Result<Vec<DummyScenario>, String> {
    let fixture: ScenarioFixture = serde_json::from_str(SCENARIOS_JSON)
        .map_err(|err| format!("invalid dummy_agent_llm fixture: {err}"))?;
    validate_scenarios(fixture.scenarios)
}

fn validate_scenarios(scenarios: Vec<DummyScenario>) -> Result<Vec<DummyScenario>, String> {
    if scenarios.is_empty() {
        return Err("dummy_agent_llm fixture must define at least one scenario".into());
    }
    let mut ids = BTreeSet::new();
    for scenario in &scenarios {
        if scenario.id.trim().is_empty() {
            return Err("dummy_agent_llm scenario id must not be blank".into());
        }
        if !ids.insert(scenario.id.clone()) {
            return Err(format!(
                "dummy_agent_llm scenario id `{}` is duplicated",
                scenario.id
            ));
        }
        if scenario.provider != DUMMY_AGENT_LLM_PROVIDER_ID {
            return Err(format!(
                "dummy_agent_llm scenario `{}` has provider `{}`",
                scenario.id, scenario.provider
            ));
        }
        if scenario.model.trim().is_empty() {
            return Err(format!(
                "dummy_agent_llm scenario `{}` model must not be blank",
                scenario.id
            ));
        }
        if scenario.frames.is_empty() {
            return Err(format!(
                "dummy_agent_llm scenario `{}` must define at least one frame",
                scenario.id
            ));
        }
        if scenario.after_tool_frames.is_empty() {
            continue;
        }
        if !scenario.frames.iter().any(DummyScenarioFrame::is_tool_call_end) {
            return Err(format!(
                "dummy_agent_llm scenario `{}` has after_tool_frames without a tool-call-end frame",
                scenario.id
            ));
        }
    }
    Ok(scenarios)
}

/// One strict fixture scenario replayed by [`DummyAgentLlmAdapter`].
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DummyScenario {
    /// Stable scenario id. Also used as the dummy model id.
    pub id: String,
    /// Human-readable title for docs/test diagnostics.
    pub title: String,
    /// Optional longer explanation of the scenario intent.
    #[serde(default)]
    pub description: String,
    /// Search/filter tags for test authors.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Provider id; must be `dummy_agent_llm`.
    pub provider: String,
    /// Model id associated with this scenario.
    pub model: String,
    /// Ordered frames replayed into the provider stream.
    pub frames: Vec<DummyScenarioFrame>,
    /// Optional frames replayed after the runtime sends a tool-result message.
    #[serde(default)]
    pub after_tool_frames: Vec<DummyScenarioFrame>,
}

impl DummyScenario {
    fn frames_for_request(&self, req: &ProviderRequest) -> &[DummyScenarioFrame] {
        if !self.after_tool_frames.is_empty() && request_contains_tool_result(req) {
            &self.after_tool_frames
        } else {
            &self.frames
        }
    }
}

fn request_contains_tool_result(req: &ProviderRequest) -> bool {
    req.messages.iter().any(message_contains_tool_result)
}

fn message_contains_tool_result(message: &Value) -> bool {
    if message.get("role").and_then(Value::as_str) == Some("tool") {
        return true;
    }
    message
        .get("content")
        .and_then(Value::as_array)
        .is_some_and(|parts| {
            parts.iter().any(|part| {
                part.get("type").and_then(Value::as_str) == Some("tool-result")
            })
        })
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
struct ScenarioFixture {
    scenarios: Vec<DummyScenario>,
}

/// One replay frame in a [`DummyScenario`].
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case", deny_unknown_fields)]
pub enum DummyScenarioFrame {
    /// Stream has started.
    StreamStart {
        /// Optional model id reported in the start event.
        #[serde(default)]
        model: Option<String>,
    },
    /// Text delta.
    TextDelta {
        /// Text chunk to append.
        text: String,
    },
    /// Reasoning delta.
    ReasoningDelta {
        /// Reasoning text chunk to append.
        text: String,
    },
    /// Tool-call start frame.
    ToolCallStart {
        /// Tool call id.
        id: String,
        /// Tool name.
        name: String,
    },
    /// Partial tool input JSON frame.
    ToolCallInputDelta {
        /// Tool call id.
        id: String,
        /// JSON delta text.
        delta: String,
    },
    /// Completed tool-call frame.
    ToolCallEnd {
        /// Tool call id.
        id: String,
        /// Tool name.
        name: String,
        /// Fully parsed tool input JSON.
        input: Value,
    },
    /// Usage frame. Dummy scenarios should keep all counts at zero.
    Usage {
        /// Input tokens.
        input_tokens: u64,
        /// Output tokens.
        output_tokens: u64,
        /// Cache read tokens.
        #[serde(default)]
        cache_read_tokens: u64,
        /// Cache write tokens.
        #[serde(default)]
        cache_write_tokens: u64,
    },
    /// Metadata frame.
    Metadata {
        /// Arbitrary metadata payload.
        metadata: Value,
    },
    /// Clean stream end.
    StreamEnd {
        /// Optional stop reason.
        #[serde(default)]
        stop_reason: Option<String>,
    },
    /// Scripted provider failure.
    Error {
        /// Error message surfaced as [`ProviderError::ProviderEvent`].
        message: String,
    },
}

impl DummyScenarioFrame {
    fn is_tool_call_end(&self) -> bool {
        matches!(self, Self::ToolCallEnd { .. })
    }

    fn to_stream_item(&self, scenario: &DummyScenario) -> ProviderResult<ProviderEvent> {
        let kind = match self {
            Self::StreamStart { model } => ProviderEventKind::StreamStart {
                model: Some(
                    model
                        .clone()
                        .unwrap_or_else(|| format!("{}/{}", scenario.provider, scenario.model)),
                ),
            },
            Self::TextDelta { text } => ProviderEventKind::TextDelta { text: text.clone() },
            Self::ReasoningDelta { text } => ProviderEventKind::ReasoningDelta {
                text: text.clone(),
            },
            Self::ToolCallStart { id, name } => ProviderEventKind::ToolCallStart {
                id: id.clone(),
                name: name.clone(),
            },
            Self::ToolCallInputDelta { id, delta } => ProviderEventKind::ToolCallInputDelta {
                id: id.clone(),
                delta: delta.clone(),
            },
            Self::ToolCallEnd { id, name, input } => ProviderEventKind::ToolCallEnd {
                id: id.clone(),
                name: name.clone(),
                input: input.clone(),
            },
            Self::Usage {
                input_tokens,
                output_tokens,
                cache_read_tokens,
                cache_write_tokens,
            } => ProviderEventKind::Usage {
                input_tokens: *input_tokens,
                output_tokens: *output_tokens,
                cache_read_tokens: *cache_read_tokens,
                cache_write_tokens: *cache_write_tokens,
            },
            Self::Metadata { metadata } => ProviderEventKind::Metadata {
                metadata: metadata.clone(),
            },
            Self::StreamEnd { stop_reason } => ProviderEventKind::StreamEnd {
                stop_reason: stop_reason.clone(),
            },
            Self::Error { message } => {
                return Err(ProviderError::ProviderEvent(message.clone()));
            }
        };
        Ok(ProviderEvent::new(kind))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use futures_util::StreamExt;
    use serde_json::{json, Map};

    use super::*;

    fn req(model: &str) -> ProviderRequest {
        ProviderRequest {
            model: format!("dummy_agent_llm/{model}"),
            api_model_id: model.to_string(),
            session_id: "dummy-session".into(),
            system: vec![],
            messages: vec![json!({ "role": "user", "content": "hello" })],
            tools: vec![],
            tool_choice: None,
            options: Map::new(),
            headers: BTreeMap::new(),
            max_output_tokens: 1024,
            temperature: None,
            top_p: None,
            top_k: None,
            credential: None,
            base_url: None,
        }
    }

    #[tokio::test]
    async fn default_scenario_is_deterministic_text_stream() {
        let adapter = DummyAgentLlmAdapter::new();
        let mut first = adapter
            .stream(req("default"), CancellationToken::new())
            .await
            .unwrap();
        let mut second = adapter
            .stream(req("default"), CancellationToken::new())
            .await
            .unwrap();

        let first_events = collect_events(&mut first).await.unwrap();
        let second_events = collect_events(&mut second).await.unwrap();
        assert_eq!(first_events, second_events);

        let text: String = first_events
            .iter()
            .filter_map(|event| match &event.kind {
                ProviderEventKind::TextDelta { text } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(text, "dummy_agent_llm deterministic response");
        assert!(first_events.iter().any(|event| {
            matches!(
                &event.kind,
                ProviderEventKind::Usage {
                    input_tokens: 0,
                    output_tokens: 0,
                    cache_read_tokens: 0,
                    cache_write_tokens: 0
                }
            )
        }));
    }

    #[tokio::test]
    async fn tool_call_scenario_emits_completed_tool_call() {
        let adapter = DummyAgentLlmAdapter::new();
        let mut stream = adapter
            .stream(req("tool-call"), CancellationToken::new())
            .await
            .unwrap();
        let events = collect_events(&mut stream).await.unwrap();
        let tool = events
            .iter()
            .find_map(|event| match &event.kind {
                ProviderEventKind::ToolCallEnd { id, name, input } => {
                    Some((id.as_str(), name.as_str(), input.clone()))
                }
                _ => None,
            })
            .expect("tool end frame");
        assert_eq!(tool.0, "call_dummy_read_1");
        assert_eq!(tool.1, "Read");
        assert_eq!(tool.2, json!({ "path": "README.md" }));
    }

    #[tokio::test]
    async fn tool_call_scenario_uses_follow_up_after_tool_result() {
        let adapter = DummyAgentLlmAdapter::new();
        let mut request = req("tool-call");
        request.messages.push(json!({
            "role": "tool",
            "content": [{
                "type": "tool-result",
                "toolCallId": "call_dummy_read_1",
                "toolName": "Read",
                "output": {"text": "read output"}
            }]
        }));
        let mut stream = adapter
            .stream(request, CancellationToken::new())
            .await
            .unwrap();
        let events = collect_events(&mut stream).await.unwrap();
        let text: String = events
            .iter()
            .filter_map(|event| match &event.kind {
                ProviderEventKind::TextDelta { text } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(
            text,
            "Read completed; dummy_agent_llm has the deterministic tool result."
        );
        assert!(!events
            .iter()
            .any(|event| matches!(&event.kind, ProviderEventKind::ToolCallEnd { .. })));
    }

    #[tokio::test]
    async fn error_scenario_returns_scripted_provider_error() {
        let adapter = DummyAgentLlmAdapter::new();
        let mut stream = adapter
            .stream(req("error"), CancellationToken::new())
            .await
            .unwrap();
        let mut saw_start = false;
        while let Some(item) = stream.next().await {
            match item {
                Ok(event) => {
                    if matches!(event.kind, ProviderEventKind::StreamStart { .. }) {
                        saw_start = true;
                    }
                }
                Err(ProviderError::ProviderEvent(message)) => {
                    assert!(saw_start);
                    assert_eq!(message, "dummy_agent_llm scripted failure");
                    return;
                }
                Err(other) => panic!("unexpected error: {other}"),
            }
        }
        panic!("expected scripted error frame");
    }

    #[test]
    fn scenario_can_be_selected_from_options() {
        let mut request = req("default");
        request
            .options
            .insert("scenario".into(), json!("dummy_agent_llm/tool-call"));
        assert_eq!(
            DummyAgentLlmAdapter::scenario_id_for_request(&request),
            "tool-call"
        );
    }

    async fn collect_events(stream: &mut ProviderStream) -> ProviderResult<Vec<ProviderEvent>> {
        let mut events = Vec::new();
        while let Some(item) = stream.next().await {
            events.push(item?);
        }
        Ok(events)
    }
}
