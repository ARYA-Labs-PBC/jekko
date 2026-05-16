//! Provider / model / credential / adapter selection helpers.
//!
//! Split out of [`crate::agent`] so the static lookup tables for catalog
//! entries, base URLs, and provider adapters live in one place.

use std::collections::BTreeMap;
use std::env;
use std::sync::{Arc, Mutex, OnceLock};

use jekko_provider::adapter::ProviderCredential;
use jekko_provider::providers::{
    AnthropicAdapter, JNoccioAdapter, JekkoAdapter, LiteLlmAdapter, OpenAiAdapter,
    OpenRouterAdapter,
};
use jekko_provider::routing::recommended_model_id;
use jekko_provider::setup::{
    catalog_entry, choose_active_provider, EnvValue, ModelKeySource, CATALOG,
};

use crate::error::{RuntimeError, RuntimeResult};
use crate::key_balancer::KeyBalancer;
use jekko_core::provider::{
    Model, ModelStatus, ProviderApiInfo, ProviderCapabilities, ProviderCost, ProviderId,
    ProviderInterleaved, ProviderLimit, ProviderModalities,
};

use super::types::AgentTurnRequest;

pub(super) fn select_provider_id(request: &AgentTurnRequest) -> RuntimeResult<String> {
    if let Some(provider) = request.provider.clone() {
        return Ok(provider);
    }
    let snapshot = env_snapshot();
    let developer_unlocked = snapshot
        .get("JNOCCIO_DEVELOPER_KEY")
        .and_then(|v| v.value.as_ref())
        .is_some_and(|v| !v.trim().is_empty());
    let selection = choose_active_provider(&snapshot, developer_unlocked);
    match selection.active_provider_id {
        Some(id) => Ok(id),
        None => Err(RuntimeError::invalid(NO_PROVIDER_CONFIGURED_MSG)),
    }
}

/// Error message used when no provider can be resolved from the env snapshot.
const NO_PROVIDER_CONFIGURED_MSG: &str =
    "no provider configured; set a provider explicitly or export provider credentials";

pub(super) fn select_model_id(
    provider_id: &str,
    request: &AgentTurnRequest,
) -> RuntimeResult<String> {
    if let Some(model) = request.model.clone() {
        return Ok(model);
    }
    if let Some(recommended) = recommended_model_id(provider_id) {
        return Ok(recommended.to_string());
    }
    match catalog_entry(provider_id).and_then(|entry| entry.recommended_model_id.map(String::from))
    {
        Some(id) => Ok(id),
        None => Err(RuntimeError::invalid(format!(
            "no recommended model known for `{provider_id}`"
        ))),
    }
}

/// Credential plus the user dir it came from. `user_id == None` means the
/// fallback `env::var()` path served the value — no per-user accounting is
/// available, so the balancer is not informed of the outcome.
#[derive(Debug, Clone)]
pub(super) struct SelectedCredential {
    pub credential: ProviderCredential,
    pub user_id: Option<String>,
}

pub(super) fn select_credential(
    provider_id: &str,
    model_id: &str,
) -> RuntimeResult<Option<SelectedCredential>> {
    if let Some(pick) = balancer_pick(provider_id, model_id) {
        tracing::debug!(
            provider = provider_id,
            model = model_id,
            user = %pick.user_id,
            env_name = %pick.env_name,
            "selected credential from key pool",
        );
        return Ok(Some(SelectedCredential {
            credential: pick.credential,
            user_id: Some(pick.user_id),
        }));
    }
    let Some(entry) = catalog_entry(provider_id) else {
        return Ok(None);
    };
    for env_name in entry.env_names {
        if let Ok(value) = env::var(env_name) {
            if !value.trim().is_empty() {
                return Ok(Some(SelectedCredential {
                    credential: ProviderCredential::ApiKey { key: value },
                    user_id: None,
                }));
            }
        }
    }
    Ok(None)
}

/// Lazily-initialised process-wide balancer. Tests that need a clean slate
/// can call [`reset_balancer_for_tests`].
fn balancer() -> &'static Mutex<Option<KeyBalancer>> {
    static BALANCER: OnceLock<Mutex<Option<KeyBalancer>>> = OnceLock::new();
    BALANCER.get_or_init(|| Mutex::new(KeyBalancer::new(jekko_jnoccio_boot::unlock::is_unlocked())))
}

fn balancer_pick(provider_id: &str, model_id: &str) -> Option<crate::key_balancer::KeyPick> {
    let mut guard = balancer().lock().ok()?;
    guard.as_mut()?.pick(provider_id, model_id)
}

/// Inform the balancer of a successful turn against `(provider, user, model)`.
pub(super) fn record_credential_success(provider_id: &str, user_id: &str, model_id: &str) {
    if let Ok(mut guard) = balancer().lock() {
        if let Some(bal) = guard.as_mut() {
            bal.record_success(provider_id, user_id, model_id);
        }
    }
}

/// Inform the balancer of a failed turn against `(provider, user, model)`.
/// Pass the upstream HTTP status when known; pass 0 for non-HTTP failures.
pub(super) fn record_credential_failure(
    provider_id: &str,
    user_id: &str,
    model_id: &str,
    http_status: u16,
) {
    if let Ok(mut guard) = balancer().lock() {
        if let Some(bal) = guard.as_mut() {
            if http_status >= 100 {
                bal.record_http(provider_id, user_id, model_id, http_status);
            } else {
                bal.record_failure(
                    provider_id,
                    user_id,
                    model_id,
                    crate::key_balancer::FailureKind::Other,
                );
            }
        }
    }
}

pub(super) fn select_base_url(provider_id: &str) -> Option<String> {
    if let Ok(value) = env::var("JEKKO_PROVIDER_BASE_URL") {
        if !value.trim().is_empty() {
            return Some(value);
        }
    }
    match provider_id {
        "litellm" | "llmgateway" => env::var("LITELLM_BASE_URL")
            .ok()
            .filter(|v| !v.trim().is_empty()),
        _ => None,
    }
}

pub(super) fn build_model(provider_id: &str, model_id: &str) -> RuntimeResult<Model> {
    let (api_npm, api_url) = match provider_id {
        "anthropic" => ("@ai-sdk/anthropic", "https://api.anthropic.com"),
        "openai" => ("@ai-sdk/openai", "https://api.openai.com"),
        "openrouter" => ("@openrouter/ai-sdk-provider", "https://openrouter.ai/api"),
        "jekko" => ("@ai-sdk/openai-compatible", "https://api.jekko.ai"),
        "jnoccio" => ("@ai-sdk/openai", "http://127.0.0.1:4317"),
        "litellm" | "llmgateway" => ("@ai-sdk/openai-compatible", "http://127.0.0.1:4000"),
        other => {
            return Err(RuntimeError::invalid(format!(
                "unsupported provider `{other}`"
            )))
        }
    };
    Ok(Model {
        id: jekko_core::provider::ModelId::new(format!("{provider_id}/{model_id}")),
        provider_id: ProviderId::new(provider_id),
        api: ProviderApiInfo {
            id: model_id.to_string(),
            url: api_url.to_string(),
            npm: api_npm.to_string(),
        },
        name: model_id.to_string(),
        family: None,
        capabilities: ProviderCapabilities {
            temperature: true,
            reasoning: false,
            attachment: true,
            toolcall: true,
            input: ProviderModalities {
                text: true,
                audio: false,
                image: true,
                video: false,
                pdf: true,
            },
            output: ProviderModalities::default(),
            interleaved: ProviderInterleaved::Bool(false),
        },
        cost: ProviderCost {
            input: 0.0,
            output: 0.0,
            cache: Default::default(),
            experimental_over_200k: None,
        },
        limit: ProviderLimit {
            context: 200_000.0,
            input: None,
            output: 8192.0,
        },
        status: ModelStatus::Active,
        options: Default::default(),
        headers: Default::default(),
        release_date: "2025-01-01".into(),
        variants: None,
    })
}

pub(super) fn provider_adapter(
    provider_id: &str,
) -> RuntimeResult<Arc<dyn jekko_provider::ProviderAdapter>> {
    match provider_id {
        "anthropic" => Ok(Arc::new(AnthropicAdapter::new())),
        "jekko" => Ok(Arc::new(JekkoAdapter::new())),
        "openai" => Ok(Arc::new(OpenAiAdapter::new())),
        "openrouter" => Ok(Arc::new(OpenRouterAdapter::new())),
        "jnoccio" => Ok(Arc::new(JNoccioAdapter::new())),
        "litellm" | "llmgateway" => Ok(Arc::new(LiteLlmAdapter::new())),
        other => Err(RuntimeError::invalid(format!(
            "unsupported provider `{other}`"
        ))),
    }
}

fn env_snapshot() -> BTreeMap<String, EnvValue> {
    let mut values = BTreeMap::new();
    for entry in CATALOG {
        for env_name in entry.env_names {
            let value = env::var(env_name).ok().filter(|v| !v.trim().is_empty());
            values.insert(
                (*env_name).to_string(),
                EnvValue {
                    value,
                    source: Some(ModelKeySource::ProcessEnv),
                },
            );
        }
        if let Some(companion) = entry.companion_env_names {
            for env_name in companion {
                let value = env::var(env_name).ok().filter(|v| !v.trim().is_empty());
                values.insert(
                    (*env_name).to_string(),
                    EnvValue {
                        value,
                        source: Some(ModelKeySource::ProcessEnv),
                    },
                );
            }
        }
    }
    values
}
