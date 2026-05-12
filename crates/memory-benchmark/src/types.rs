use crate::result::RecallResult;

#[derive(Debug, Clone)]
pub struct Event {
    pub id: String,
    pub kind: EventKind,
    pub subject: String,
    pub body: String,
    pub sources: Vec<Source>,
    pub valid_from: Option<String>,
    pub valid_to: Option<String>,
    pub tx_time: String,
    pub event_time: Option<String>,
    pub observation_time: Option<String>,
    pub review_time: Option<String>,
    pub policy_time: Option<String>,
    pub dependencies: Vec<String>,
    pub supersedes: Vec<String>,
    pub contradicts: Vec<String>,
    pub derived_from: Vec<String>,
    pub namespace: Option<String>,
    pub privacy_class: PrivacyClass,
    pub claim_modality: Option<ClaimModality>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventKind {
    Observation,
    Claim,
    Equation,
    Theorem,
    Skill,
    Resource,
    Dataset,
    Experiment,
    Hypothesis,
    Counterexample,
    Lesson,
    Question,
    VaultCanary,
    SchemaMigration,
    Supersede {
        target_event_id: String,
        reason: String,
    },
    Feedback,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrivacyClass {
    Public,
    Internal,
    Confidential,
    Secret,
    Vault,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClaimModality {
    Observed,
    AssertedBySource,
    InferredByAgent,
    HumanApproved,
    FormallyVerified,
}

#[derive(Debug, Clone)]
pub struct Source {
    pub uri: String,
    pub citation: String,
    pub quality: f32,
}

#[derive(Debug, Clone)]
pub struct Query {
    pub text: String,
    pub intent: QueryIntent,
    pub mentions: Vec<String>,
    pub token_budget: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryIntent {
    Fact,
    Equation,
    Theorem,
    Citation,
    Coref,
    Procedure,
    Workflow,
    Contradiction,
    Recall,
    HistoryAt,
    HistoryAsOf,
    Forget,
    Mixed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Warning {
    Superseded,
    Contradicted,
    LowConfidence,
    Redacted,
    CausalMaskApplied,
    UntrustedInstructionLikeContent,
    SkeptikSurfaced,
    UnitMismatch,
    SchemaMigrated,
    DependencyInvalidated,
    CitationUnsupported,
    CitationBloated,
    CompressionDrift,
    PrivacyTransformBlocked,
    UnsafeToolRefused,
    Abstained,
    BeliefTimeApplied,
}

#[derive(Debug, Clone)]
pub struct Receipt {
    pub event_id: Option<String>,
    pub mutation_id: String,
    pub at: String,
    pub previous_hash: String,
    pub hash: String,
}

#[derive(Debug, Clone)]
pub struct Tombstone {
    pub memory_id: String,
    pub reason: String,
    pub deletion_proof: String,
    pub deleted_at: String,
}

#[derive(Debug, Clone)]
pub struct Feedback {
    pub outcome: Outcome,
    pub used: Vec<String>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    TaskSuccess,
    TaskFailure,
    Verified,
    Falsified,
    Ignored,
}

pub trait MemorySystem {
    fn name(&self) -> &'static str;
    fn observe(&mut self, event: &Event) -> Receipt;
    fn recall(&mut self, query: &Query) -> RecallResult;
    fn recall_at(&mut self, query: &Query, world_time: &str) -> RecallResult;
    fn recall_as_of(&mut self, query: &Query, tx_time: &str) -> RecallResult;

    fn belief_as_of(&mut self, query: &Query, tx_time: &str) -> RecallResult {
        self.recall_as_of(query, tx_time)
    }

    fn build_context(&mut self, query: &Query, budget_tokens: u32) -> RecallResult {
        let mut q = query.clone();
        q.token_budget = budget_tokens;
        self.recall(&q)
    }

    fn feedback(&mut self, pack_id: &str, outcome: &Feedback) -> Receipt;
    fn forget(&mut self, memory_id: &str, reason: &str) -> Tombstone;
    fn rebuild(&mut self) -> Receipt;
    fn export_state_hash(&self) -> String;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Domain {
    Science,
    Math,
    English,
    Privacy,
    Procedural,
}

impl Domain {
    pub const ALL: &'static [Domain] = &[
        Domain::Science,
        Domain::Math,
        Domain::English,
        Domain::Privacy,
        Domain::Procedural,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Domain::Science => "science",
            Domain::Math => "math",
            Domain::English => "english",
            Domain::Privacy => "privacy",
            Domain::Procedural => "procedural",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Pathology {
    FutureLeak,
    SupersededClaim,
    PrivacyLeak,
    UnitMismatch,
    SourceHallucination,
    CoreferenceError,
    CompressionDrift,
    RankingIgnored,
    SkepticBlindness,
    ModalityConfusion,
}

impl Pathology {
    pub const ALL: &'static [Pathology] = &[
        Pathology::FutureLeak,
        Pathology::SupersededClaim,
        Pathology::PrivacyLeak,
        Pathology::UnitMismatch,
        Pathology::SourceHallucination,
        Pathology::CoreferenceError,
        Pathology::CompressionDrift,
        Pathology::RankingIgnored,
        Pathology::SkepticBlindness,
        Pathology::ModalityConfusion,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Pathology::FutureLeak => "future_leak",
            Pathology::SupersededClaim => "superseded_claim",
            Pathology::PrivacyLeak => "privacy_leak",
            Pathology::UnitMismatch => "unit_mismatch",
            Pathology::SourceHallucination => "source_hallucination",
            Pathology::CoreferenceError => "coreference_error",
            Pathology::CompressionDrift => "compression_drift",
            Pathology::RankingIgnored => "ranking_ignored",
            Pathology::SkepticBlindness => "skeptic_blindness",
            Pathology::ModalityConfusion => "modality_confusion",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixtureBlock {
    Ingest,
    RecallCurrent,
    RecallAt,
    RecallAsOf,
    Contradiction,
    Procedural,
    Feedback,
    Determinism,
}

impl FixtureBlock {
    pub fn name(self) -> &'static str {
        match self {
            FixtureBlock::Ingest => "ingest",
            FixtureBlock::RecallCurrent => "recall_current",
            FixtureBlock::RecallAt => "recall_at",
            FixtureBlock::RecallAsOf => "recall_as_of",
            FixtureBlock::Contradiction => "contradiction",
            FixtureBlock::Procedural => "procedural",
            FixtureBlock::Feedback => "feedback",
            FixtureBlock::Determinism => "determinism",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PublicBench {
    LongMemEvalInfoExtraction,
    LongMemEvalMultiSession,
    LongMemEvalTemporal,
    LongMemEvalKnowledgeUpdate,
    LongMemEvalAbstention,
    LoCoMoOverall,
    LoCoMoTemporal,
    LoCoMoPersonal,
    MemoryAgentBenchRetrieval,
    MemoryAgentBenchLearning,
    MemoryAgentBenchLongRange,
    MemoryAgentBenchForgetting,
    ScienceMemoryBench,
    EquationRecallBench,
    TemporalContradictionBench,
    DatasetReproBench,
    SkillReliabilityBench,
    MemoryPoisoningBench,
    ContextPackCitationBench,
    IndexRebuildDeterminismBench,
}

impl PublicBench {
    pub fn name(self) -> &'static str {
        match self {
            PublicBench::LongMemEvalInfoExtraction => "LongMemEval/InfoExtraction",
            PublicBench::LongMemEvalMultiSession => "LongMemEval/MultiSession",
            PublicBench::LongMemEvalTemporal => "LongMemEval/Temporal",
            PublicBench::LongMemEvalKnowledgeUpdate => "LongMemEval/KnowledgeUpdate",
            PublicBench::LongMemEvalAbstention => "LongMemEval/Abstention",
            PublicBench::LoCoMoOverall => "LoCoMo/Overall",
            PublicBench::LoCoMoTemporal => "LoCoMo/Temporal",
            PublicBench::LoCoMoPersonal => "LoCoMo/Personal",
            PublicBench::MemoryAgentBenchRetrieval => "MemoryAgentBench/Retrieval",
            PublicBench::MemoryAgentBenchLearning => "MemoryAgentBench/Learning",
            PublicBench::MemoryAgentBenchLongRange => "MemoryAgentBench/LongRange",
            PublicBench::MemoryAgentBenchForgetting => "MemoryAgentBench/Forgetting",
            PublicBench::ScienceMemoryBench => "ScienceMemoryBench",
            PublicBench::EquationRecallBench => "EquationRecallBench",
            PublicBench::TemporalContradictionBench => "TemporalContradictionBench",
            PublicBench::DatasetReproBench => "DatasetReproBench",
            PublicBench::SkillReliabilityBench => "SkillReliabilityBench",
            PublicBench::MemoryPoisoningBench => "MemoryPoisoningBench",
            PublicBench::ContextPackCitationBench => "ContextPackCitationBench",
            PublicBench::IndexRebuildDeterminismBench => "IndexRebuildDeterminismBench",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemporalLens {
    Current,
    At,
    AsOf,
    AtAsOf,
    NoQuery,
}
