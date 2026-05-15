use qbank_builder::{
    acceptance_passes, canonicalize_paper, cogcore_events_for_papers, ensure_bank_layout,
    final_paper_challenge_artifact_hash, finalize_challenge, manifest_hash, production_bank_errors,
    read_challenges, read_json, read_papers, seed_fixture_bank, sorted_challenges,
    write_json_pretty, AgentRunnerMode, BuildPaperTournamentConfig, ChallengeRecord,
    FinalPaperChallengeArtifact, FullTextDiscoveryConfig, LicenseRecord, PaperRecord, PaperSection,
    RouteModelPolicy, WorkItem, MIN_SUCCESSFUL_GENERATORS, MIN_SUCCESSFUL_GRADERS,
    MIN_SUCCESSFUL_TESTERS, MIN_SUCCESSFUL_VERIFIERS, PAPER_SCHEMA_VERSION,
    PRODUCTION_CHALLENGE_SCHEMA_VERSION, PRODUCTION_MANIFEST_SCHEMA_VERSION,
};
use serde_json::json;
use std::env;
use std::path::{Path, PathBuf};

mod bank;
mod discover;
mod tournament;
mod tournament_audit;
mod util;
mod workflow;

pub(crate) use util::{f64_value, i32_value, path_value, u64_value, usize_value, value};

pub async fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    let Some(command) = args.get(1).map(String::as_str) else {
        print_help();
        return Err("missing command".to_string());
    };
    match command {
        "discover" => discover::discover(&args[2..]).await,
        "discover-publications" => discover::discover(&args[2..]).await,
        "discover-full-text" => discover::discover_full_text_command(&args[2..]).await,
        "seed-fixture-bank" => discover::seed_fixture_bank_command(&args[2..]),
        "publish-paper" => workflow::publish_paper(&args[2..]),
        "extract-publication" => workflow::publish_paper(&args[2..]),
        "make-work" => workflow::make_work(&args[2..]),
        "pack-context" => workflow::pack_context_command(&args[2..]),
        "build-paper-tournament" => {
            let command_args = args[2..].to_vec();
            tokio::task::spawn_blocking(move || {
                tournament::build_paper_tournament_command(&command_args)
            })
            .await
            .map_err(|err| format!("build-paper-tournament task failed: {err}"))?
        }
        "audit-paper-tournament" => tournament_audit::audit_paper_tournament_command(&args[2..]),
        "reduce" => bank::reduce(&args[2..]),
        "reduce-trials" => bank::reduce_trials(&args[2..]),
        "publish" => bank::publish_manifest(&args[2..]),
        "audit-bank" => bank::audit_bank(&args[2..]),
        "emit-cogcore" => bank::emit_cogcore(&args[2..]),
        "--help" | "-h" => {
            print_help();
            Ok(())
        }
        other => Err(format!("unknown command {other:?}")),
    }
}

fn print_help() {
    eprintln!(
        "qbank <discover|discover-publications|discover-full-text|seed-fixture-bank|publish-paper|extract-publication|make-work|pack-context|build-paper-tournament|audit-paper-tournament|reduce|reduce-trials|publish|audit-bank|emit-cogcore> [--bank path] [--run-root path] [--agent-runner mock|jnoccio] [--jnoccio-model id] [--jnoccio-max-output-tokens n] [--jnoccio-request-timeout-seconds n] [--paper-timeout-seconds n] [--generators n] [--verifiers n] [--testers n] [--graders n] [--min-successful-generators n] [--min-successful-verifiers n] [--min-successful-testers n] [--min-successful-graders n] [--progress-jsonl path] [--candidate-manifest path] [--resume] [--phase-retries n] [--allow-mock-smoke] [--json-errors-ok]"
    );
}
