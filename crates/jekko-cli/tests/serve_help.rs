//! `jekko serve --help` smoke test.

use assert_cmd::Command;
use predicates::str::contains;

#[test]
fn serve_help_exits_zero_and_mentions_port() {
    let mut cmd = Command::cargo_bin("jekko").expect("jekko binary");
    cmd.args(["serve", "--help"])
        .assert()
        .success()
        .stdout(contains("--port"));
}

#[test]
fn serve_help_mentions_hostname_flag() {
    let mut cmd = Command::cargo_bin("jekko").expect("jekko binary");
    cmd.args(["serve", "--help"])
        .assert()
        .success()
        .stdout(contains("--hostname"));
}
