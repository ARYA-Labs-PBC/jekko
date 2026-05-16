//! LiteLLM proxy adapter.
//!
//! LiteLLM acts as an OpenAI-compatible proxy in front of arbitrary
//! underlying providers. The adapter reuses the OpenAI body and event
//! mappings; the only differences are the base URL and (optionally) the
//! requirement to always include a `tools` parameter when message history
//! contains tool calls (handled at the runtime layer).
use async_trait::async_trait;
use futures_util::StreamExt;
use serde_json::Value;
use tokio_util::sync::CancellationToken;

use crate::adapter::{require_credential, ProviderAdapter, ProviderRequest, ProviderStream};
use crate::error::{ProviderError, ProviderResult};
use crate::stream::{ProviderCapabilities, ProviderEvent, SseFrame};

use super::openai::{
    decode_openai_sse, map_openai_frame_stateful, OpenAiAdapter, OpenAiStreamState,
};
use super::shared::{headers_from, make_client, sse_into_provider_stream};

/// LiteLLM proxy adapter.
#[derive(Debug, Clone, Default)]
pub struct LiteLlmAdapter {
    client: reqwest::Client,
}

impl LiteLlmAdapter {
    /// Construct a new adapter.
    pub fn new() -> Self {
        Self {
            client: make_client(),
        }
    }

    /// Build the JSON body (OpenAI-compatible).
    pub fn build_body(&self, req: &ProviderRequest) -> Value {
        OpenAiAdapter::new().build_body(req)
    }

    /// Build the headers (Bearer auth).
    pub fn build_headers(
        &self,
        req: &ProviderRequest,
    ) -> ProviderResult<reqwest::header::HeaderMap> {
        let mut headers = headers_from(req);
        headers.insert(
            reqwest::header::HeaderName::from_static("content-type"),
            reqwest::header::HeaderValue::from_static("application/json"),
        );
        let cred = require_credential(req, "litellm")?;
        let bearer = match cred {
            crate::adapter::ProviderCredential::ApiKey { key } => key.clone(),
            crate::adapter::ProviderCredential::Bearer { token } => token.clone(),
            crate::adapter::ProviderCredential::OAuth { access_token } => access_token.clone(),
        };
        headers.insert(
            reqwest::header::AUTHORIZATION,
            reqwest::header::HeaderValue::from_str(&format!("Bearer {bearer}"))
                .map_err(|_| ProviderError::MissingCredential("litellm".into()))?,
        );
        Ok(headers)
    }

    /// Map a single SSE frame using OpenAI's parser.
    pub fn map_frame(frame: &SseFrame) -> ProviderResult<Vec<ProviderEvent>> {
        OpenAiAdapter::map_frame(frame)
    }
}

#[async_trait]
impl ProviderAdapter for LiteLlmAdapter {
    async fn stream(
        &self,
        req: ProviderRequest,
        abort: CancellationToken,
    ) -> ProviderResult<ProviderStream> {
        let base = match req.base_url.as_deref() {
            Some(base) => base,
            None => return Err(ProviderError::MissingCredential("litellm base_url".into())),
        };
        let url = format!("{}/v1/chat/completions", base);
        let body = self.build_body(&req);
        let headers = self.build_headers(&req)?;

        let response = self
            .client
            .post(&url)
            .headers(headers)
            .json(&body)
            .send()
            .await?;
        if !response.status().is_success() {
            let status = response.status().as_u16();
            // Explicit propagation: name the body-read failure instead of
            // silently coercing it to an empty string, so callers can tell
            // a body-less response apart from a transport read error.
            let body = match response.text().await {
                Ok(text) => text,
                Err(err) => format!("<failed to read error body: {err}>"),
            };
            return Err(ProviderError::Http { status, body });
        }
        let mut state = OpenAiStreamState::new();
        let stream = sse_into_provider_stream(response, abort, move |frame| {
            map_openai_frame_stateful(frame, &mut state)
        });
        Ok(Box::pin(stream.map(|r| r)) as ProviderStream)
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            streaming: true,
            cache_control: false,
            tool_streaming: true,
        }
    }
}

/// Test helper: decode a buffered LiteLLM SSE response.
pub fn decode_litellm_sse(bytes: &[u8]) -> ProviderResult<Vec<ProviderEvent>> {
    decode_openai_sse(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::ProviderCredential;
    use serde_json::{json, Map};

    #[test]
    fn requires_base_url() {
        let r = ProviderRequest {
            model: "litellm/gpt-4".into(),
            api_model_id: "gpt-4".into(),
            session_id: "sess-1".into(),
            system: vec![],
            messages: vec![json!({ "role": "user", "content": "hi" })],
            tools: vec![],
            tool_choice: None,
            options: Map::new(),
            headers: Default::default(),
            max_output_tokens: 4096,
            temperature: None,
            top_p: None,
            top_k: None,
            credential: Some(ProviderCredential::Bearer { token: "k".into() }),
            base_url: None,
        };
        let a = LiteLlmAdapter::new();
        // We can build body/headers without base_url; only stream() fails.
        let body = a.build_body(&r);
        assert_eq!(body["model"], "gpt-4");
        let h = a.build_headers(&r).unwrap();
        assert!(h.get(reqwest::header::AUTHORIZATION).is_some());
    }
}
