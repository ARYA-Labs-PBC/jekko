//! Sampling parameters and per-model variant tables.
//!
//! Ported from:
//! - `packages/jekko/src/provider/transform-variants.ts`
//! - `transform-variants-core.ts`
//! - `transform-variants-logic.ts`

use jekko_core::provider::Model;
use serde_json::{Map, Value};

mod dispatch;
mod efforts;
mod providers;
mod sampling;

#[cfg(test)]
mod tests;

pub use sampling::{sampling_params, temperature, top_k, top_p, SamplingParams};

use dispatch::build_model_variants;

/// Per-model variant table (e.g. `{ low: { reasoningEffort: "low" }, … }`).
///
/// Mirror of `variants(...)` in `transform-variants.ts`.
pub fn variants(model: &Model) -> Map<String, Value> {
    build_model_variants(model)
}
