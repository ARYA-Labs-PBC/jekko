//! Per-provider adapters.
//!
//! Each adapter is a thin wrapper around the transform layer plus a streaming
//! HTTP request. Adapters are kept small and individually testable; shared
//! HTTP plumbing lives in [`shared`].

pub mod anthropic;
pub mod dummy_agent_llm;
pub mod jekko;
pub mod jnoccio;
pub mod litellm;
pub mod openai;
pub mod openrouter;
pub mod shared;

pub use anthropic::AnthropicAdapter;
pub use dummy_agent_llm::DummyAgentLlmAdapter;
pub use jekko::JekkoAdapter;
pub use jnoccio::JNoccioAdapter;
pub use litellm::LiteLlmAdapter;
pub use openai::OpenAiAdapter;
pub use openrouter::OpenRouterAdapter;
