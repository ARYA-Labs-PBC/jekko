//! Provider catalog, request transforms, and LLM streaming for the Jekko
//! Rust port.
//!
//! Ports the following TypeScript surface from `packages/jekko/src/provider/`:
//!
//! - `provider-schema.ts`, `provider-runtime.ts` -> [`catalog`]
//! - `model-routing/recommendations.ts` -> [`routing`]
//! - `model-setup/model-keys*.ts` -> [`setup`]
//! - `transform.ts`, `transform-*.ts` -> [`transform`]
//! - `session/llm.ts` -> [`stream`] + [`adapter`] + [`providers`]
//!
//! The crate exposes a [`ProviderAdapter`] trait implemented by per-provider
//! HTTP adapters and an internal SSE parser that decodes provider events into
//! the canonical [`ProviderEvent`] shape.
#![deny(rust_2018_idioms)]
#![warn(missing_docs)]

pub mod adapter;
pub mod catalog;
pub mod error;
pub mod key_pool;
pub mod providers;
pub mod routing;
pub mod setup;
pub mod stream;
pub mod transform;

pub use adapter::{ProviderAdapter, ProviderRequest, ProviderStream};
pub use catalog::{
    catalog_is_locked_provider, ProviderCatalog, ProviderCatalogEntry, ProviderModelKey,
};
pub use error::{ProviderError, ProviderResult};
pub use routing::{is_known_recommended_model, recommended_model_id, RECOMMENDED_MODELS};
pub use stream::{
    parse_sse_block, AggregatedToolCall, ProviderCapabilities as AdapterCapabilities,
    ProviderEvent, ProviderEventKind, SseDecoder, SseFrame, ToolCallAggregator,
};

// Re-export the canonical jekko-core types used in this crate's public API so
// downstream callers don't have to depend on `jekko-core` directly. The
// `Catalog*` aliases keep the call sites short and disambiguate from the
// adapter-level [`AdapterCapabilities`] type above.
pub use jekko_core::provider::{
    Model as CatalogModel, ProviderApiInfo, ProviderCapabilities as CatalogModelCapabilities,
    ProviderInfo, ProviderListResult,
};
