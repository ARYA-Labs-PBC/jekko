use super::agent_json::{
    parse_agent_json, validate_generator_output, validate_grading_output, validate_testing_output,
    validate_verification_output,
};
use super::{
    canonical_paper_text, canonicalize_paper, collect_json_files, ensure_bank_layout,
    finalize_challenge, manifest_hash, pack_context, production_acceptance_errors, read_papers,
    sha256_hex, token_estimate, validate_full_text_paper, write_json_pretty, AcceptanceMetrics,
    AcceptanceRecord, AgentCallReceipt, AgentFailure, AnswerAttempt, AnswerKey, ArtifactProvenance,
    CandidateAttemptReceipt, ChallengeRecord, ContextPackProvenance, FinalPaperChallengeArtifact,
    GeneratorAgentOutput, GeneratorTrial, GradingAgentOutput, GradingTrial, JudgeTrial,
    LicenseRecord, ModelDecision, ModelTrial, PaperRecord, PaperSection, RouteMetadata,
    SourcePublication, SupportQuote, SupportRef, TestingAgentOutput, TestingTrial, TokenUsage,
    VerificationAgentOutput, VerificationTrial, FINAL_PAPER_CHALLENGE_SCHEMA_VERSION,
    HARD_MAX_TESTER_CORRECT_RATE, MIN_SUCCESSFUL_GRADERS, MIN_SUCCESSFUL_VERIFIERS,
    PAPER_SCHEMA_VERSION, PAPER_TOURNAMENT_SCHEMA_VERSION, PRODUCTION_CHALLENGE_SCHEMA_VERSION,
    PRODUCTION_MANIFEST_SCHEMA_VERSION, QBANK_REDUCER_VERSION,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::BTreeMap;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub enum AgentRunnerMode {
    Mock,
    Jnoccio,
}

impl AgentRunnerMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Mock => "mock_smoke",
            Self::Jnoccio => "live_jnoccio",
        }
    }

    pub fn is_mock(&self) -> bool {
        matches!(self, Self::Mock)
    }
}

#[derive(Debug, Clone)]
pub struct BuildPaperTournamentConfig {
    pub bank: PathBuf,
    pub run_root: PathBuf,
    pub target_accepted: usize,
    pub candidate_papers: usize,
    pub generators: usize,
    pub verifiers: usize,
    pub testers: usize,
    pub graders: usize,
    pub min_successful_generators: usize,
    pub min_successful_verifiers: usize,
    pub min_successful_testers: usize,
    pub min_successful_graders: usize,
    pub distractor_papers: usize,
    pub strict_production: bool,
    pub agent_runner: AgentRunnerMode,
    pub jnoccio_base_url: Option<String>,
    pub jnoccio_model: Option<String>,
    pub jnoccio_max_output_tokens: u64,
    pub jnoccio_request_timeout_seconds: u64,
    pub paper_timeout_seconds: u64,
    pub phase_retries: usize,
    pub generator_pool_target: usize,
    pub max_question_alternates_per_paper: usize,
    pub blind_prescreen_testers: usize,
    pub blind_prescreen_max_correct_rate: f64,
    pub min_support_quote_score: i32,
    pub hard_distractors: bool,
    pub mask_blind_context_metadata: bool,
    pub route_model_deny: Vec<RouteModelPolicy>,
    pub route_model_allow: Vec<RouteModelPolicy>,
    pub write_rejection_analysis: bool,
    pub progress_jsonl: Option<PathBuf>,
    pub candidate_manifest: Option<PathBuf>,
    pub resume: bool,
    pub allow_mock_smoke: bool,
    pub mock_agents: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct RouteModelPolicy {
    pub phase: String,
    pub pattern: String,
}

#[derive(Debug, Clone)]
pub struct BuildPaperTournamentSummary {
    pub generated: usize,
    pub accepted: usize,
    pub rejected: usize,
    pub failed: usize,
    pub run_root: PathBuf,
    pub sample_accepted_artifact: Option<PathBuf>,
    pub sample_rejected_artifact: Option<PathBuf>,
    pub reduce_report: PathBuf,
}

#[derive(Debug, Clone)]
pub(crate) struct SupportQuoteCandidate {
    pub(crate) id: String,
    pub(crate) section_id: String,
    pub(crate) section_hash: String,
    pub(crate) section_title: String,
    pub(crate) quote: String,
    pub(crate) score: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GeneratorSelectionOutput {
    question: String,
    answer: String,
    difficulty_rationale: String,
    expected_failure_mode: String,
    support_quote_id: String,
    required_key_points: Vec<String>,
    confidence: u8,
}

pub use build::build_paper_tournament;

#[path = "paper_tournament/build.rs"]
mod build;
#[path = "paper_tournament/execution.rs"]
mod execution;
#[path = "paper_tournament/gating.rs"]
mod gating;
#[path = "paper_tournament/hashish.rs"]
mod hashish;
#[path = "paper_tournament/live.rs"]
mod live;
#[path = "paper_tournament/live_candidate.rs"]
mod live_candidate;
#[path = "paper_tournament/live_outcome.rs"]
mod live_outcome;
#[path = "paper_tournament/live_outcome_support.rs"]
mod live_outcome_support;
#[path = "paper_tournament/live_runner.rs"]
mod live_runner;
#[path = "paper_tournament/live_runner_call.rs"]
mod live_runner_call;
#[path = "paper_tournament/live_runner_http.rs"]
mod live_runner_http;
#[path = "paper_tournament/live_runner_parse.rs"]
mod live_runner_parse;
#[path = "paper_tournament/live_runner_support.rs"]
mod live_runner_support;
#[path = "paper_tournament/live_write.rs"]
mod live_write;
#[path = "paper_tournament/preflight.rs"]
mod preflight;
#[path = "paper_tournament/prompt.rs"]
mod prompt;
#[path = "paper_tournament/prompt_support.rs"]
mod prompt_support;
#[path = "paper_tournament/provenance.rs"]
mod provenance;
#[path = "paper_tournament/schemas.rs"]
mod schemas;
#[path = "paper_tournament/selection.rs"]
mod selection;
#[path = "paper_tournament/summary.rs"]
mod summary;
#[path = "paper_tournament/summary_reports.rs"]
mod summary_reports;

use execution::{run_single_paper, TournamentWriteResult};
use gating::{
    grade_reduction_with_min, tournament_rejection_reasons, valid_generation_trials,
    valid_testing_trials, valid_verification_trials, verification_majority_with_min,
};
use hashish::{domain_for_paper, run_id, smoke_paper, title_case};
use live::run_single_paper_jnoccio;
use live_outcome::{append_progress_row, ensure_paper_time_remaining, progress_jsonl_path};
use live_runner::JnoccioCallError;
use live_runner::JnoccioHttpRunner;
use live_runner_support::{
    retry_jitter_ms, route_metadata_from_jnoccio, validate_live_route_metadata,
};
use preflight::{
    collect_model_summaries, compact_model_summary, configured_jnoccio_model, fetch_optional_json,
    fetch_required_text, insert_model_summary, model_matches_gateway_visible_model,
    route_summary_for_challenge, summarize_jnoccio_models,
};
use prompt::{build_testing_prompt_with_options, challenge_from_artifact};
use prompt_support::{
    accepted_ratio, mean_tester_confidence, provenance, testing_correct_rate_with_min,
};
use provenance::{
    answer_from_quote, failure, failure_category, failure_route_label, fatal_failure_category,
    first_sentence, live_call_failure, receipt, select_distractors, select_hard_distractors,
};
use schemas::{
    confidence_schema, generator_output_from_selection, generator_prompt,
    generator_selection_response_schema, grader_prompt, grading_response_schema,
    paper_prompt_context, string_schema, testing_response_schema, verification_response_schema,
    verifier_prompt,
};
pub(crate) use selection::{
    content_tokens, eligible_support_quote, eligible_support_section, exact_sentences,
    paper_quality_allowed, support_quote_candidates, support_quote_candidates_with_min_score,
    support_quote_hardness_score, support_quote_score, support_quote_specificity_marker_count,
};
use summary::{
    existing_accepted_count, filter_papers_by_candidate_manifest, paper_already_attempted,
    read_json_silent, rejection_category, write_failure_summary, write_manifest,
};
use summary_reports::{write_jnoccio_preflight_report, write_rejection_analysis};

pub use gating::{grade_reduction, verification_majority};
pub use hashish::final_paper_challenge_artifact_hash;
pub use prompt::build_testing_prompt;
