use anyhow::Result;
use jekko_store::daemon::{self, ModelOutcomeRow};
use jekko_store::db::Db;
use serde_json::json;
use sha1::{Digest, Sha1};

use crate::model_client::ModelCallReceipt;

use super::helpers::now_ms;

/// Persist a model call receipt in `daemon_model_outcome`.
pub fn persist_model_receipt(db: &Db, run_id: &str, receipt: &ModelCallReceipt) -> Result<()> {
    daemon::upsert_model_outcome(
        db.connection(),
        &ModelOutcomeRow {
            id: receipt.id.clone(),
            run_id: run_id.to_string(),
            task_id: receipt.task_id.clone(),
            model_id: receipt.model.clone(),
            role: receipt.kind.clone(),
            cost_usd: receipt.cost_usd,
            latency_ms: Some(receipt.latency_ms as i64),
            status: if receipt.success {
                "success".to_string()
            } else {
                "failure".to_string()
            },
            reviewer_score: None,
            winner: receipt.success,
            payload_json: Some(json!({
                "provider": receipt.provider,
                "response_sha256": receipt.response.as_ref().map(|response| {
                    let mut hasher = Sha1::new();
                    hasher.update(response.as_bytes());
                    format!("{:x}", hasher.finalize())
                }),
                "response_bytes": receipt.response.as_ref().map(|response| response.len()),
                "error": receipt.error,
                "budget_used": receipt.budget_used,
                "budget_remaining": receipt.budget_remaining,
                "route": receipt.route,
                "credential_policy": receipt.credential_policy,
                "credential_user_id": receipt.credential_user_id,
                "retry_count": receipt.retry_count,
            })),
            time_created: now_ms(),
            time_updated: now_ms(),
        },
    )?;
    daemon::record_model_reliability_outcome(
        db.connection(),
        &receipt.model,
        &receipt.kind,
        &receipt.kind,
        receipt.success,
        receipt.success,
        receipt.latency_ms as i64,
        receipt.cost_usd.unwrap_or(0.0),
        now_ms(),
    )?;
    Ok(())
}
