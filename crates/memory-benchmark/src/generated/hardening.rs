//! Topic-hardening suite — repeated-query convergence.
//!
//! Each case ingests a small set of reinforcement events and asks the
//! same query repeatedly (synthesized as one consolidated query with the
//! corresponding ingest stream). The adapter is rewarded for surfacing a
//! convergent answer that includes the canonical event.

use crate::case::{BenchCase, CaseOracle, EpisodeStep, OracleKind, Split};
use crate::{
    ClaimModality, Domain, Event, EventKind, FixtureBlock, PrivacyClass, PublicBench, Query,
    QueryIntent, Source, TemporalLens,
};

use super::seed::SeedRng;

pub struct HardeningConfig {
    pub benchmark_version: &'static str,
    pub seed_label: String,
    pub fixture_count: usize,
}

pub fn generate_hardening_suite(config: &HardeningConfig) -> Vec<BenchCase> {
    let mut rng = SeedRng::from_label(&format!(
        "{}:hardening:{}",
        config.benchmark_version, config.seed_label
    ));
    let mut cases = Vec::with_capacity(config.fixture_count);
    for idx in 0..config.fixture_count {
        cases.push(case_at(&mut rng, idx));
    }
    cases
}

fn source(id: &str) -> Source {
    Source {
        uri: format!("synthetic://{}", id),
        citation: format!("Synthetic source {}", id),
        quality: 0.95,
    }
}

fn event(id: &str, subject: &str, body: String, tx: String) -> Event {
    Event {
        id: id.to_string(),
        kind: EventKind::Claim,
        subject: subject.to_string(),
        body,
        sources: vec![source(id)],
        valid_from: Some("2026-01-01T00:00:00Z".to_string()),
        valid_to: None,
        tx_time: tx,
        event_time: None,
        observation_time: None,
        review_time: None,
        policy_time: None,
        dependencies: Vec::new(),
        supersedes: Vec::new(),
        contradicts: Vec::new(),
        derived_from: Vec::new(),
        namespace: Some("public-hardening".to_string()),
        privacy_class: PrivacyClass::Public,
        claim_modality: Some(ClaimModality::FormallyVerified),
        tags: Vec::new(),
    }
}

fn case_at(rng: &mut SeedRng, idx: usize) -> BenchCase {
    let subject = format!("topic H{}", idx);
    let canonical_id = format!("g-{:05}-h-canon", idx);
    let body = format!(
        "Canonical fact about {}: stable value {}.",
        subject,
        rng.range(1, 200)
    );
    let canonical = event(
        &canonical_id,
        &subject,
        body,
        format!("2026-08-{:02}T00:00:00Z", idx % 28 + 1),
    );
    let mut events = vec![canonical];
    // Five reinforcement events emphasising the canonical subject.
    for k in 0..5usize {
        let rid = format!("g-{:05}-h-r{}", idx, k);
        let rev = event(
            &rid,
            &subject,
            format!(
                "Reinforcement {} re-states the canonical fact about {}.",
                k, subject
            ),
            format!("2026-08-{:02}T{:02}:00:00Z", idx % 28 + 1, (k + 1) * 2),
        );
        events.push(rev);
    }
    BenchCase {
        id: format!("{}-{:05}", Split::PublicHardening.name(), idx),
        block: FixtureBlock::RecallCurrent,
        domain: Domain::Science,
        pathologies: vec![],
        public_bench: vec![PublicBench::MemoryAgentBenchLongRange],
        events,
        steps: vec![EpisodeStep::Teach, EpisodeStep::Query],
        query: Some(Query {
            text: format!("What is the canonical fact about {}?", subject),
            intent: QueryIntent::Recall,
            mentions: vec![subject.clone()],
            token_budget: 2048,
        }),
        lens: TemporalLens::Current,
        world_time: None,
        tx_time: None,
        oracle: CaseOracle {
            kind: OracleKind::Hardening,
            must_include: vec![canonical_id],
            must_exclude: vec![],
            must_contain: vec![subject.clone()],
            must_not_contain: vec![],
            required_warnings: vec![],
            expected_answer: None,
            max_used_ids: 8,
            max_context_tokens: 2048,
        },
    }
}
