//! Jekko internal provider adapter.
//!
//! The Jekko automation lanes use the in-house OpenAI-compatible API exposed
//! under `JEKKO_API_KEY` / `JEKKO_PROVIDER_BASE_URL`. This adapter mirrors the
//! JNOccio and LiteLLM adapters but keeps the provider-specific credential and
//! default base URL.
use crate::define_openai_compat_adapter;

use super::shared::no_extra_headers;

/// Default Jekko API base URL.
pub const JEKKO_DEFAULT_BASE_URL: &str = "https://api.jekko.ai";

define_openai_compat_adapter!(
    JekkoAdapter,
    provider_id = "jekko",
    default_base_url = JEKKO_DEFAULT_BASE_URL,
    extra_headers = no_extra_headers,
    doc = "Jekko internal provider adapter."
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::{ProviderCredential, ProviderRequest};
    use crate::providers::shared::test_request_with;

    fn req() -> ProviderRequest {
        test_request_with(
            "jekko/big-pickle",
            "big-pickle",
            ProviderCredential::ApiKey {
                key: "jekko-test-key".into(),
            },
            None,
            Some(0.7),
        )
    }

    #[test]
    fn body_is_openai_compatible() {
        let a = JekkoAdapter::new();
        let body = a.build_body(&req());
        assert_eq!(body["model"], "big-pickle");
        assert_eq!(body["stream"], true);
        assert_eq!(body["max_completion_tokens"], 4096);
        assert_eq!(body["messages"][0]["role"], "system");
        assert_eq!(body["messages"][1]["content"], "hi");
    }

    #[test]
    fn headers_use_bearer_auth() {
        let a = JekkoAdapter::new();
        let h = a.build_headers(&req()).unwrap();
        assert_eq!(
            h.get(reqwest::header::AUTHORIZATION).unwrap(),
            "Bearer jekko-test-key"
        );
        assert!(h.get("x-jnoccio-client").is_none());
    }
}
