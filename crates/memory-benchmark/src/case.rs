use crate::{Domain, Event, FixtureBlock, Pathology, PublicBench, Query, TemporalLens};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Split {
    PublicSmoke,
    PublicGenerated,
    PrivateGenerated,
    Stress,
    RealPapers,
    /// Multi-hop chain reasoning suite (v3 north-star).
    PublicCompounding,
    /// 5-timestep repeated-query convergence suite (v3 north-star).
    PublicHardening,
}

impl Split {
    pub fn name(self) -> &'static str {
        match self {
            Split::PublicSmoke => "public-smoke",
            Split::PublicGenerated => "public-generated",
            Split::PrivateGenerated => "private-generated",
            Split::Stress => "stress",
            Split::RealPapers => "real-papers",
            Split::PublicCompounding => "public-compounding",
            Split::PublicHardening => "public-hardening",
        }
    }
}

#[derive(Debug, Clone)]
pub struct SuiteConfig {
    pub benchmark_version: &'static str,
    pub split: Split,
    pub seed_label: String,
    pub fixture_count: usize,
    pub difficulty: u8,
    pub context_budget: u32,
    pub paper_bank_path: Option<String>,
    pub qbank_top_n: usize,
    pub qbank_selection_path: Option<String>,
    pub qbank_topic_focus: Option<String>,
    pub safe_window_tokens: u32,
}

impl Default for SuiteConfig {
    fn default() -> Self {
        Self {
            benchmark_version: "memory-benchmark-v2",
            split: Split::PublicSmoke,
            seed_label: "public-dev-0001".to_string(),
            fixture_count: 100,
            difficulty: 2,
            context_budget: 4096,
            paper_bank_path: None,
            qbank_top_n: 100,
            qbank_selection_path: None,
            qbank_topic_focus: None,
            safe_window_tokens: 128000,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BenchCase {
    pub id: String,
    pub block: FixtureBlock,
    pub domain: Domain,
    pub pathologies: Vec<Pathology>,
    pub public_bench: Vec<PublicBench>,
    pub events: Vec<Event>,
    pub steps: Vec<EpisodeStep>,
    pub query: Option<Query>,
    pub lens: TemporalLens,
    pub world_time: Option<String>,
    pub tx_time: Option<String>,
    pub oracle: CaseOracle,
}

#[derive(Debug, Clone)]
pub struct HardeningCase {
    pub id: String,
    pub subject: String,
    pub base_events: Vec<Event>,
    pub reinforcements: Vec<Event>,
    pub query: Query,
    pub oracle: CaseOracle,
}

#[derive(Debug, Clone)]
pub struct CompoundCase {
    pub id: String,
    pub block: FixtureBlock,
    pub domain: Domain,
    pub events: Vec<Event>,
    pub queries: Vec<CompoundQuery>,
}

#[derive(Debug, Clone)]
pub struct CompoundQuery {
    pub label: String,
    pub query: Query,
    pub oracle: CaseOracle,
    pub hop_depth: u8,
    pub depth_weight: f32,
    pub control: bool,
}

#[derive(Debug, Clone)]
pub enum EpisodeStep {
    Teach,
    Distract,
    Compress,
    Mutate,
    Query,
    Attack,
    Rebuild,
    MetamorphicReplay,
}

#[derive(Debug, Clone)]
pub struct CaseOracle {
    pub kind: OracleKind,
    pub must_include: Vec<String>,
    pub must_exclude: Vec<String>,
    pub must_contain: Vec<String>,
    pub must_not_contain: Vec<String>,
    pub required_warnings: Vec<String>,
    pub expected_answer: Option<String>,
    pub max_used_ids: usize,
    pub max_context_tokens: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OracleKind {
    ExactText,
    UnitAlgebra,
    TheoremDag,
    Temporal,
    Privacy,
    Provenance,
    Workflow,
    Metamorphic,
    /// Multi-hop chain — used by `Split::PublicCompounding`.
    Compounding,
    /// Repeated-query convergence — used by `Split::PublicHardening`.
    Hardening,
}

impl From<&crate::fixture::Fixture> for BenchCase {
    fn from(f: &crate::fixture::Fixture) -> Self {
        let query = f.query_text.map(|text| Query {
            text: text.to_string(),
            intent: f.query_intent,
            mentions: f.query_mentions.iter().map(|s| s.to_string()).collect(),
            token_budget: 4096,
        });
        BenchCase {
            id: format!("t0-{:03}", f.id),
            block: f.block,
            domain: f.domain,
            pathologies: f.pathologies.to_vec(),
            public_bench: f.public_bench.to_vec(),
            events: Vec::new(),
            steps: vec![EpisodeStep::Teach, EpisodeStep::Query],
            query,
            lens: f.lens,
            world_time: f.world_time.map(str::to_string),
            tx_time: f.tx_time.map(str::to_string),
            oracle: CaseOracle {
                kind: OracleKind::ExactText,
                must_include: f
                    .expected
                    .must_include
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                must_exclude: f
                    .expected
                    .must_exclude
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                must_contain: f
                    .expected
                    .must_contain
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                must_not_contain: f
                    .expected
                    .must_not_contain
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                required_warnings: f
                    .expected
                    .required_warnings
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                expected_answer: None,
                max_used_ids: 8,
                max_context_tokens: 4096,
            },
        }
    }
}
