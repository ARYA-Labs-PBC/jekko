pub const LANGUAGE_BAD_BEHAVIOR_LANES: &[(&str, &str)] = &[
    ("ci-bad-behavior", "ci"),
    ("git-bad-behavior", "git"),
    ("release-bad-behavior", "release"),
];

pub const LANGUAGE_BAD_BEHAVIOR_COMMAND: &str =
    "cargo test --manifest-path crates/jankurai/Cargo.toml --test language_bad_behavior --no-fail-fast";
