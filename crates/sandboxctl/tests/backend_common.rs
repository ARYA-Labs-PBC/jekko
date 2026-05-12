use std::fs;
use std::process::Command;

use sandboxctl::backend::common::run_argv_with_output;
use tempfile::tempdir;

#[test]
fn captures_streams_and_exit_code() {
    let tmp = tempdir().expect("tmp");
    let stdout_path = tmp.path().join("stdout.txt");
    let stderr_path = tmp.path().join("stderr.txt");

    let mut cmd = Command::new("sh");
    cmd.args(["-c", "printf 'out'; printf 'err' >&2; exit 7"]);

    let outcome = run_argv_with_output(cmd, &stdout_path, &stderr_path, || {
        "backend common test command".to_string()
    })
    .expect("command runs");

    assert_eq!(outcome.exit_code, 7);
    assert_eq!(fs::read_to_string(stdout_path).expect("stdout"), "out");
    assert_eq!(fs::read_to_string(stderr_path).expect("stderr"), "err");
}

