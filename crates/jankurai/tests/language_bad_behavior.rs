use anyhow::{ensure, Result};

use jankurai::{LANGUAGE_BAD_BEHAVIOR_COMMAND, LANGUAGE_BAD_BEHAVIOR_LANES};

fn lane_names() -> Vec<&'static str> {
    LANGUAGE_BAD_BEHAVIOR_LANES.iter().map(|(lane, _)| *lane).collect()
}

#[test]
fn covers_ci_git_and_release_bad_behavior_lanes() -> Result<()> {
    ensure!(
        lane_names() == vec!["ci-bad-behavior", "git-bad-behavior", "release-bad-behavior"],
        "unexpected lane coverage: {:?}",
        lane_names()
    );
    Ok(())
}

#[test]
fn uses_manifest_path_test_command() {
    assert!(
        LANGUAGE_BAD_BEHAVIOR_COMMAND.contains("--manifest-path crates/jankurai/Cargo.toml"),
        "expected manifest-path cargo test command: {}",
        LANGUAGE_BAD_BEHAVIOR_COMMAND
    );
    assert!(
        LANGUAGE_BAD_BEHAVIOR_COMMAND.contains("--test language_bad_behavior"),
        "expected named test target: {}",
        LANGUAGE_BAD_BEHAVIOR_COMMAND
    );
}

#[test]
fn lane_suffixes_match_behavior_surface() {
    for (lane, surface) in LANGUAGE_BAD_BEHAVIOR_LANES {
        assert!(
            lane.ends_with("-bad-behavior"),
            "lane should end with -bad-behavior: {lane}"
        );
        assert!(
            !surface.is_empty(),
            "lane surface name should not be empty: {lane}"
        );
    }
}
