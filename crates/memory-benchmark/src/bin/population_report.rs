//! `population_report` CLI wrapper.

use std::env;
use std::process;

use memory_benchmark::chase_report::{self, CliOptions};

fn main() {
    let options = parse_args();
    if let Err(err) = chase_report::run(options) {
        eprintln!("population_report: {}", err);
        process::exit(1);
    }
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
            _ => {
                i += 1;
            }
        }
    }
    options
}
