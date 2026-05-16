//! Sampling parameter rules (per-family temperature/top_p/top_k tables).

use jekko_core::provider::Model;

/// Sampling parameters: `(temperature, topP, topK)`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SamplingParams {
    /// Sampling temperature.
    pub temperature: Option<f64>,
    /// Top-P nucleus sampling.
    pub top_p: Option<f64>,
    /// Top-K sampling.
    pub top_k: Option<u32>,
}

/// Mirror of `samplingParams(...)` in `transform-variants.ts`.
pub fn sampling_params(model: &Model) -> SamplingParams {
    let id = model.id.to_string().to_lowercase();
    if id.contains("qwen") {
        return SamplingParams {
            temperature: Some(0.55),
            top_p: Some(1.0),
            top_k: None,
        };
    }
    if id.contains("gemini") {
        return SamplingParams {
            temperature: Some(1.0),
            top_p: Some(0.95),
            top_k: Some(64),
        };
    }
    if id.contains("glm-4.6") || id.contains("glm-4.7") {
        return SamplingParams {
            temperature: Some(1.0),
            top_p: None,
            top_k: None,
        };
    }
    if id.contains("minimax-m2") {
        let top_k = if ["m2.", "m25", "m21"].iter().any(|s| id.contains(s)) {
            40
        } else {
            20
        };
        return SamplingParams {
            temperature: Some(1.0),
            top_p: Some(0.95),
            top_k: Some(top_k),
        };
    }
    if id.contains("kimi-k2.5") || id.contains("kimi-k2p5") || id.contains("kimi-k2-5") {
        return SamplingParams {
            temperature: Some(1.0),
            top_p: Some(0.95),
            top_k: None,
        };
    }
    if id.contains("kimi-k2") {
        let temperature = if ["thinking", "k2.", "k2p"].iter().any(|s| id.contains(s)) {
            1.0
        } else {
            0.6
        };
        return SamplingParams {
            temperature: Some(temperature),
            top_p: None,
            top_k: None,
        };
    }
    SamplingParams::default()
}

/// Sampling temperature for a model.
pub fn temperature(model: &Model) -> Option<f64> {
    sampling_params(model).temperature
}

/// Top-P for a model.
pub fn top_p(model: &Model) -> Option<f64> {
    sampling_params(model).top_p
}

/// Top-K for a model.
pub fn top_k(model: &Model) -> Option<u32> {
    sampling_params(model).top_k
}
