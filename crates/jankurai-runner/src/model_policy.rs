//! Model-routing policy for port workflow tasks.

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

/// Static model policy. Jnoccio can replace these ids at runtime.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelPolicy {
    /// Cheap reliable model id for routine work.
    pub routine_model: String,
    /// Power model id for hard synthesis and review.
    pub power_model: String,
    /// Allow power routing for routine roles.
    #[serde(default)]
    pub allow_power_for_routine_roles: bool,
}

impl Default for ModelPolicy {
    fn default() -> Self {
        Self {
            routine_model: "jnoccio/routine".to_string(),
            power_model: "jnoccio/power-winner".to_string(),
            allow_power_for_routine_roles: false,
        }
    }
}

impl ModelPolicy {
    /// Select a model id for a workflow task kind.
    pub fn select(&self, kind: ModelTaskKind) -> &str {
        if self.allow_power_for_routine_roles || kind.uses_power_model() {
            &self.power_model
        } else {
            &self.routine_model
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
    fn routine_uses_cheap_model_and_hard_tasks_use_power_model() {
        let policy = ModelPolicy::default();
        assert_eq!(policy.select(ModelTaskKind::Implement), "jnoccio/routine");
        assert_eq!(policy.select(ModelTaskKind::Verifier), "jnoccio/routine");
        assert_eq!(
            policy.select(ModelTaskKind::StageBrainstorm),
            "jnoccio/routine"
        );
        assert_eq!(
            policy.select(ModelTaskKind::Healing),
            "jnoccio/power-winner"
        );
        assert_eq!(policy.select(ModelTaskKind::Review), "jnoccio/power-winner");
        assert_eq!(
            policy.select(ModelTaskKind::StageCritique),
            "jnoccio/power-winner"
        );
        assert_eq!(policy.select(ModelTaskKind::Critic), "jnoccio/power-winner");
        assert_eq!(
            policy.select(ModelTaskKind::HardEscalation),
            "jnoccio/power-winner"
        );
        assert_eq!(
            policy.select(ModelTaskKind::MetaJudge),
            "jnoccio/power-winner"
        );
        assert_eq!(
            policy.select(ModelTaskKind::RedTeam),
            "jnoccio/power-winner"
        );
        assert_eq!(
            policy.select(ModelTaskKind::HeroGenerate),
            "jnoccio/routine"
        );
        assert_eq!(policy.select(ModelTaskKind::JudgePatch), "jnoccio/routine");
    }
}
