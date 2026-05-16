//! Smoke test for `jekko --version`.
//!
//! Asserts the binary builds and prints a non-empty version line.

use assert_cmd::Command;
use predicates::prelude::*;
use predicates::str::contains;

#[test]
fn version_prints_non_empty() {
    let mut cmd = Command::cargo_bin("jekko").expect("jekko binary");
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(contains("jekko").or(contains("0.")))
        .stdout(predicates::str::is_empty().not());
}

#[test]
fn short_v_alias_works() {
    let mut cmd = Command::cargo_bin("jekko").expect("jekko binary");
    cmd.arg("-V").assert().success();
}
