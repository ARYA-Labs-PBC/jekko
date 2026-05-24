//! Provider-neutral model-routing policy for ZYAL workflow tasks.

use serde::{Deserialize, Serialize};

/// Port workflow model task kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelTaskKind {
    /// Request framing.
    Frame,
    /// Stage brainstorming.
    StageBrainstorm,
    /// Stage critique.
    StageCritique,
    /// Stage reduction/finalization.
    StageReduce,
    /// Phase brainstorming.
    PhaseBrainstorm,
    /// Hypothesis generation.
    Hypothesis,
    /// Critic pass.
    Critic,
    /// Executable/source verifier.
    Verifier,
    /// Durable memory curation.
    MemoryCurate,
    /// Parity case/report generation.
    ParityGenerate,
    /// Performance parity closure.
    PerfClose,
    /// Hard escalation.
    HardEscalation,
    /// Routine implementation.
    Implement,
    /// Phase finalization.
    PhaseFinalize,
    /// Stuck debugging.
    StuckDebug,
    /// Cross-phase healing.
    Healing,
    /// Performance gap analysis.
    PerfGap,
    /// Reviewer pass.
    Review,
    /// Hero candidate generation.
    HeroGenerate,
    /// Judge prompt patching.
    JudgePatch,
    /// Literature synthesis.
    LiteratureSynthesis,
    /// Adversarial red-team pass.
    RedTeam,
    /// Meta-judge reduction.
    MetaJudge,
    /// Verified knowledge curation.
    KnowledgeCurate,
}

/// One provider-neutral route. Both fields are optional: an empty route lets
/// the runtime choose the provider and model from its configured defaults.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ModelRoute {
    /// Structured route form.
    Record(ModelRouteRecord),
    /// Compact `provider/model` or `model` string form.
    Compact(String),
}

impl Default for ModelRoute {
    fn default() -> Self {
        Self::Record(ModelRouteRecord::default())
    }
}

impl ModelRoute {
    /// Return this route as a normalized record.
    pub fn normalized(&self) -> ModelRouteRecord {
        match self {
            Self::Record(record) => record.clone(),
            Self::Compact(value) => ModelRouteRecord::from_compact(value),
        }
    }
}

/// Structured route record.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelRouteRecord {
    /// Optional provider id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    /// Optional model id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

impl ModelRouteRecord {
    fn from_compact(value: &str) -> Self {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Self::default();
        }
        match trimmed.split_once('/') {
            Some((provider, model)) if !provider.is_empty() && !model.is_empty() => Self {
                provider: Some(provider.to_string()),
                model: Some(model.to_string()),
            },
            _ => Self {
                provider: None,
                model: Some(trimmed.to_string()),
            },
        }
    }

    /// Whether neither provider nor model was explicitly configured.
    pub fn is_empty(&self) -> bool {
        self.provider.is_none() && self.model.is_none()
    }
}

/// Static route policy. Defaults are provider-neutral.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelPolicy {
    /// Routine route.
    #[serde(default)]
    pub routine: ModelRoute,
    /// Power route for hard synthesis and review.
    #[serde(default)]
    pub power: ModelRoute,
    /// Critic route.
    #[serde(default)]
    pub critic: ModelRoute,
    /// Verifier route.
    #[serde(default)]
    pub verifier: ModelRoute,
    /// Meta-judge route.
    #[serde(default)]
    pub meta_judge: ModelRoute,
    /// Reproducer route.
    #[serde(default)]
    pub reproducer: ModelRoute,
    /// Memory-curator route.
    #[serde(default)]
    pub memory_curator: ModelRoute,
    /// Allow power routing for routine roles.
    #[serde(default)]
    pub allow_power_for_routine_roles: bool,
}

impl ModelPolicy {
    /// Select a route for a workflow task kind.
    pub fn select(&self, kind: ModelTaskKind) -> ModelRouteRecord {
        let power = || self.power.normalized();
        let inherit_power_when_empty = |route: ModelRouteRecord| {
            if route.is_empty() {
                power()
            } else {
                route
            }
        };
        match kind {
            ModelTaskKind::Critic | ModelTaskKind::RedTeam => {
                inherit_power_when_empty(self.critic.normalized())
            }
            ModelTaskKind::Verifier => self.verifier.normalized(),
            ModelTaskKind::MetaJudge => inherit_power_when_empty(self.meta_judge.normalized()),
            ModelTaskKind::MemoryCurate | ModelTaskKind::KnowledgeCurate => {
                self.memory_curator.normalized()
            }
            ModelTaskKind::ParityGenerate => self.reproducer.normalized(),
            _ => {
                if self.allow_power_for_routine_roles || kind.uses_power_model() {
                    self.power.normalized()
                } else {
                    self.routine.normalized()
                }
            }
        }
    }
}

impl ModelTaskKind {
    /// Whether this task routes to the power model by default.
    pub fn uses_power_model(self) -> bool {
        match self {
            ModelTaskKind::StageReduce
            | ModelTaskKind::StageCritique
            | ModelTaskKind::Critic
            | ModelTaskKind::PerfClose
            | ModelTaskKind::HardEscalation
            | ModelTaskKind::PhaseFinalize
            | ModelTaskKind::StuckDebug
            | ModelTaskKind::Healing
            | ModelTaskKind::PerfGap
            | ModelTaskKind::Review
            | ModelTaskKind::RedTeam
            | ModelTaskKind::MetaJudge => true,
            ModelTaskKind::Frame
            | ModelTaskKind::StageBrainstorm
            | ModelTaskKind::PhaseBrainstorm
            | ModelTaskKind::Hypothesis
            | ModelTaskKind::Verifier
            | ModelTaskKind::MemoryCurate
            | ModelTaskKind::ParityGenerate
            | ModelTaskKind::Implement
            | ModelTaskKind::HeroGenerate
            | ModelTaskKind::JudgePatch
            | ModelTaskKind::LiteratureSynthesis
            | ModelTaskKind::KnowledgeCurate => false,
        }
    }
}

/// One model outcome receipt.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelOutcome {
    /// Task id.
    pub task_id: String,
    /// Model id.
    pub model_id: String,
    /// Cost in USD.
    pub cost_usd: f64,
    /// Latency in milliseconds.
    pub latency_ms: u64,
    /// Whether the task succeeded.
    pub success: bool,
    /// Optional reviewer score.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reviewer_score: Option<f64>,
    /// Whether this outcome became a winner.
    pub winner: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_policy_default_is_provider_neutral() {
        let policy = ModelPolicy::default();
        for kind in [
            ModelTaskKind::Implement,
            ModelTaskKind::Verifier,
            ModelTaskKind::StageBrainstorm,
            ModelTaskKind::Healing,
            ModelTaskKind::Review,
            ModelTaskKind::StageCritique,
            ModelTaskKind::Critic,
            ModelTaskKind::HardEscalation,
            ModelTaskKind::MetaJudge,
            ModelTaskKind::RedTeam,
            ModelTaskKind::HeroGenerate,
            ModelTaskKind::JudgePatch,
        ] {
            let route = policy.select(kind);
            assert_eq!(route.provider, None);
            assert_eq!(route.model, None);
        }
    }

    #[test]
    fn compact_route_string_splits_provider_and_model() {
        let policy = ModelPolicy {
            routine: ModelRoute::Compact("openrouter/qwen".to_string()),
            ..ModelPolicy::default()
        };
        let route = policy.select(ModelTaskKind::Implement);
        assert_eq!(route.provider.as_deref(), Some("openrouter"));
        assert_eq!(route.model.as_deref(), Some("qwen"));
    }
}
