use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::Result;
use async_trait::async_trait;

use crate::model_policy::{ModelPolicy, ModelRouteRecord, ModelTaskKind};

use super::labels::{kind_label, receipt_id};
use super::{CredentialSourcePolicy, ModelCallReceipt, ModelClient};

/// Jekko-runtime-backed live model client.
///
/// This invokes `jekko run --ephemeral --json`, so the provider call still
/// routes through Jekko's runtime without forcing every runner unit test to
/// link the runtime/provider stack.
#[derive(Debug, Clone)]
pub struct JekkoRuntimeModelClient {
    provider: Option<String>,
    model_override: Option<String>,
    policy: ModelPolicy,
    credential_policy: CredentialSourcePolicy,
}

impl JekkoRuntimeModelClient {
    /// Construct with optional provider/model overrides.
    pub fn new(provider: Option<String>, model: Option<String>) -> Self {
        Self {
            provider,
            model_override: model,
            policy: ModelPolicy::default(),
            credential_policy: CredentialSourcePolicy::UsersOnly,
        }
    }

    /// Construct with a policy used when no explicit model override is supplied.
    pub fn with_policy(
        provider: Option<String>,
        model_override: Option<String>,
        policy: ModelPolicy,
    ) -> Self {
        Self {
            provider,
            model_override,
            policy,
            credential_policy: CredentialSourcePolicy::UsersOnly,
        }
    }

    /// Override the credential source policy forwarded to the runtime child.
    pub fn with_credential_policy(mut self, credential_policy: CredentialSourcePolicy) -> Self {
        self.credential_policy = credential_policy;
        self
    }

    /// Return the selected provider/model route for a task kind.
    pub fn selected_route(&self, kind: ModelTaskKind) -> ModelRouteRecord {
        let policy_route = self.policy.select(kind);
        ModelRouteRecord {
            provider: self.provider.clone().or(policy_route.provider),
            model: self.model_override.clone().or(policy_route.model),
        }
    }

    /// Return the selected model for a task kind, if one is explicitly routed.
    pub fn selected_model(&self, kind: ModelTaskKind) -> Option<String> {
        self.selected_route(kind).model
    }

    /// Test helper exposing the exact runtime argv without spawning `jekko`.
    pub fn argv_for_test(&self, kind: ModelTaskKind, cwd: &Path, prompt: &str) -> Vec<String> {
        let mut args = vec![
            "run".to_string(),
            "--ephemeral".to_string(),
            "--json".to_string(),
            "--agent".to_string(),
            "plan".to_string(),
            "--cwd".to_string(),
            cwd.display().to_string(),
        ];
        let route = self.selected_route(kind);
        if let Some(provider) = route.provider {
            args.push("--provider".to_string());
            args.push(provider);
        }
        if let Some(model) = route.model {
            args.push("--model".to_string());
            args.push(model);
        }
        args.push(prompt.to_string());
        args
    }
}

impl Default for JekkoRuntimeModelClient {
    fn default() -> Self {
        Self::new(None, None)
    }
}

#[async_trait]
impl ModelClient for JekkoRuntimeModelClient {
    async fn complete(
        &self,
        kind: ModelTaskKind,
        prompt: &str,
        cwd: &Path,
    ) -> Result<ModelCallReceipt> {
        let started = Instant::now();
        let mut command = Command::new(jekko_bin());
        command
            .env("JEKKO_RUN_DISABLE_TOOLS", "1")
            .env(
                "JEKKO_KEY_SOURCE_POLICY",
                self.credential_policy.env_value(),
            )
            .arg("run")
            .arg("--ephemeral")
            .arg("--json")
            .arg("--agent")
            .arg("plan")
            .arg("--cwd")
            .arg(cwd);
        let route = self.selected_route(kind);
        if let Some(provider) = route.provider.as_deref() {
            command.arg("--provider").arg(provider);
        }
        if let Some(selected_model) = route.model.as_deref() {
            command.arg("--model").arg(selected_model);
        }
        command.arg(prompt);
        let result = output_with_timeout(command, model_call_timeout());
        let latency_ms = started.elapsed().as_millis() as u64;
        match result {
            Ok(Some(output)) if output.status.success() => {
                let value: serde_json::Value =
                    serde_json::from_slice(&output.stdout).unwrap_or(serde_json::Value::Null);
                Ok(ModelCallReceipt {
                    id: receipt_id("live"),
                    kind: kind_label(kind).to_string(),
                    task_id: None,
                    provider: value
                        .get("provider_id")
                        .and_then(serde_json::Value::as_str)
                        .map(str::to_string)
                        .or_else(|| route.provider.clone())
                        .unwrap_or_else(|| "auto".to_string()),
                    model: value
                        .get("model_id")
                        .and_then(serde_json::Value::as_str)
                        .map(str::to_string)
                        .or_else(|| route.model.clone())
                        .unwrap_or_else(|| "auto".to_string()),
                    latency_ms,
                    success: true,
                    cost_usd: None,
                    response: value
                        .get("assistant_text")
                        .and_then(serde_json::Value::as_str)
                        .map(str::to_string),
                    error: None,
                    budget_used: None,
                    budget_remaining: None,
                    route: Some(kind_label(kind).to_string()),
                    credential_policy: Some(self.credential_policy.env_value().to_string()),
                    credential_user_id: value
                        .get("credential_user_id")
                        .and_then(serde_json::Value::as_str)
                        .map(str::to_string),
                    retry_count: value
                        .get("retry_count")
                        .and_then(serde_json::Value::as_u64)
                        .map(|value| value as usize)
                        .or(Some(0)),
                })
            }
            Ok(Some(output)) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let stdout = String::from_utf8_lossy(&output.stdout);
                let error = if stderr.trim().is_empty() {
                    stdout.trim().to_string()
                } else {
                    stderr.trim().to_string()
                };
                Ok(ModelCallReceipt {
                    id: receipt_id("live"),
                    kind: kind_label(kind).to_string(),
                    task_id: None,
                    provider: route.provider.clone().unwrap_or_else(|| "auto".to_string()),
                    model: route.model.clone().unwrap_or_else(|| "auto".to_string()),
                    latency_ms,
                    success: false,
                    cost_usd: None,
                    response: None,
                    error: Some(error),
                    budget_used: None,
                    budget_remaining: None,
                    route: Some(kind_label(kind).to_string()),
                    credential_policy: Some(self.credential_policy.env_value().to_string()),
                    credential_user_id: None,
                    retry_count: Some(0),
                })
            }
            Ok(None) => Ok(ModelCallReceipt {
                id: receipt_id("live"),
                kind: kind_label(kind).to_string(),
                task_id: None,
                provider: route.provider.clone().unwrap_or_else(|| "auto".to_string()),
                model: route.model.clone().unwrap_or_else(|| "auto".to_string()),
                latency_ms,
                success: false,
                cost_usd: None,
                response: None,
                error: Some(format!(
                    "model command timed out after {}s",
                    model_call_timeout().as_secs()
                )),
                budget_used: None,
                budget_remaining: None,
                route: Some(kind_label(kind).to_string()),
                credential_policy: Some(self.credential_policy.env_value().to_string()),
                credential_user_id: None,
                retry_count: Some(0),
            }),
            Err(err) => Ok(ModelCallReceipt {
                id: receipt_id("live"),
                kind: kind_label(kind).to_string(),
                task_id: None,
                provider: route.provider.unwrap_or_else(|| "auto".to_string()),
                model: route.model.unwrap_or_else(|| "auto".to_string()),
                latency_ms,
                success: false,
                cost_usd: None,
                response: None,
                error: Some(err.to_string()),
                budget_used: None,
                budget_remaining: None,
                route: Some(kind_label(kind).to_string()),
                credential_policy: Some(self.credential_policy.env_value().to_string()),
                credential_user_id: None,
                retry_count: Some(0),
            }),
        }
    }
}

fn output_with_timeout(mut command: Command, timeout: Duration) -> std::io::Result<Option<Output>> {
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = command.spawn()?;
    let started = Instant::now();
    loop {
        if child.try_wait()?.is_some() {
            return child.wait_with_output().map(Some);
        }
        if started.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            return Ok(None);
        }
        thread::sleep(Duration::from_millis(100));
    }
}

fn model_call_timeout() -> Duration {
    let secs = std::env::var("JEKKO_MODEL_CALL_TIMEOUT_SECS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(120)
        .max(5);
    Duration::from_secs(secs)
}

fn jekko_bin() -> PathBuf {
    std::env::var_os("JEKKO_BIN")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("jekko"))
}
