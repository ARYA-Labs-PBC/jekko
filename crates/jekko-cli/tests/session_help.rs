//! `jekko session --help` smoke test.

use assert_cmd::Command;
use predicates::str::contains;

#[test]
fn session_help_exits_zero() {
    let mut cmd = Command::cargo_bin("jekko").expect("jekko binary");
    cmd.args(["session", "--help"]).assert().success();
}

#[test]
fn session_help_lists_subcommands() {
    let mut cmd = Command::cargo_bin("jekko").expect("jekko binary");
    cmd.args(["session", "--help"])
        .assert()
        .success()
        .stdout(contains("list"))
        .stdout(contains("show"))
        .stdout(contains("delete"));
}
