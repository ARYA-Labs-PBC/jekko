use crate::{Domain, Event, FixtureBlock, Pathology, PublicBench, Query, TemporalLens};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Split {
    PublicSmoke,
    PublicGenerated,
    PrivateGenerated,
    Stress,
}

impl Split {
    pub fn name(self) -> &'static str {
        match self {
            Split::PublicSmoke => "public-smoke",
            Split::PublicGenerated => "public-generated",
            Split::PrivateGenerated => "private-generated",
            Split::Stress => "stress",
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
