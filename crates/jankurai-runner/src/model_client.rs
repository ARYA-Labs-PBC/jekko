//! Model client facade for live and deterministic ZYAL port planning calls.

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::Semaphore;

use crate::model_policy::{ModelPolicy, ModelTaskKind};

/// Receipt emitted for every model call attempt.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelCallReceipt {
    /// Stable-ish receipt id.
    pub id: String,
    /// Model task kind.
    pub kind: String,
    /// Optional task id this call served.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    /// Provider id, or `fake`.
    pub provider: String,
    /// Model id.
    pub model: String,
    /// Latency in milliseconds.
    pub latency_ms: u64,
    /// Success flag.
    pub success: bool,
    /// Optional cost when reported by the provider layer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_usd: Option<f64>,
    /// Assistant text or fake response.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response: Option<String>,
    /// Error text when the call failed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Calls consumed in the active live budget.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub budget_used: Option<usize>,
    /// Calls remaining in the active live budget.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub budget_remaining: Option<usize>,
}

impl ModelCallReceipt {
    /// Deterministic success constructor for tests.
    pub fn fake_success(kind: ModelTaskKind, response: impl Into<String>) -> Self {
        Self {
            id: receipt_id("fake"),
            kind: kind_label(kind).to_string(),
            task_id: None,
            provider: "fake".to_string(),
            model: "fake-model".to_string(),
            latency_ms: 0,
            success: true,
            cost_usd: Some(0.0),
            response: Some(response.into()),
            error: None,
            budget_used: None,
            budget_remaining: None,
        }
    }

    /// Deterministic failure constructor.
    pub fn failure(
        kind: ModelTaskKind,
        provider: impl Into<String>,
        model: impl Into<String>,
        error: impl Into<String>,
    ) -> Self {
        Self {
            id: receipt_id("failure"),
            kind: kind_label(kind).to_string(),
            task_id: None,
            provider: provider.into(),
            model: model.into(),
            latency_ms: 0,
            success: false,
            cost_usd: None,
            response: None,
            error: Some(error.into()),
            budget_used: None,
            budget_remaining: None,
        }
    }
}

/// Model completion boundary.
#[async_trait]
pub trait ModelClient: Send + Sync {
    /// Complete a planning prompt.
    async fn complete(
        &self,
        kind: ModelTaskKind,
        prompt: &str,
        cwd: &Path,
    ) -> Result<ModelCallReceipt>;
}

/// Fake deterministic model client for CI.
#[derive(Debug, Clone)]
pub struct FakeModelClient {
    response: String,
    fail: bool,
}

impl FakeModelClient {
    /// Build a successful fake client.
    pub fn success(response: impl Into<String>) -> Self {
        Self {
            response: response.into(),
            fail: false,
        }
    }

    /// Build a failing fake client.
    pub fn failure(error: impl Into<String>) -> Self {
        Self {
            response: error.into(),
            fail: true,
        }
    }
}

#[async_trait]
impl ModelClient for FakeModelClient {
    async fn complete(
        &self,
        kind: ModelTaskKind,
        _prompt: &str,
        _cwd: &Path,
    ) -> Result<ModelCallReceipt> {
        if self.fail {
            Ok(ModelCallReceipt {
                id: receipt_id("fake"),
                kind: kind_label(kind).to_string(),
                task_id: None,
                provider: "fake".to_string(),
                model: "fake-model".to_string(),
                latency_ms: 0,
                success: false,
                cost_usd: Some(0.0),
                response: None,
                error: Some(self.response.clone()),
                budget_used: None,
                budget_remaining: None,
            })
        } else {
            Ok(ModelCallReceipt::fake_success(kind, self.response.clone()))
        }
    }
}

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
}

impl JekkoRuntimeModelClient {
    /// Construct with optional provider/model overrides.
    pub fn new(provider: Option<String>, model: Option<String>) -> Self {
        Self {
            provider,
            model_override: model,
            policy: ModelPolicy::default(),
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
        }
    }

    /// Return the model selected for a task kind.
    pub fn selected_model(&self, kind: ModelTaskKind) -> String {
        self.model_override
            .clone()
            .unwrap_or_else(|| self.policy.select(kind).to_string())
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
            .arg("run")
            .arg("--ephemeral")
            .arg("--json")
            .arg("--agent")
            .arg("plan")
            .arg("--cwd")
            .arg(cwd);
        if let Some(provider) = self.provider.as_deref() {
            command.arg("--provider").arg(provider);
        }
        let selected_model = self.selected_model(kind);
        command.arg("--model").arg(&selected_model);
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
                        .or_else(|| self.provider.clone())
                        .unwrap_or_else(|| "auto".to_string()),
                    model: value
                        .get("model_id")
                        .and_then(serde_json::Value::as_str)
                        .map(str::to_string)
                        .unwrap_or_else(|| selected_model.clone()),
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
                    provider: self.provider.clone().unwrap_or_else(|| "auto".to_string()),
                    model: selected_model.clone(),
                    latency_ms,
                    success: false,
                    cost_usd: None,
                    response: None,
                    error: Some(error),
                    budget_used: None,
                    budget_remaining: None,
                })
            }
            Ok(None) => Ok(ModelCallReceipt {
                id: receipt_id("live"),
                kind: kind_label(kind).to_string(),
                task_id: None,
                provider: self.provider.clone().unwrap_or_else(|| "auto".to_string()),
                model: selected_model,
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
            }),
            Err(err) => Ok(ModelCallReceipt {
                id: receipt_id("live"),
                kind: kind_label(kind).to_string(),
                task_id: None,
                provider: self.provider.clone().unwrap_or_else(|| "auto".to_string()),
                model: selected_model,
                latency_ms,
                success: false,
                cost_usd: None,
                response: None,
                error: Some(err.to_string()),
                budget_used: None,
                budget_remaining: None,
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

/// Budgeting and live-receipt guard around another model client.
#[derive(Debug)]
pub struct BudgetedModelClient<C> {
    inner: C,
    max_calls: usize,
    used: AtomicUsize,
    semaphore: Arc<Semaphore>,
    require_live: bool,
}

impl<C> BudgetedModelClient<C> {
    /// Wrap an inner model client with a call budget and concurrency cap.
    pub fn new(inner: C, max_calls: usize, max_parallel: usize, require_live: bool) -> Self {
        Self {
            inner,
            max_calls: max_calls.max(1),
            used: AtomicUsize::new(0),
            semaphore: Arc::new(Semaphore::new(max_parallel.max(1))),
            require_live,
        }
    }

    /// Number of calls used so far.
    pub fn calls_used(&self) -> usize {
        self.used.load(Ordering::SeqCst)
    }

    /// Number of calls remaining.
    pub fn calls_remaining(&self) -> usize {
        self.max_calls.saturating_sub(self.calls_used())
    }
}

#[async_trait]
impl<C> ModelClient for BudgetedModelClient<C>
where
    C: ModelClient,
{
    async fn complete(
        &self,
        kind: ModelTaskKind,
        prompt: &str,
        cwd: &Path,
    ) -> Result<ModelCallReceipt> {
        let Ok(_permit) = self.semaphore.clone().acquire_owned().await else {
            return Ok(ModelCallReceipt::failure(
                kind,
                "budget",
                "budget",
                "live call semaphore closed",
            ));
        };
        let previous = self.used.fetch_add(1, Ordering::SeqCst);
        if previous >= self.max_calls {
            self.used.store(self.max_calls, Ordering::SeqCst);
            let mut receipt = ModelCallReceipt::failure(
                kind,
                "budget",
                "budget",
                format!("live call budget exhausted at {} calls", self.max_calls),
            );
            receipt.budget_used = Some(self.max_calls);
            receipt.budget_remaining = Some(0);
            return Ok(receipt);
        }
        let used = previous + 1;
        let remaining = self.max_calls.saturating_sub(used);
        let mut receipt = self.inner.complete(kind, prompt, cwd).await?;
        receipt.budget_used = Some(used);
        receipt.budget_remaining = Some(remaining);
        if self.require_live && receipt.provider == "fake" {
            receipt.success = false;
            receipt.error = Some(
                "live model calls are required; deterministic model receipt rejected".to_string(),
            );
        }
        Ok(receipt)
    }
}

fn jekko_bin() -> PathBuf {
    std::env::var_os("JEKKO_BIN")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("jekko"))
}

pub fn kind_label(kind: ModelTaskKind) -> &'static str {
    match kind {
        ModelTaskKind::Frame => "frame",
        ModelTaskKind::StageBrainstorm => "stage_brainstorm",
        ModelTaskKind::StageCritique => "stage_critique",
        ModelTaskKind::StageReduce => "stage_reduce",
        ModelTaskKind::PhaseBrainstorm => "phase_brainstorm",
        ModelTaskKind::Hypothesis => "hypothesis",
        ModelTaskKind::Critic => "critic",
        ModelTaskKind::Verifier => "verifier",
        ModelTaskKind::MemoryCurate => "memory_curate",
        ModelTaskKind::ParityGenerate => "parity_generate",
        ModelTaskKind::PerfClose => "perf_close",
        ModelTaskKind::HardEscalation => "hard_escalation",
        ModelTaskKind::Implement => "implement",
        ModelTaskKind::PhaseFinalize => "phase_finalize",
        ModelTaskKind::StuckDebug => "stuck_debug",
        ModelTaskKind::Healing => "healing",
        ModelTaskKind::PerfGap => "perf_gap",
        ModelTaskKind::Review => "review",
        ModelTaskKind::HeroGenerate => "hero_generate",
        ModelTaskKind::JudgePatch => "judge_patch",
        ModelTaskKind::LiteratureSynthesis => "literature_synthesis",
        ModelTaskKind::RedTeam => "red_team",
        ModelTaskKind::MetaJudge => "meta_judge",
        ModelTaskKind::KnowledgeCurate => "knowledge_curate",
    }
}

fn receipt_id(prefix: &str) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    format!("model-{prefix}-{now}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn fake_model_success_receipt_is_deterministic() {
        let dir = tempdir().unwrap();
        let client = FakeModelClient::success("planned");
        let receipt = client
            .complete(ModelTaskKind::PhaseFinalize, "prompt", dir.path())
            .await
            .unwrap();
        assert!(receipt.success);
        assert_eq!(receipt.provider, "fake");
        assert_eq!(receipt.model, "fake-model");
        assert_eq!(receipt.response.as_deref(), Some("planned"));
    }

    #[tokio::test]
    async fn fake_model_failure_receipt_records_error() {
        let dir = tempdir().unwrap();
        let client = FakeModelClient::failure("no provider configured");
        let receipt = client
            .complete(ModelTaskKind::PhaseFinalize, "prompt", dir.path())
            .await
            .unwrap();
        assert!(!receipt.success);
        assert!(receipt
            .error
            .as_deref()
            .unwrap()
            .contains("no provider configured"));
    }

    #[tokio::test]
    async fn budgeted_client_blocks_after_limit() {
        let dir = tempdir().unwrap();
        let client = BudgetedModelClient::new(FakeModelClient::success("{}"), 1, 1, false);
        let first = client
            .complete(ModelTaskKind::Frame, "prompt", dir.path())
            .await
            .unwrap();
        let second = client
            .complete(ModelTaskKind::Frame, "prompt", dir.path())
            .await
            .unwrap();
        assert!(first.success);
        assert!(!second.success);
        assert_eq!(second.budget_remaining, Some(0));
        assert!(second.error.unwrap().contains("budget exhausted"));
    }

    #[tokio::test]
    async fn budgeted_client_rejects_fake_when_live_required() {
        let dir = tempdir().unwrap();
        let client = BudgetedModelClient::new(FakeModelClient::success("{}"), 2, 1, true);
        let receipt = client
            .complete(ModelTaskKind::Frame, "prompt", dir.path())
            .await
            .unwrap();
        assert!(!receipt.success);
        assert!(receipt
            .error
            .unwrap()
            .contains("deterministic model receipt rejected"));
    }

    #[test]
    fn runtime_client_routes_by_policy_without_override() {
        let client = JekkoRuntimeModelClient::with_policy(None, None, Default::default());
        assert_eq!(
            client.selected_model(ModelTaskKind::StageBrainstorm),
            "jnoccio/routine"
        );
        assert_eq!(
            client.selected_model(ModelTaskKind::StageReduce),
            "jnoccio/power-winner"
        );
        let override_client = JekkoRuntimeModelClient::with_policy(
            None,
            Some("explicit/model".into()),
            Default::default(),
        );
        assert_eq!(
            override_client.selected_model(ModelTaskKind::StageReduce),
            "explicit/model"
        );
    }
}
