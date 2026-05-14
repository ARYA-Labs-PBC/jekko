//! `chase_reduce` — strict reducer for memory-benchmark chase outputs.
//!
//! The reducer is intentionally narrow: it requires lane reports plus the
//! promotion artifacts and refuses to run if the chase contract is not fully
//! populated.

use std::env;
use std::process;

use memory_benchmark::chase_report::{self, CliOptions};

fn main() {
    let options = parse_args();
    if let Err(err) = validate(&options).and_then(|_| chase_report::run(options)) {
        eprintln!("chase_reduce: {err}");
        process::exit(1);
    }
}

fn validate(options: &CliOptions) -> Result<(), String> {
    if options.lanes_path.is_none() {
        return Err("--lanes is required".to_string());
    }
    if options.best_state.is_none() {
        return Err("--best-state is required".to_string());
    }
    if options.promotion_decision.is_none() {
        return Err("--promotion-decision is required".to_string());
    }
    if options.negative_memory.is_none() {
        return Err("--negative-memory is required".to_string());
    }
    if options.best_patch.is_none() {
        return Err("--best-patch is required".to_string());
    }
    if options.out.is_none() {
        return Err("--out is required".to_string());
    }
    Ok(())
}

fn parse_args() -> CliOptions {
    let args: Vec<String> = env::args().collect();
    let mut options = CliOptions::default();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--population" => {
                options.population = args.get(i + 1).cloned();
                i += 2;
            }
            "--baseline" => {
                options.baseline_path = args.get(i + 1).cloned();
                i += 2;
            }
            "--exec" => {
                options.exec_path = args.get(i + 1).cloned();
                i += 2;
            }
            "--shadow-report" => {
                options.shadow_report = args.get(i + 1).cloned();
                i += 2;
            }
            "--lanes" => {
                options.lanes_path = args.get(i + 1).cloned();
                i += 2;
            }
            "--current-best-state" => {
                options.current_best_state = args.get(i + 1).cloned();
                i += 2;
            }
            "--current-candidates" => {
                options.current_candidates = args.get(i + 1).cloned();
                i += 2;
            }
            "--scoreboard" => {
                options.scoreboard = args.get(i + 1).cloned();
                i += 2;
            }
            "--best-state" => {
                options.best_state = args.get(i + 1).cloned();
                i += 2;
            }
            "--promotion-decision" => {
                options.promotion_decision = args.get(i + 1).cloned();
                i += 2;
            }
            "--negative-memory" => {
                options.negative_memory = args.get(i + 1).cloned();
                i += 2;
            }
            "--best-patch" => {
                options.best_patch = args.get(i + 1).cloned();
                i += 2;
            }
            "--out" => {
                options.out = args.get(i + 1).cloned();
                i += 2;
            }
            "--markdown" => {
                options.markdown = args.get(i + 1).cloned();
                i += 2;
            }
            "--comparison" => {
                options.comparison = args.get(i + 1).cloned();
                i += 2;
            }
            "--triangulation" => {
                options.triangulation = args.get(i + 1).cloned();
                i += 2;
            }
            "--curriculum" => {
                options.curriculum = args.get(i + 1).cloned();
                i += 2;
            }
            "--reference-report" => {
                if let Some(value) = args.get(i + 1) {
                    options.reference_reports.push(value.clone());
                }
                i += 2;
            }
            _ => {
                i += 1;
            }
        }
    }
    options
}
