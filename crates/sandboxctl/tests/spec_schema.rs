//! Layer 1 — schema validation as integration test. Mirrors the inline
//! `spec::tests` so a green `cargo test --test spec_schema` proves the public
//! parser surface accepts every supported field shape.

use std::path::PathBuf;

use sandboxctl::spec;

fn fixture_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("tests");
    p.push("fixtures");
    p.push("sample-lanes.toml");
    p
}

#[test]
fn fixture_parses_and_validates() {
    let doc = spec::load(&fixture_path()).expect("load");
    assert!(!doc.lanes.is_empty(), "fixture has lanes");
    spec::validate(&doc).expect("valid");
}

#[test]
fn fixture_uses_at_least_two_backends() {
    let doc = spec::load(&fixture_path()).expect("load");
    let backends: std::collections::HashSet<_> =
        doc.lanes.iter().map(|l| l.runtime.backend).collect();
    assert!(
        backends.len() >= 2,
        "fixture should exercise multiple backends, found {backends:?}"
    );
}

#[test]
fn unique_lane_names_and_command_ids() {
    let doc = spec::load(&fixture_path()).expect("load");
    let names: std::collections::HashSet<_> = doc.lanes.iter().map(|l| &l.name).collect();
    let ids: std::collections::HashSet<_> = doc.lanes.iter().map(|l| &l.command_id).collect();
    assert_eq!(names.len(), doc.lanes.len(), "names unique");
    assert_eq!(ids.len(), doc.lanes.len(), "command_ids unique");
}

#[test]
fn every_lane_has_non_empty_allowlist() {
    let doc = spec::load(&fixture_path()).expect("load");
    for lane in &doc.lanes {
        assert!(
            !lane.commands.allowed_patterns.is_empty(),
            "lane {} must declare at least one allowed pattern",
            lane.name
        );
    }
}

#[test]
fn canonical_lanes_file_parses() {
    let canonical = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("agent")
        .join("sandbox-lanes.toml");
    if !canonical.exists() {
        eprintln!("SKIP: canonical {} missing", canonical.display());
        return;
    }
    let doc = spec::load(&canonical).expect("canonical sandbox-lanes parses");
    assert!(!doc.lanes.is_empty());
    spec::validate(&doc).expect("canonical valid");
}
