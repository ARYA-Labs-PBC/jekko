//! Shared HTTP plumbing for provider adapters.
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use bytes::Bytes;
use futures_util::stream::{Stream, StreamExt};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde_json::{json, Map, Value};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::adapter::{ProviderRequest, ProviderStream};
use crate::error::{ProviderError, ProviderResult};
use crate::stream::{ProviderCapabilities, ProviderEvent, SseDecoder, SseFrame};

/// Default request timeout for non-streaming setup (headers / connect).
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(120);

/// Construct a [`reqwest::Client`] tuned for streaming SSE.
///
/// The builder options used here (no TLS config, no proxy override) cannot
/// fail at runtime, so we surface any unexpected error via `expect` rather
/// than silently degrading to a default client that would lose the SSE
/// pool/nodelay tuning.
pub fn make_client() -> reqwest::Client {
    reqwest::Client::builder()
        .pool_max_idle_per_host(0)
        .tcp_nodelay(true)
        .build()
        .expect("reqwest client builder with default TLS must not fail")
}

/// Build a [`HeaderMap`] from a [`ProviderRequest::headers`] string map,
/// silently skipping headers with invalid names/values (e.g. accidentally
/// inserted control bytes from user config).
pub fn headers_from(req: &ProviderRequest) -> HeaderMap {
    let mut map = HeaderMap::new();
    for (k, v) in &req.headers {
        if let (Ok(name), Ok(value)) = (HeaderName::try_from(k.as_str()), HeaderValue::try_from(v))
        {
            map.insert(name, value);
        }
    }
    map
}

/// Build the OpenAI-compatible JSON body used by OpenAI and Jekko-style
/// adapters.
pub fn build_openai_style_body(req: &ProviderRequest) -> Value {
    let mut body = Map::new();
    body.insert("model".into(), Value::String(req.api_model_id.clone()));
    body.insert("stream".into(), Value::Bool(true));
    body.insert(
        "max_completion_tokens".into(),
        Value::Number(serde_json::Number::from(req.max_output_tokens)),
    );

    if let Some(t) = req.temperature {
        body.insert(
            "temperature".into(),
            serde_json::Number::from_f64(t)
                .map(Value::Number)
                .unwrap_or(Value::Null),
        );
    }
    if let Some(p) = req.top_p {
        body.insert(
            "top_p".into(),
            serde_json::Number::from_f64(p)
                .map(Value::Number)
                .unwrap_or(Value::Null),
        );
    }

    let mut messages: Vec<Value> = Vec::new();
    for seg in &req.system {
        messages.push(json!({ "role": "system", "content": seg }));
    }
    messages.extend(req.messages.clone());
    body.insert("messages".into(), Value::Array(messages));

    if !req.tools.is_empty() {
        let tools: Vec<Value> = req
            .tools
            .iter()
            .map(|t| {
                json!({
                    "type": "function",
                    "function": {
                        "name": t.name,
                        "description": t.description,
                        "parameters": t.input_schema.clone(),
                    },
                })
            })
            .collect();
        body.insert("tools".into(), Value::Array(tools));
        if let Some(tc) = &req.tool_choice {
            let v = match tc.as_str() {
                "auto" => json!("auto"),
                "required" => json!("required"),
                "none" => json!("none"),
                other => json!(other),
            };
            body.insert("tool_choice".into(), v);
        }
    }

    if let Some(opts) = req.options.get("openai").and_then(Value::as_object) {
        for (k, v) in opts {
            body.insert(k.clone(), v.clone());
        }
    }

    if let Some(store) = req.options.get("store") {
        body.insert("store".into(), store.clone());
    }
    if let Some(key) = req.options.get("promptCacheKey") {
        body.insert("prompt_cache_key".into(), key.clone());
    }

    Value::Object(body)
}

/// Build the OpenAI-style authorization headers shared by OpenAI and
/// Jekko-style adapters.
pub fn build_openai_style_headers(
    req: &ProviderRequest,
    provider_id: &str,
) -> ProviderResult<HeaderMap> {
    let mut headers = headers_from(req);
    headers.insert(
        HeaderName::from_static("content-type"),
        HeaderValue::from_static("application/json"),
    );
    let cred = crate::adapter::require_credential(req, provider_id)?;
    let bearer = match cred {
        crate::adapter::ProviderCredential::ApiKey { key } => key.clone(),
        crate::adapter::ProviderCredential::Bearer { token } => token.clone(),
        crate::adapter::ProviderCredential::OAuth { access_token } => access_token.clone(),
    };
    headers.insert(
        reqwest::header::AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {bearer}"))
            .map_err(|_| ProviderError::MissingCredential(provider_id.into()))?,
    );
    Ok(headers)
}

/// Internal spec for OpenAI-compatible provider adapters.
#[derive(Debug, Clone, Copy)]
pub(crate) struct OpenAiCompatSpec {
    /// Provider identifier used for credential lookup and errors.
    pub(crate) provider_id: &'static str,
    /// Default base URL when the request does not override it.
    pub(crate) default_base_url: &'static str,
    /// Adapter-specific extra headers appended after the OpenAI-style set.
    pub(crate) extra_headers: fn(&ProviderRequest, &mut HeaderMap) -> ProviderResult<()>,
}

/// Internal helper that owns the shared request and streaming scaffold for
/// OpenAI-compatible providers.
#[derive(Debug, Clone)]
pub(crate) struct OpenAiCompatAdapter {
    client: reqwest::Client,
    spec: OpenAiCompatSpec,
}

impl OpenAiCompatAdapter {
    /// Construct a new helper for a specific provider shape.
    pub(crate) fn new(spec: OpenAiCompatSpec) -> Self {
        Self {
            client: make_client(),
            spec,
        }
    }

    /// Build the JSON body for the underlying chat-completions call.
    pub(crate) fn build_body(&self, req: &ProviderRequest) -> Value {
        build_openai_style_body(req)
    }

    /// Build the request headers, including provider-specific metadata.
    pub(crate) fn build_headers(&self, req: &ProviderRequest) -> ProviderResult<HeaderMap> {
        let mut headers = build_openai_style_headers(req, self.spec.provider_id)?;
        (self.spec.extra_headers)(req, &mut headers)?;
        Ok(headers)
    }

    /// Stream the response from the provider's chat-completions endpoint.
    pub(crate) async fn stream(
        &self,
        req: ProviderRequest,
        abort: CancellationToken,
    ) -> ProviderResult<McpReceiverStream<ProviderResult<ProviderEvent>>> {
        let base = req
            .base_url
            .as_deref()
            .unwrap_or(self.spec.default_base_url);
        let url = format!("{base}/v1/chat/completions");
        let body = self.build_body(&req);
        let headers = self.build_headers(&req)?;

        let mut state = super::openai::OpenAiStreamState::new();
        let stream =
            post_json_sse_stream(&self.client, &url, headers, &body, abort, move |frame| {
                super::openai::map_openai_frame_stateful(frame, &mut state)
            })
            .await?;
        Ok(stream)
    }

    /// Stream the response as a fully boxed [`ProviderStream`], shared by
    /// every adapter that simply delegates to the OpenAI-compatible pipeline.
    pub(crate) async fn boxed_stream(
        &self,
        req: ProviderRequest,
        abort: CancellationToken,
    ) -> ProviderResult<ProviderStream> {
        let stream = self.stream(req, abort).await?;
        Ok(Box::pin(stream))
    }
}

/// Capability set shared by every OpenAI-compatible delegating adapter (Jekko,
/// JNOccio). Hoisted so the adapter wrappers stay symbolic 1-liners.
pub(crate) fn openai_compat_capabilities() -> ProviderCapabilities {
    ProviderCapabilities {
        streaming: true,
        cache_control: false,
        tool_streaming: true,
    }
}

/// Generate a public `*Adapter` newtype that delegates every method to an
/// internal [`OpenAiCompatAdapter`]. Keeps the surface area identical to a
/// hand-written wrapper but eliminates the per-provider boilerplate.
///
/// Each invocation produces:
/// - a `pub struct $name { inner: OpenAiCompatAdapter }`
/// - inherent constructors and `build_body` / `build_headers` / `map_frame`
/// - `Default` and `ProviderAdapter` impls
///
/// The spec fields are baked into a `const ${NAME}_SPEC: OpenAiCompatSpec`
/// hidden in the macro expansion, so there is no per-provider spec literal
/// duplicated in product code. `$doc` becomes the struct docstring.
#[macro_export]
macro_rules! define_openai_compat_adapter {
    (
        $(#[$meta:meta])*
        $name:ident,
        provider_id = $provider_id:expr,
        default_base_url = $base_url:expr,
        extra_headers = $extra:expr,
        doc = $doc:literal
    ) => {
        #[doc = $doc]
        $(#[$meta])*
        #[derive(Debug, Clone)]
        pub struct $name {
            inner: $crate::providers::shared::OpenAiCompatAdapter,
        }

        impl $name {
            #[doc = concat!("Construct a new ", stringify!($name), ".")]
            pub fn new() -> Self {
                const SPEC: $crate::providers::shared::OpenAiCompatSpec =
                    $crate::providers::shared::OpenAiCompatSpec {
                        provider_id: $provider_id,
                        default_base_url: $base_url,
                        extra_headers: $extra,
                    };
                Self {
                    inner: $crate::providers::shared::OpenAiCompatAdapter::new(SPEC),
                }
            }

            /// Build the JSON request body.
            pub fn build_body(
                &self,
                req: &$crate::adapter::ProviderRequest,
            ) -> ::serde_json::Value {
                self.inner.build_body(req)
            }

            /// Build the request headers.
            pub fn build_headers(
                &self,
                req: &$crate::adapter::ProviderRequest,
            ) -> $crate::error::ProviderResult<::reqwest::header::HeaderMap> {
                self.inner.build_headers(req)
            }

            /// Map a single SSE frame using the OpenAI parser.
            pub fn map_frame(
                frame: &$crate::stream::SseFrame,
            ) -> $crate::error::ProviderResult<Vec<$crate::stream::ProviderEvent>> {
                $crate::providers::openai::OpenAiAdapter::map_frame(frame)
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        #[::async_trait::async_trait]
        impl $crate::adapter::ProviderAdapter for $name {
            async fn stream(
                &self,
                req: $crate::adapter::ProviderRequest,
                abort: ::tokio_util::sync::CancellationToken,
            ) -> $crate::error::ProviderResult<$crate::adapter::ProviderStream> {
                self.inner.boxed_stream(req, abort).await
            }

            fn capabilities(&self) -> $crate::stream::ProviderCapabilities {
                $crate::providers::shared::openai_compat_capabilities()
            }
        }
    };
}

/// No-op extra-header hook for OpenAI-compatible providers without metadata.
pub(crate) fn no_extra_headers(
    _req: &ProviderRequest,
    _headers: &mut HeaderMap,
) -> ProviderResult<()> {
    Ok(())
}

/// Hand-rolled receiver stream that does not depend on `tokio-stream`.
pub struct McpReceiverStream<T> {
    rx: mpsc::Receiver<T>,
}

impl<T> McpReceiverStream<T> {
    /// Construct from a tokio mpsc receiver.
    pub fn new(rx: mpsc::Receiver<T>) -> Self {
        Self { rx }
    }
}

impl<T> Stream for McpReceiverStream<T> {
    type Item = T;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.rx.poll_recv(cx)
    }
}

/// Maps a [`reqwest::Response`] into an async stream of [`ProviderEvent`]s by
/// running a per-frame `map_event` closure over each decoded SSE block.
///
/// The closure receives the raw frame and returns zero or more canonical
/// events. Errors short-circuit the stream.
pub fn sse_into_provider_stream<F>(
    response: reqwest::Response,
    abort: CancellationToken,
    mut map_event: F,
) -> McpReceiverStream<ProviderResult<ProviderEvent>>
where
    F: FnMut(&SseFrame) -> ProviderResult<Vec<ProviderEvent>> + Send + 'static,
{
    let (tx, rx) = mpsc::channel(128);
    let mut decoder = SseDecoder::new();
    tokio::spawn(async move {
        let mut body = response.bytes_stream();
        loop {
            tokio::select! {
                _ = abort.cancelled() => {
                    let _ = tx.send(Err(ProviderError::Aborted)).await;
                    break;
                }
                next = body.next() => {
                    match next {
                        Some(Ok(chunk)) => {
                            let frames = decoder.feed(&chunk);
                            for frame in frames {
                                match map_event(&frame) {
                                    Ok(events) => {
                                        for ev in events {
                                            if tx.send(Ok(ev)).await.is_err() {
                                                return;
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        let _ = tx.send(Err(e)).await;
                                        return;
                                    }
                                }
                            }
                        }
                        Some(Err(e)) => {
                            let _ = tx.send(Err(ProviderError::Transport(e.to_string()))).await;
                            return;
                        }
                        None => break,
                    }
                }
            }
        }
        let frames = decoder.flush();
        for frame in frames {
            if let Ok(events) = map_event(&frame) {
                for ev in events {
                    if tx.send(Ok(ev)).await.is_err() {
                        return;
                    }
                }
            }
        }
    });
    McpReceiverStream::new(rx)
}

/// Send a JSON POST and convert the SSE response into a provider stream.
pub async fn post_json_sse_stream<F>(
    client: &reqwest::Client,
    url: &str,
    headers: HeaderMap,
    body: &Value,
    abort: CancellationToken,
    map_event: F,
) -> ProviderResult<McpReceiverStream<ProviderResult<ProviderEvent>>>
where
    F: FnMut(&SseFrame) -> ProviderResult<Vec<ProviderEvent>> + Send + 'static,
{
    let response = client.post(url).headers(headers).json(body).send().await?;
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

    Ok(sse_into_provider_stream(response, abort, map_event))
}

/// Buffer-mode variant of [`sse_into_provider_stream`] used by tests: takes a
/// fully-buffered byte slice and synchronously produces the event sequence.
///
/// Each chunk is fed to the SSE decoder in one shot, then the resulting frames
/// are run through `map_event`.
pub fn sse_decode_all<F>(bytes: &[u8], mut map_event: F) -> ProviderResult<Vec<ProviderEvent>>
where
    F: FnMut(&SseFrame) -> ProviderResult<Vec<ProviderEvent>>,
{
    let mut decoder = SseDecoder::new();
    let mut out = Vec::new();
    let frames = decoder.feed(&Bytes::copy_from_slice(bytes));
    for frame in frames {
        out.extend(map_event(&frame)?);
    }
    let final_frames = decoder.flush();
    for frame in final_frames {
        out.extend(map_event(&frame)?);
    }
    Ok(out)
}

/// Try to parse the SSE data payload as JSON.
pub fn parse_data_as_json(data: &str) -> ProviderResult<Value> {
    serde_json::from_str(data).map_err(|e| ProviderError::SseDecode(e.to_string()))
}

/// Outcome of pre-parsing a raw SSE frame. The decoder either short-circuits
/// with a pre-baked event vector (end-of-stream sentinel, ignored event,
/// blank data) or hands back the JSON payload ready for protocol-specific
/// mapping.
pub enum SsePreparse<'a> {
    /// Short-circuit: the frame already maps to this set of events; the
    /// caller should return them directly without further parsing.
    Resolved(Vec<ProviderEvent>),
    /// Frame data is ready to hand off to the per-provider JSON mapper.
    Payload(&'a str),
}

/// Preparse a raw SSE frame using a shared triage routine. Adapters call
/// this once at the top of their per-frame mapper to handle the universal
/// concerns (empty data, end-of-stream sentinel, ignored keepalive events)
/// in one place. `done_sentinel` is the literal data payload that signals
/// end-of-stream (e.g. `Some("[DONE]")` for OpenAI-shaped streams, `None`
/// for protocols that use a structured event). `skip_event` lets callers
/// drop protocol-specific keepalive events like Anthropic's `ping`.
pub fn preparse_sse_frame<'a>(
    frame: &'a SseFrame,
    done_sentinel: Option<&str>,
    skip_event: Option<&str>,
) -> SsePreparse<'a> {
    if let Some(done) = done_sentinel {
        if frame.data == done {
            return SsePreparse::Resolved(vec![ProviderEvent::new(
                crate::stream::ProviderEventKind::StreamEnd { stop_reason: None },
            )]);
        }
    }
    if frame.data.is_empty() {
        return SsePreparse::Resolved(Vec::new());
    }
    if let Some(name) = skip_event {
        if frame.event == name {
            return SsePreparse::Resolved(Vec::new());
        }
    }
    SsePreparse::Payload(&frame.data)
}

/// Test-only fixture builder shared by every provider adapter's unit suite.
/// Lets each adapter override the differentiating fields (model id, api id,
/// credential, base url) without duplicating the 18-line `ProviderRequest`
/// boilerplate in every test module.
#[cfg(test)]
pub(crate) fn test_request_with(
    model: &str,
    api_model_id: &str,
    credential: crate::adapter::ProviderCredential,
    base_url: Option<String>,
    temperature: Option<f64>,
) -> crate::adapter::ProviderRequest {
    crate::adapter::ProviderRequest {
        model: model.into(),
        api_model_id: api_model_id.into(),
        session_id: "sess-1".into(),
        system: vec!["sys".into()],
        messages: vec![json!({ "role": "user", "content": "hi" })],
        tools: vec![],
        tool_choice: None,
        options: serde_json::Map::new(),
        headers: Default::default(),
        max_output_tokens: 4096,
        temperature,
        top_p: None,
        top_k: None,
        credential: Some(credential),
        base_url,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::adapter::{ProviderCredential, ProviderRequest};

    #[test]
    fn build_openai_style_helpers_cover_body_and_headers() {
        let req = ProviderRequest {
            model: "openai/gpt-4.1".into(),
            api_model_id: "gpt-4.1".into(),
            session_id: "sess-1".into(),
            system: vec!["system prompt".into()],
            messages: vec![json!({ "role": "user", "content": "hi" })],
            tools: vec![],
            tool_choice: None,
            options: serde_json::Map::new(),
            headers: BTreeMap::new(),
            max_output_tokens: 256,
            temperature: Some(0.2),
            top_p: None,
            top_k: None,
            credential: Some(ProviderCredential::ApiKey {
                key: "demo-key".into(),
            }),
            base_url: None,
        };

        let body = build_openai_style_body(&req);
        assert_eq!(body["model"], "gpt-4.1");
        assert_eq!(body["messages"][0]["role"], "system");
        assert_eq!(body["messages"][0]["content"], "system prompt");

        let headers = build_openai_style_headers(&req, "openai").unwrap();
        assert_eq!(
            headers
                .get(reqwest::header::AUTHORIZATION)
                .and_then(|value| value.to_str().ok()),
            Some("Bearer demo-key")
        );
    }
}
