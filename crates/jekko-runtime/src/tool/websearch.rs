//! `websearch` tool — search the web via the Brave Search API.
//!
//! Ported from `packages/jekko/src/tool/websearch.ts`. The TS module
//! talks to Exa via an MCP adapter; here we use Brave Search directly to
//! keep the runtime free of additional services. Set the
//! `BRAVE_SEARCH_API_KEY` env var to enable.

use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::{RuntimeError, RuntimeResult};

use super::{Tool, ToolContext, ToolOutput};

/// Brave Search REST endpoint.
pub const BRAVE_SEARCH_URL: &str = "https://api.search.brave.com/res/v1/web/search";
/// Env var carrying the Brave API key.
pub const BRAVE_API_KEY_ENV: &str = "BRAVE_SEARCH_API_KEY";

const SCHEMA: &str = r#"{
  "type": "object",
  "properties": {
    "query":   { "type": "string", "description": "Search query" },
    "count":   { "type": "number", "description": "Number of results (1..=20, default 10)" },
    "country": { "type": "string", "description": "Optional 2-letter country code" }
  },
  "required": ["query"]
}"#;

/// Input schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSearchInput {
    /// Search query.
    pub query: String,
    /// Optional result count (1..=20, defaults to 10).
    #[serde(default)]
    pub count: Option<u32>,
    /// Optional country bias (e.g. `"us"`).
    #[serde(default)]
    pub country: Option<String>,
}

/// One search result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSearchResult {
    /// Page title.
    pub title: String,
    /// Result URL.
    pub url: String,
    /// Snippet / description.
    pub description: String,
}

/// Aggregate response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSearchResponse {
    /// Echoed query.
    pub query: String,
    /// Search results in provider order.
    pub results: Vec<WebSearchResult>,
}

/// `websearch` tool.
#[derive(Debug, Clone, Copy, Default)]
pub struct WebSearchTool;

#[async_trait]
impl Tool for WebSearchTool {
    fn id(&self) -> &'static str {
        "websearch"
    }

    fn description(&self) -> &'static str {
        "Search the web via Brave Search and return ranked results."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::from_str(SCHEMA).unwrap()
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        _ctx: ToolContext,
    ) -> RuntimeResult<ToolOutput> {
        let parsed: WebSearchInput =
            serde_json::from_value(input).map_err(|e| RuntimeError::invalid(e.to_string()))?;
        let resp = search(&parsed).await?;
        let title = format!(
            "Web search: {} ({} results)",
            resp.query,
            resp.results.len()
        );
        let mut lines: Vec<String> = Vec::with_capacity(resp.results.len());
        for (idx, r) in resp.results.iter().enumerate() {
            lines.push(format!(
                "{}. {}\n   {}\n   {}",
                idx + 1,
                r.title,
                r.url,
                r.description
            ));
        }
        let output = if lines.is_empty() {
            "No results.".to_string()
        } else {
            lines.join("\n\n")
        };
        let metadata = serde_json::to_value(&resp)
            .map_err(|e| RuntimeError::other(format!("serialize websearch response: {e}")))?;
        Ok(ToolOutput {
            title,
            output,
            metadata,
        })
    }
}

/// Internal: perform the search.
pub async fn search(input: &WebSearchInput) -> RuntimeResult<WebSearchResponse> {
    let api_key = std::env::var(BRAVE_API_KEY_ENV)
        .map_err(|_| RuntimeError::other(format!("{BRAVE_API_KEY_ENV} not set")))?;
    let query = input.query.trim();
    if query.is_empty() {
        return Err(RuntimeError::invalid("query must not be empty"));
    }
    let count = input.count.unwrap_or(10).clamp(1, 20);

    let mut url = format!(
        "{}?q={}&count={}",
        BRAVE_SEARCH_URL,
        urlencoding::encode(query),
        count
    );
    if let Some(country) = &input.country {
        let trimmed = country.trim();
        if !trimmed.is_empty() {
            url.push_str("&country=");
            url.push_str(&urlencoding::encode(trimmed));
        }
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| RuntimeError::other(format!("build reqwest client: {e}")))?;
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::ACCEPT,
        reqwest::header::HeaderValue::from_static("application/json"),
    );
    let key_value = reqwest::header::HeaderValue::from_str(&api_key)
        .map_err(|e| RuntimeError::invalid(format!("invalid {BRAVE_API_KEY_ENV}: {e}")))?;
    headers.insert(
        reqwest::header::HeaderName::from_static("x-subscription-token"),
        key_value,
    );

    let resp = client
        .get(&url)
        .headers(headers)
        .send()
        .await
        .map_err(|e| RuntimeError::other(format!("websearch request failed: {e}")))?;

    let status = resp.status();
    let body = resp
        .text()
        .await
        .map_err(|e| RuntimeError::other(format!("websearch read body: {e}")))?;
    if !status.is_success() {
        return Err(RuntimeError::other(format!(
            "websearch http {}: {}",
            status.as_u16(),
            body.chars().take(400).collect::<String>()
        )));
    }
    let parsed: serde_json::Value = serde_json::from_str(&body).map_err(|e| {
        RuntimeError::other(format!(
            "websearch parse json: {e}; body head: {}",
            body.chars().take(200).collect::<String>()
        ))
    })?;

    let mut results = Vec::<WebSearchResult>::new();
    if let Some(arr) = parsed
        .get("web")
        .and_then(|w| w.get("results"))
        .and_then(|r| r.as_array())
    {
        for item in arr {
            let title = item
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let url_field = item
                .get("url")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let description = item
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if url_field.is_empty() {
                continue;
            }
            results.push(WebSearchResult {
                title,
                url: url_field,
                description,
            });
        }
    }

    Ok(WebSearchResponse {
        query: query.to_string(),
        results,
    })
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    /// Cross-test lock so the env-var mutation in one test doesn't race
    /// with another concurrent test in the same binary. The lock guards
    /// the `BRAVE_SEARCH_API_KEY` env var; release it after restoring.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    /// Snapshot the env var, run `f`, then restore.
    fn with_env<R>(key: &str, value: Option<&str>, f: impl FnOnce() -> R) -> R {
        let _guard = match ENV_LOCK.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        let prior = std::env::var(key).ok();
        unsafe {
            // SAFETY: writes are serialised by `ENV_LOCK`; these tests are the
            // only writers of this env var in this process.
            match value {
                Some(v) => std::env::set_var(key, v),
                None => std::env::remove_var(key),
            }
        }
        let out = f();
        unsafe {
            // SAFETY: writes are serialised by `ENV_LOCK`; we restore the
            // original env var after the test body and no other writers exist.
            match prior {
                Some(v) => std::env::set_var(key, v),
                None => std::env::remove_var(key),
            }
        }
        out
    }

    #[test]
    fn errors_when_api_key_missing() {
        let err = with_env(BRAVE_API_KEY_ENV, None, || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(WebSearchTool.execute(
                    serde_json::json!({ "query": "rust" }),
                    ToolContext::bare("."),
                ))
                .unwrap_err()
        });
        match err {
            RuntimeError::Other(msg) => {
                assert!(msg.contains(BRAVE_API_KEY_ENV), "msg={msg}");
            }
            other => panic!("expected Other(...), got {other:?}"),
        }
    }

    #[test]
    fn empty_query_is_invalid() {
        let err = with_env(BRAVE_API_KEY_ENV, Some("fake-key-for-test"), || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(search(&WebSearchInput {
                    query: "   ".into(),
                    count: None,
                    country: None,
                }))
                .unwrap_err()
        });
        assert!(matches!(err, RuntimeError::InvalidInput(_)));
    }
}
