use std::path::Path;

use anyhow::Result;
use async_trait::async_trait;

use crate::model_policy::ModelTaskKind;

use super::labels::{kind_label, receipt_id};
use super::{ModelCallReceipt, ModelClient};

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
                route: Some(kind_label(kind).to_string()),
                credential_policy: None,
                credential_user_id: None,
                retry_count: Some(0),
            })
        } else {
            Ok(ModelCallReceipt::fake_success(kind, self.response.clone()))
        }
    }
}
