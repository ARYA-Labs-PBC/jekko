//! `jekko run --help` smoke test.

use assert_cmd::Command;
use predicates::prelude::*;
use predicates::str::contains;

#[test]
fn run_help_exits_zero_and_mentions_prompt() {
    let mut cmd = Command::cargo_bin("jekko").expect("jekko binary");
    cmd.args(["run", "--help"])
        .assert()
        .success()
        .stdout(contains("prompt").or(contains("PROMPT")));
}

#[test]
fn run_help_mentions_provider_and_model_flags() {
    let mut cmd = Command::cargo_bin("jekko").expect("jekko binary");
    cmd.args(["run", "--help"])
        .assert()
        .success()
        .stdout(contains("--provider"))
        .stdout(contains("--model"));
}
