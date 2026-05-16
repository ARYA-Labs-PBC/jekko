//! `webfetch` tool — fetch a URL.
//!
//! Ported from `packages/jekko/src/tool/webfetch.ts`. This implementation
//! uses `reqwest` with `rustls-tls` and streams the body so we can enforce
//! a hard `max_bytes` cap regardless of what the server claims in
//! `Content-Length`. We deliberately only allow HTTP(S) schemes.

use std::collections::BTreeMap;
use std::time::Duration;

use async_trait::async_trait;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};

use crate::error::{RuntimeError, RuntimeResult};

use super::{Tool, ToolContext, ToolOutput};

/// Default per-request timeout. Mirrors the TS default of 15s.
const DEFAULT_TIMEOUT_MS: u64 = 15_000;
/// Hard ceiling enforced regardless of caller-supplied timeout (30s).
const MAX_TIMEOUT_MS: u64 = 30_000;
/// Default body cap when caller omits `max_bytes` (5 MiB).
const DEFAULT_MAX_BYTES: u64 = 5_000_000;
/// User-agent string used for outbound requests.
const USER_AGENT: &str = concat!("jekko-runtime/", env!("CARGO_PKG_VERSION"));

const SCHEMA: &str = r#"{
  "type": "object",
  "properties": {
    "url": { "type": "string", "description": "URL to fetch (http/https)" },
    "method": { "type": "string", "description": "HTTP method (default GET)" },
    "headers": { "type": "object", "description": "Optional request headers" },
    "timeout_ms": { "type": "number", "description": "Per-request timeout in ms (default 15000, max 30000)" },
    "max_bytes": { "type": "number", "description": "Max body bytes to keep (default 5000000)" }
  },
  "required": ["url"]
}"#;

/// Input schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebFetchInput {
    /// URL to fetch.
    pub url: String,
    /// Optional HTTP method (defaults to GET).
    #[serde(default)]
    pub method: Option<String>,
    /// Optional request headers.
    #[serde(default)]
    pub headers: Option<BTreeMap<String, String>>,
    /// Optional per-request timeout in ms.
    #[serde(default)]
    pub timeout_ms: Option<u64>,
    /// Optional cap on downloaded body bytes.
    #[serde(default)]
    pub max_bytes: Option<u64>,
}

/// Structured response surfaced to the caller.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebFetchResponse {
    /// HTTP status code.
    pub status: u16,
    /// Response headers (lowercased keys).
    pub headers: BTreeMap<String, String>,
    /// Body text (truncated to `max_bytes`).
    pub body: String,
    /// `Content-Type` header value (if any).
    pub content_type: String,
    /// Actual bytes downloaded (may be larger than `body.len()` if truncated).
    pub bytes: u64,
    /// Whether the body was truncated at `max_bytes`.
    pub truncated: bool,
}

/// `webfetch` tool.
#[derive(Debug, Clone, Copy, Default)]
pub struct WebFetchTool;

#[async_trait]
impl Tool for WebFetchTool {
    fn id(&self) -> &'static str {
        "webfetch"
    }

    fn description(&self) -> &'static str {
        "Fetch a URL and return the response body (HTTP/HTTPS only)."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::from_str(SCHEMA).unwrap()
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        _ctx: ToolContext,
    ) -> RuntimeResult<ToolOutput> {
        let parsed: WebFetchInput =
            serde_json::from_value(input).map_err(|e| RuntimeError::invalid(e.to_string()))?;
        let resp = fetch_url(&parsed).await?;

        let metadata = serde_json::to_value(&resp)
            .map_err(|e| RuntimeError::other(format!("serialize webfetch response: {e}")))?;
        let title = format!("{} {}", resp.status, parsed.url);
        Ok(ToolOutput {
            title,
            output: resp.body.clone(),
            metadata,
        })
    }
}

/// Perform the underlying HTTP fetch. Split out so tests can hit it
/// without going through `ToolOutput` packaging.
pub async fn fetch_url(input: &WebFetchInput) -> RuntimeResult<WebFetchResponse> {
    let url = input.url.as_str();
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err(RuntimeError::invalid(format!(
            "webfetch only supports http/https schemes (got {url})"
        )));
    }
    let method = input
        .method
        .as_deref()
        .unwrap_or("GET")
        .trim()
        .to_ascii_uppercase();
    let method = reqwest::Method::from_bytes(method.as_bytes())
        .map_err(|e| RuntimeError::invalid(format!("invalid HTTP method: {e}")))?;
    let max_bytes = input.max_bytes.unwrap_or(DEFAULT_MAX_BYTES);
    let timeout_ms = input
        .timeout_ms
        .unwrap_or(DEFAULT_TIMEOUT_MS)
        .min(MAX_TIMEOUT_MS);

    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::USER_AGENT,
        reqwest::header::HeaderValue::from_static(USER_AGENT),
    );
    headers.insert(
        reqwest::header::ACCEPT,
        reqwest::header::HeaderValue::from_static("*/*"),
    );
    if let Some(extra) = &input.headers {
        for (k, v) in extra {
            let name = reqwest::header::HeaderName::from_bytes(k.as_bytes())
                .map_err(|e| RuntimeError::invalid(format!("invalid header name `{k}`: {e}")))?;
            let value = reqwest::header::HeaderValue::from_str(v)
                .map_err(|e| RuntimeError::invalid(format!("invalid header value: {e}")))?;
            headers.insert(name, value);
        }
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(MAX_TIMEOUT_MS))
        .build()
        .map_err(|e| RuntimeError::other(format!("build reqwest client: {e}")))?;

    let req = client
        .request(method, url)
        .headers(headers)
        .timeout(Duration::from_millis(timeout_ms));
    let resp = req
        .send()
        .await
        .map_err(|e| RuntimeError::other(format!("webfetch request failed: {e}")))?;

    let status = resp.status().as_u16();
    let mut header_map = BTreeMap::new();
    let mut content_type = String::new();
    for (k, v) in resp.headers().iter() {
        let key = k.as_str().to_ascii_lowercase();
        let value = v.to_str().unwrap_or("").to_string();
        if key == "content-type" {
            content_type = value.clone();
        }
        header_map.insert(key, value);
    }

    let mut total: u64 = 0;
    let mut buf: Vec<u8> = Vec::new();
    let mut truncated = false;
    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| RuntimeError::other(format!("webfetch read chunk: {e}")))?;
        total = total.saturating_add(chunk.len() as u64);
        if (buf.len() as u64) < max_bytes {
            let remaining = max_bytes.saturating_sub(buf.len() as u64) as usize;
            if chunk.len() > remaining {
                buf.extend_from_slice(&chunk[..remaining]);
                truncated = true;
            } else {
                buf.extend_from_slice(&chunk);
            }
        } else {
            truncated = true;
        }
    }

    // Lossy because remote bodies might not be valid UTF-8 (e.g. binary).
    let body = String::from_utf8_lossy(&buf).into_owned();
    Ok(WebFetchResponse {
        status,
        headers: header_map,
        body,
        content_type,
        bytes: total,
        truncated,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn rejects_non_http_scheme() {
        let err = WebFetchTool
            .execute(
                serde_json::json!({ "url": "file:///etc/passwd" }),
                ToolContext::bare("."),
            )
            .await
            .unwrap_err();
        match err {
            RuntimeError::InvalidInput(msg) => {
                assert!(msg.contains("http/https"), "msg={msg}");
            }
            other => panic!("expected InvalidInput, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn rejects_garbage_method() {
        let err = fetch_url(&WebFetchInput {
            url: "http://127.0.0.1:1/".into(),
            method: Some("bad method!".into()),
            headers: None,
            timeout_ms: Some(50),
            max_bytes: None,
        })
        .await
        .unwrap_err();
        assert!(matches!(err, RuntimeError::InvalidInput(_)));
    }
}
