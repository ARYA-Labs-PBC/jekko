use super::*;

pub(crate) struct JnoccioCallError {
    pub(crate) message: String,
    pub(crate) receipt: Option<AgentCallReceipt>,
    pub(crate) retryable: bool,
    pub(crate) category: Option<String>,
}

impl JnoccioCallError {
    pub(crate) fn new(message: String) -> Self {
        Self {
            message,
            receipt: None,
            retryable: true,
            category: None,
        }
    }

    pub(crate) fn retryable(message: String) -> Self {
        Self::new(message)
    }

    pub(crate) fn non_retryable(message: String) -> Self {
        Self {
            message,
            receipt: None,
            retryable: false,
            category: None,
        }
    }

    pub(crate) fn with_receipt(message: String, receipt: AgentCallReceipt) -> Self {
        Self {
            message,
            receipt: Some(receipt),
            retryable: true,
            category: None,
        }
    }

    pub(crate) fn with_category(mut self, category: &str) -> Self {
        self.category = Some(category.to_string());
        self
    }

    pub(crate) fn with_context(mut self, context: String) -> Self {
        self.message = format!("{} ({context})", self.message);
        self
    }
}

pub(crate) struct JnoccioHttpRunner {
    pub(crate) client: reqwest::blocking::Client,
    pub(crate) endpoint: String,
    pub(crate) model: String,
    pub(crate) max_output_tokens: u64,
    pub(crate) phase_retries: usize,
    pub(crate) bearer_token: Option<String>,
    pub(crate) progress_jsonl: PathBuf,
    pub(crate) run_root: PathBuf,
    pub(crate) strict_production: bool,
    pub(crate) route_model_deny: Vec<RouteModelPolicy>,
    pub(crate) route_model_allow: Vec<RouteModelPolicy>,
}
