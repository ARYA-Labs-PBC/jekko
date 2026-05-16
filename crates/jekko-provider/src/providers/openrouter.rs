//! OpenRouter adapter.
//!
//! Uses an OpenAI-compatible chat-completions endpoint with OpenRouter-specific
//! attribution headers (`HTTP-Referer`, `X-Title`).
use crate::adapter::ProviderRequest;
use crate::define_openai_compat_adapter;
use crate::error::ProviderResult;
use crate::stream::ProviderEvent;

use super::openai::decode_openai_sse;

/// Default OpenRouter base URL.
pub const OPENROUTER_DEFAULT_BASE_URL: &str = "https://openrouter.ai/api";

define_openai_compat_adapter!(
    OpenRouterAdapter,
    provider_id = "openrouter",
    default_base_url = OPENROUTER_DEFAULT_BASE_URL,
    extra_headers = add_openrouter_headers,
    doc = "OpenRouter adapter."
);

/// Test helper: decode a buffered OpenRouter SSE response.
pub fn decode_openrouter_sse(bytes: &[u8]) -> ProviderResult<Vec<ProviderEvent>> {
    decode_openai_sse(bytes)
}

/// Append OpenRouter's site/title attribution headers required for routing
/// telemetry and rate-limit categorisation on their side.
fn add_openrouter_headers(
    _req: &ProviderRequest,
    headers: &mut reqwest::header::HeaderMap,
) -> ProviderResult<()> {
    headers.insert(
        reqwest::header::HeaderName::from_static("http-referer"),
        reqwest::header::HeaderValue::from_static("https://jekko.ai"),
    );
    headers.insert(
        reqwest::header::HeaderName::from_static("x-title"),
        reqwest::header::HeaderValue::from_static("Jekko"),
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::ProviderCredential;
    use crate::providers::shared::test_request_with;

    fn req() -> ProviderRequest {
        test_request_with(
            "openrouter/anthropic/claude-3-haiku",
            "anthropic/claude-3-haiku",
            ProviderCredential::ApiKey {
                key: "or-test".into(),
            },
            None,
            None,
        )
    }

    #[test]
    fn headers_include_referer_and_title() {
        let a = OpenRouterAdapter::new();
        let h = a.build_headers(&req()).unwrap();
        assert_eq!(h.get("http-referer").unwrap(), "https://jekko.ai");
        assert_eq!(h.get("x-title").unwrap(), "Jekko");
        assert_eq!(
            h.get(reqwest::header::AUTHORIZATION).unwrap(),
            "Bearer or-test"
        );
    }
}
