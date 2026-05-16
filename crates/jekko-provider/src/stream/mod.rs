//! SSE parsing and canonical provider events.
//!
//! The stream module is provider-agnostic: it decodes Server-Sent Event
//! frames from any HTTP body and yields a [`ProviderEvent`] stream that the
//! runtime layer consumes. Per-provider [`crate::providers`] adapters handle
//! the mapping from raw SSE JSON payloads to [`ProviderEventKind`].
mod aggregator;
mod events;
mod sse;

pub use aggregator::{AggregatedToolCall, ToolCallAggregator};
pub use events::{ProviderEvent, ProviderEventKind};
pub use sse::{parse_sse_block, ProviderCapabilities, SseDecoder, SseFrame};

#[cfg(test)]
mod tests;
