//! Layer 4 — end-to-end CLI smoke: `sandboxctl validate <fixture>` succeeds.
//! Uses `assert_cmd` to invoke the binary built by the test profile.

use std::path::PathBuf;

use assert_cmd::Command;
use predicates::str::contains;

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("sample-lanes.toml")
}

#[test]
fn validate_subcommand_accepts_fixture() {
    let mut cmd = Command::cargo_bin("sandboxctl").expect("binary");
    cmd.arg("--lanes")
        .arg(fixture_path())
        .arg("validate")
        .assert()
        .success()
        .stdout(contains("validated"));
}

#[test]
fn validate_json_output() {
    let mut cmd = Command::cargo_bin("sandboxctl").expect("binary");
    cmd.arg("--lanes")
        .arg(fixture_path())
        .arg("--json")
        .arg("validate")
        .assert()
        .success()
        .stdout(contains("\"ok\":true"))
        .stdout(contains("\"schema_version\":\"1.0.0\""));
}

#[test]
fn validate_strict_flag_does_not_error_on_valid_fixture() {
    let mut cmd = Command::cargo_bin("sandboxctl").expect("binary");
    cmd.arg("--lanes")
        .arg(fixture_path())
        .arg("validate")
        .arg("--strict")
        .assert()
        .success();
}

#[test]
fn list_subcommand_runs() {
    // Isolate sandbox root so we don't surface real entries from the user's
    // host into the test output.
    let tmp = tempfile::tempdir().expect("tmp");
    let mut cmd = Command::cargo_bin("sandboxctl").expect("binary");
    cmd.env("SANDBOXCTL_ROOT", tmp.path())
        .arg("list")
        .assert()
        .success()
        .stdout(contains("no sandboxes"));
}
