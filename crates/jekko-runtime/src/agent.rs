//! Agent runtime boundary.
//!
//! This module owns the one-shot execution boundary:
//!
//! - prompt parsing and session bookkeeping
//! - provider/model selection
//! - provider-backed streaming of the assistant turn
//! - assistant message persistence
//!
//! The execution layer is intentionally injectable so tests can run with a
//! deterministic fake executor while the default runtime uses the real
//! provider adapters.
//!
//! The implementation is split per-seam under [`agent`](self) submodules
//! ([`types`] for payloads, [`executor`] for the streaming loop, [`oneshot`]
//! for `Runtime::run_oneshot` and helpers, [`provider`] for catalog / adapter
//! lookups). Public types and helpers are re-exported here so the original
//! `jekko_runtime::agent::*` import paths continue to work unchanged.

pub mod executor;
pub mod oneshot;
pub mod provider;
pub mod types;

#[cfg(test)]
mod tests;

pub use executor::{
    mock_assistant_stream, mock_assistant_text, mock_llm_enabled, AgentExecutor,
    ProviderAdapterResolver, ProviderAgentExecutor, MOCK_LLM_ENV, MOCK_RESPONSE_DEFAULT,
    MOCK_RESPONSE_ENV,
};
pub use types::{AgentTurnRequest, AgentTurnResult, RunRequest, RunResult};
