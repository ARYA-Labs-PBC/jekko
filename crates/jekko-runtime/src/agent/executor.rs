//! Provider-backed one-shot agent executor.
//!
//! Holds the [`AgentExecutor`] trait, the default [`ProviderAgentExecutor`],
//! and the streaming tool-call loop that talks to the provider adapter.
//!
//! The implementation is split across sibling files: [`mock`] owns the
//! `JEKKO_TUI_TEST_MOCK_LLM` short-circuit + its env-var helpers, and
//! [`turn`] owns the `AgentExecutor` impl for [`ProviderAgentExecutor`]
//! (the streaming tool-call loop plus its private helpers). This file
//! keeps the trait, struct, constructors, default adapter resolver,
//! `Debug` impl, and integration tests for the mock contract +
//! provenance headers.

use std::sync::Arc;

use crate::error::RuntimeResult;

use super::provider::provider_adapter;
use super::types::{AgentTurnRequest, AgentTurnResult};

mod mock;
mod turn;

pub use mock::{
    mock_assistant_stream, mock_assistant_text, mock_llm_enabled, MOCK_LLM_ENV,
    MOCK_RESPONSE_DEFAULT, MOCK_RESPONSE_ENV,
};

/// Pluggable one-shot agent executor.
#[async_trait::async_trait]
pub trait AgentExecutor: Send + Sync {
    /// Execute a one-shot agent turn.
    async fn execute(&self, request: AgentTurnRequest) -> RuntimeResult<AgentTurnResult>;
}

/// Provider adapter lookup hook.
pub trait ProviderAdapterResolver: Send + Sync {
    /// Resolve a provider adapter for `provider_id`.
    fn resolve(&self, provider_id: &str)
        -> RuntimeResult<Arc<dyn jekko_provider::ProviderAdapter>>;
}

/// Default provider-backed executor.
pub struct ProviderAgentExecutor {
    pub(super) permissions: Arc<crate::permission::PermissionService>,
    pub(super) sessions: Arc<crate::session::SessionService>,
    pub(super) resolver: Arc<dyn ProviderAdapterResolver>,
}

impl ProviderAgentExecutor {
    /// Construct a new provider-backed executor.
    pub fn new(
        permissions: Arc<crate::permission::PermissionService>,
        sessions: Arc<crate::session::SessionService>,
    ) -> Self {
        Self::with_resolver(
            permissions,
            sessions,
            Arc::new(DefaultProviderAdapterResolver),
        )
    }

    /// Construct with a caller-supplied adapter resolver.
    pub fn with_resolver(
        permissions: Arc<crate::permission::PermissionService>,
        sessions: Arc<crate::session::SessionService>,
        resolver: Arc<dyn ProviderAdapterResolver>,
    ) -> Self {
        Self {
            permissions,
            sessions,
            resolver,
        }
    }
}

#[derive(Debug, Default)]
struct DefaultProviderAdapterResolver;

impl ProviderAdapterResolver for DefaultProviderAdapterResolver {
    fn resolve(
        &self,
        provider_id: &str,
    ) -> RuntimeResult<Arc<dyn jekko_provider::ProviderAdapter>> {
        provider_adapter(provider_id)
    }
}

impl std::fmt::Debug for ProviderAgentExecutor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProviderAgentExecutor")
            .field("permissions", &self.permissions)
            .field("sessions", &self.sessions)
            .field("resolver", &"<dyn ProviderAdapterResolver>")
            .finish()
    }
}

#[cfg(test)]
mod mock_llm_hook_tests {
    //! Unit coverage for the [`MOCK_LLM_ENV`] short-circuit. Each test owns
    //! a tiny env-guard plus `#[serial]` because the cases share process
    //! env vars (`JEKKO_TUI_TEST_MOCK_*`) and would race in parallel.

    use super::turn::{api_model_id_for, runtime_provenance_headers};
    use super::*;
    use futures_util::StreamExt;
    use jekko_provider::stream::ProviderEventKind;
    use serial_test::serial;

    /// RAII guard that scopes a process-wide env var change to a single test.
    struct EnvVarGuard {
        key: &'static str,
        original: Option<String>,
    }

    impl EnvVarGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let original = std::env::var(key).ok();
            std::env::set_var(key, value);
            Self { key, original }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            match self.original.take() {
                Some(value) => std::env::set_var(self.key, value),
                None => std::env::remove_var(self.key),
            }
        }
    }

    #[tokio::test]
    #[serial(jekko_mock_llm_env)]
    async fn mock_llm_hook_yields_text_delta() {
        let _mock = EnvVarGuard::set(MOCK_LLM_ENV, "1");
        let _resp = EnvVarGuard::set(MOCK_RESPONSE_ENV, "hello from mock");

        let mut stream = mock_assistant_stream();
        let mut events = Vec::new();
        while let Some(item) = stream.next().await {
            events.push(item.expect("mock stream never yields errors"));
        }
        assert_eq!(events.len(), 3, "mock stream is start + delta + end");
        assert!(
            matches!(events[0].kind, ProviderEventKind::StreamStart { .. }),
            "first event must be StreamStart: got {:?}",
            events[0].kind
        );
        match &events[1].kind {
            ProviderEventKind::TextDelta { text } => {
                assert_eq!(text, "hello from mock");
            }
            other => panic!("second event must be TextDelta: got {other:?}"),
        }
        assert!(
            matches!(events[2].kind, ProviderEventKind::StreamEnd { .. }),
            "third event must be StreamEnd: got {:?}",
            events[2].kind
        );
    }

    #[test]
    #[serial(jekko_mock_llm_env)]
    fn mock_assistant_text_accepts_plain_string() {
        let _resp = EnvVarGuard::set(MOCK_RESPONSE_ENV, "plain text reply");
        assert_eq!(mock_assistant_text(), "plain text reply");
    }

    #[test]
    #[serial(jekko_mock_llm_env)]
    fn mock_assistant_text_accepts_json_response_field() {
        let _resp = EnvVarGuard::set(
            MOCK_RESPONSE_ENV,
            r#"{"response":"json-shaped reply","delayMs":25}"#,
        );
        assert_eq!(mock_assistant_text(), "json-shaped reply");
    }

    #[test]
    #[serial(jekko_mock_llm_env)]
    fn mock_assistant_text_falls_back_when_unset() {
        // Ensure we restore whatever the surrounding env had.
        let original = std::env::var(MOCK_RESPONSE_ENV).ok();
        std::env::remove_var(MOCK_RESPONSE_ENV);
        let value = mock_assistant_text();
        match original {
            Some(v) => std::env::set_var(MOCK_RESPONSE_ENV, v),
            None => std::env::remove_var(MOCK_RESPONSE_ENV),
        }
        assert_eq!(value, MOCK_RESPONSE_DEFAULT);
    }

    #[test]
    #[serial(jekko_mock_llm_env)]
    fn mock_llm_enabled_requires_exact_one() {
        let _mock = EnvVarGuard::set(MOCK_LLM_ENV, "1");
        assert!(mock_llm_enabled());

        let _mock_zero = EnvVarGuard::set(MOCK_LLM_ENV, "0");
        assert!(!mock_llm_enabled());
    }

    #[test]
    #[serial(jekko_runtime_provenance_env)]
    fn runtime_provenance_headers_include_zyal_and_credential_fields() {
        let _run = EnvVarGuard::set("JEKKO_ZYAL_RUN_ID", "run-123");
        let _lane = EnvVarGuard::set("JEKKO_ZYAL_LANE_ID", "openqg");
        let _role = EnvVarGuard::set("JEKKO_AGENT_ROLE", "researcher");
        let request = AgentTurnRequest {
            prompt: "hello".to_string(),
            parsed_prompt: crate::prompt::parse("hello"),
            cwd: std::path::PathBuf::from("/tmp"),
            session_id: "session-1".to_string(),
            agent: None,
            provider: None,
            model: None,
            credential: None,
            selected_credential_user_id: None,
            ephemeral: true,
        };

        let headers = runtime_provenance_headers(&request, "users-only", Some("user_2"));

        assert_eq!(
            headers.get("x-jekko-client").map(String::as_str),
            Some("jekko-runtime")
        );
        assert_eq!(
            headers.get("x-jekko-run-id").map(String::as_str),
            Some("run-123")
        );
        assert_eq!(
            headers.get("x-jekko-zyal-lane-id").map(String::as_str),
            Some("openqg")
        );
        assert_eq!(
            headers.get("x-jekko-agent-role").map(String::as_str),
            Some("researcher")
        );
        assert_eq!(
            headers
                .get("x-jekko-credential-user-id")
                .map(String::as_str),
            Some("user_2")
        );
        assert_eq!(
            headers.get("x-jekko-credential-policy").map(String::as_str),
            Some("users-only")
        );
    }

    #[test]
    fn jnoccio_router_uses_gateway_model_id() {
        assert_eq!(
            api_model_id_for("jnoccio", "jnoccio-router"),
            "jnoccio-fusion"
        );
        assert_eq!(api_model_id_for("jnoccio", "custom"), "custom");
        assert_eq!(
            api_model_id_for("openrouter", "jnoccio-router"),
            "jnoccio-router"
        );
    }
}
