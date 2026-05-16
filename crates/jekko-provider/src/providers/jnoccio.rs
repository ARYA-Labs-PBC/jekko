//! JNOccio Fusion adapter.
//!
//! JNOccio exposes an OpenAI-compatible chat-completions endpoint, so we
//! reuse the OpenAI body/event mapping with a customised header set:
//! bearer auth plus `X-Jnoccio-*` metadata and a fixed base URL.
use crate::adapter::ProviderRequest;
use crate::define_openai_compat_adapter;
use crate::error::ProviderResult;
use crate::stream::ProviderEvent;

use super::openai::decode_openai_sse;

/// Default JNOccio Fusion base URL.
pub const JNOCCIO_DEFAULT_BASE_URL: &str = "http://127.0.0.1:4317";

/// Default JNOccio Fusion API key (handed out by the local server).
pub const JNOCCIO_DEFAULT_API_KEY: &str = "jnoccio-local";

define_openai_compat_adapter!(
    JNoccioAdapter,
    provider_id = "jnoccio",
    default_base_url = JNOCCIO_DEFAULT_BASE_URL,
    extra_headers = add_jnoccio_headers,
    doc = "JNOccio Fusion adapter."
);

/// Test helper: decode a buffered JNOccio SSE response.
pub fn decode_jnoccio_sse(bytes: &[u8]) -> ProviderResult<Vec<ProviderEvent>> {
    decode_openai_sse(bytes)
}

fn add_jnoccio_headers(
    _req: &ProviderRequest,
    headers: &mut reqwest::header::HeaderMap,
) -> ProviderResult<()> {
    headers.insert(
        reqwest::header::HeaderName::from_static("x-jnoccio-client"),
        reqwest::header::HeaderValue::from_static("jekko-rust"),
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
            "jnoccio/jnoccio-fusion",
            "jnoccio-fusion",
            ProviderCredential::ApiKey {
                key: JNOCCIO_DEFAULT_API_KEY.into(),
            },
            Some(JNOCCIO_DEFAULT_BASE_URL.into()),
            None,
        )
    }

    #[test]
    fn body_is_openai_compatible() {
        let a = JNoccioAdapter::new();
        let body = a.build_body(&req());
        assert_eq!(body["model"], "jnoccio-fusion");
        assert_eq!(body["stream"], true);
        assert_eq!(body["messages"][0]["role"], "system");
        assert_eq!(body["messages"][1]["content"], "hi");
    }

    #[test]
    fn header_includes_client_tag() {
        let a = JNoccioAdapter::new();
        let h = a.build_headers(&req()).unwrap();
        assert_eq!(h.get("x-jnoccio-client").unwrap(), "jekko-rust");
        assert!(h
            .get(reqwest::header::AUTHORIZATION)
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("Bearer "));
    }
}
