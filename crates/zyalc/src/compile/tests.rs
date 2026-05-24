use std::fs;
use std::path::Path;

use super::emit::{emit_toml, strip_pragmas};
use super::target::source_reference;
use super::*;

#[test]
fn round_trip_strips_pragmas() {
    let raw = "# zyal: declarative target=toml schema=test@1\nschema_version: \"1.0.0\"\nlanes:\n  - name: a\n    command_id: x.a\n    cost: 1\n";
    let stripped = strip_pragmas(raw);
    assert!(!stripped.contains("# zyal:"));
    assert!(stripped.contains("schema_version"));
}

#[test]
fn toml_emit_basic() {
    let raw = "# zyal: declarative target=toml schema=t@1\nschema_version: \"1.0.0\"\nlanes:\n  - name: a\n    cost: 1\n";
    let out = emit_toml(raw).expect("emit");
    assert!(out.contains("schema_version"));
    assert!(out.contains("[[lane]]"));
    assert!(out.contains("name = \"a\""));
}

#[test]
fn idempotent_emit() {
    let raw = "# zyal: declarative target=toml schema=t@1\nschema_version: \"1.0.0\"\nlanes:\n  - name: a\n    cost: 1\n";
    let a = emit_toml(raw).unwrap();
    let b = emit_toml(raw).unwrap();
    assert_eq!(a, b, "compile must be idempotent");
}

#[test]
fn runbook_profiles_validate_without_emitting_legacy_yaml() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("smoke.zyal");
    fs::write(
        &source,
        "<<<ZYAL v1:daemon id=smoke>>>\njob:\n  name: smoke\n<<<END_ZYAL id=smoke>>>\n",
    )
    .unwrap();

    let outcome = compile_one(&source, None, true).unwrap();
    assert!(matches!(outcome, Outcome::Unchanged(path) if path == source));
    assert!(
        !source.with_extension("yml").exists(),
        "runbook validation must not emit retired .zyal.yml artifacts"
    );
}

#[test]
fn source_reference_preserves_canonical_subdirectory() {
    assert_eq!(
        source_reference(Path::new("./agent/zyal/sandbox-lanes.zyal")),
        "agent/zyal/sandbox-lanes.zyal"
    );
}
