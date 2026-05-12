use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use memory_benchmark::json::{self, Json};

fn bin_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_population_report"))
}

fn repo_target_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/memory-benchmark/population-report-chase-tests")
}

fn unique_dir(test_name: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let dir = repo_target_dir().join(format!("{}-{}-{}", test_name, std::process::id(), stamp));
    fs::create_dir_all(&dir).expect("create test dir");
    dir
}

fn write(path: impl AsRef<Path>, content: &str) {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent");
    }
    fs::write(path, content).expect("write test file");
}

fn lane_json(
    name: &str,
    source: &str,
    total: f64,
    low: f64,
    high: f64,
    gate_findings: &str,
    patch: Option<&str>,
) -> String {
    let mut body = format!(
        r#"{{"name":"{}","source":"{}","total":{},"bootstrap_ci":{{"ci95_low":{},"ci95_high":{}}},"gate_findings":{}"#,
        name, source, total, low, high, gate_findings
    );
    if let Some(patch) = patch {
        body.push_str(&format!(r#","patch":"{}""#, patch.replace('\n', "\\n")));
    }
    body.push('}');
    body
}

fn current_state_json(name: &str, source: &str, total: f64, low: f64, high: f64) -> String {
    format!(
        r#"{{"kind":"best-state","winner":{{"name":"{}","source":"{}","total":{},"bootstrap_ci":{{"ci95_low":{},"ci95_high":{}}},"gate_findings":{{"deterministic":true}}}}}}"#,
        name, source, total, low, high
    )
}

fn run_report(args: &[(&str, &Path)]) {
    let mut cmd = Command::new(bin_path());
    cmd.current_dir(env!("CARGO_MANIFEST_DIR"));
    for (flag, path) in args {
        cmd.arg(flag).arg(path);
    }
    let output = cmd.output().expect("run population_report");
    assert!(
        output.status.success(),
        "population_report failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn json_object(value: &Json) -> &BTreeMap<String, Json> {
    match value {
        Json::Object(map) => map,
        other => panic!("expected object, got {:?}", other),
    }
}

fn json_string<'a>(obj: &'a BTreeMap<String, Json>, key: &str) -> &'a str {
    match obj.get(key) {
        Some(Json::Str(value)) => value.as_str(),
        other => panic!("missing string field {}: {:?}", key, other),
    }
}

fn read_json(path: impl AsRef<Path>) -> Json {
    let text = fs::read_to_string(path).expect("read json");
    json::parse(&text).expect("parse json")
}

fn read_text(path: impl AsRef<Path>) -> String {
    fs::read_to_string(path).expect("read text")
}

fn decision_field<'a>(decision: &'a Json, key: &str) -> &'a Json {
    json_object(decision)
        .get(key)
        .unwrap_or_else(|| panic!("missing field {}", key))
}

#[test]
fn promotes_best_clean_lane_when_raw_top_is_gated() {
    let root = unique_dir("promotes_best_clean_lane_when_raw_top_is_gated");
    let lanes = root.join("lanes");
    fs::create_dir_all(&lanes).unwrap();
    let current = root.join("current.json");
    let best_state = root.join("best-state.json");
    let decision = root.join("promotion-decision.json");
    let negative = root.join("negative-memory.jsonl");
    let best_patch = root.join("best.patch");
    let scoreboard = root.join("scoreboard.tsv");
    let out = root.join("final.json");

    write(
        lanes.join("gated.json"),
        &lane_json(
            "lane-gated",
            "gated-source",
            90.0,
            89.0,
            91.0,
            r#"{"privacy_leaks":1}"#,
            Some("diff --git a/x b/x\n"),
        ),
    );
    write(
        lanes.join("clean.json"),
        &lane_json(
            "lane-clean",
            "clean-source",
            76.0,
            75.0,
            77.0,
            r#"{"deterministic":true}"#,
            Some("diff --git a/y b/y\n"),
        ),
    );
    write(
        &current,
        &current_state_json("current", "current-source", 73.0, 73.0, 73.0),
    );

    run_report(&[
        ("--lanes", lanes.as_path()),
        ("--current-best-state", current.as_path()),
        ("--best-state", best_state.as_path()),
        ("--promotion-decision", decision.as_path()),
        ("--negative-memory", negative.as_path()),
        ("--best-patch", best_patch.as_path()),
        ("--scoreboard", scoreboard.as_path()),
        ("--out", out.as_path()),
    ]);

    let decision_json = read_json(&decision);
    let decision_obj = json_object(&decision_json);
    assert_eq!(json_string(decision_obj, "decision"), "promote");
    assert_eq!(
        json_string(
            json_object(decision_field(&decision_json, "selected")),
            "name"
        ),
        "lane-clean"
    );
    assert_eq!(
        json_string(
            json_object(decision_field(&decision_json, "raw_top")),
            "name"
        ),
        "lane-gated"
    );
    assert_eq!(
        json_string(
            json_object(decision_field(&decision_json, "winner")),
            "name"
        ),
        "lane-clean"
    );

    let negative_text = read_text(&negative);
    assert!(negative_text.contains("\"lane\":\"lane-gated\""));
    assert!(!negative_text.contains("\"lane\":\"lane-clean\""));
    assert!(!negative_text.contains("\"lane\":\"current\""));
    assert_eq!(read_text(&best_patch), "diff --git a/y b/y\n");
    assert!(read_text(&scoreboard).contains("blocked_top"));
}

#[test]
fn rejects_clean_lane_below_0_75_margin() {
    let root = unique_dir("rejects_clean_lane_below_0_75_margin");
    let lanes = root.join("lanes");
    fs::create_dir_all(&lanes).unwrap();
    let current = root.join("current.json");
    let decision = root.join("promotion-decision.json");
    let negative = root.join("negative-memory.jsonl");
    let best_patch = root.join("best.patch");
    let out = root.join("final.json");

    write(
        lanes.join("clean.json"),
        &lane_json(
            "lane-clean",
            "clean-source",
            73.6,
            73.6,
            73.8,
            r#"{"deterministic":true}"#,
            Some("diff --git a/z b/z\n"),
        ),
    );
    write(
        &current,
        &current_state_json("current", "current-source", 73.0, 73.0, 73.0),
    );

    run_report(&[
        ("--lanes", lanes.as_path()),
        ("--current-best-state", current.as_path()),
        ("--promotion-decision", decision.as_path()),
        ("--negative-memory", negative.as_path()),
        ("--best-patch", best_patch.as_path()),
        ("--out", out.as_path()),
    ]);

    let decision_json = read_json(&decision);
    let decision_obj = json_object(&decision_json);
    assert_eq!(json_string(decision_obj, "decision"), "reject");
    assert!(json_string(decision_obj, "reason").contains("improves"));
    assert_eq!(
        json_string(
            json_object(decision_field(&decision_json, "selected")),
            "name"
        ),
        "lane-clean"
    );
    assert_eq!(read_text(&best_patch), "");
    assert!(read_text(&negative).is_empty());
}

#[test]
fn does_not_write_selected_or_current_to_negative_memory() {
    let root = unique_dir("does_not_write_selected_or_current_to_negative_memory");
    let lanes = root.join("lanes");
    fs::create_dir_all(&lanes).unwrap();
    let current = root.join("current.json");
    let negative = root.join("negative-memory.jsonl");
    let best_patch = root.join("best.patch");
    let decision = root.join("promotion-decision.json");
    let out = root.join("final.json");

    write(
        lanes.join("selected.json"),
        &lane_json(
            "lane-selected",
            "selected-source",
            78.0,
            78.0,
            78.2,
            r#"{"deterministic":true}"#,
            Some("diff --git a/s b/s\n"),
        ),
    );
    write(
        lanes.join("gated.json"),
        &lane_json(
            "lane-gated",
            "gated-source",
            79.0,
            79.0,
            79.3,
            r#"{"privacy_leaks":1}"#,
            Some("diff --git a/t b/t\n"),
        ),
    );
    write(
        &current,
        &current_state_json("current", "current-source", 75.0, 75.0, 75.0),
    );

    run_report(&[
        ("--lanes", lanes.as_path()),
        ("--current-best-state", current.as_path()),
        ("--promotion-decision", decision.as_path()),
        ("--negative-memory", negative.as_path()),
        ("--best-patch", best_patch.as_path()),
        ("--out", out.as_path()),
    ]);

    let negative_text = read_text(&negative);
    assert!(negative_text.contains("\"lane\":\"lane-gated\""));
    assert!(!negative_text.contains("\"lane\":\"lane-selected\""));
    assert!(!negative_text.contains("\"lane\":\"current\""));
    assert_eq!(
        json_string(json_object(&read_json(&decision)), "decision"),
        "promote"
    );
}

#[test]
fn negative_memory_appends_without_rewriting_existing_lines() {
    let root = unique_dir("negative_memory_appends_without_rewriting_existing_lines");
    let lanes = root.join("lanes");
    fs::create_dir_all(&lanes).unwrap();
    let current = root.join("current.json");
    let negative = root.join("negative-memory.jsonl");
    let best_patch = root.join("best.patch");
    let decision = root.join("promotion-decision.json");
    let out = root.join("final.json");

    let seed_line = r#"{"kind":"negative-memory","lane":"seed","source":"seed","reason":"lower_ranking","score":0,"gate_count":0}"#;
    write(&negative, &format!("{seed_line}\n"));
    write(
        lanes.join("loser.json"),
        &lane_json(
            "lane-loser",
            "loser-source",
            70.0,
            70.0,
            70.0,
            r#"{"deterministic":true}"#,
            Some("diff --git a/l b/l\n"),
        ),
    );
    write(
        lanes.join("winner.json"),
        &lane_json(
            "lane-winner",
            "winner-source",
            74.5,
            74.5,
            74.5,
            r#"{"deterministic":true}"#,
            Some("diff --git a/m b/m\n"),
        ),
    );
    write(
        &current,
        &current_state_json("current", "current-source", 73.0, 73.0, 73.0),
    );

    run_report(&[
        ("--lanes", lanes.as_path()),
        ("--current-best-state", current.as_path()),
        ("--promotion-decision", decision.as_path()),
        ("--negative-memory", negative.as_path()),
        ("--best-patch", best_patch.as_path()),
        ("--out", out.as_path()),
    ]);

    let contents = read_text(&negative);
    assert!(contents.starts_with(seed_line));
    assert!(contents.lines().count() >= 2);
    assert!(contents.contains("\"lane\":\"lane-loser\""));
}

#[test]
fn rejects_patch_path_outside_lane_directory() {
    let root = unique_dir("rejects_patch_path_outside_lane_directory");
    let lanes = root.join("lanes");
    fs::create_dir_all(&lanes).unwrap();
    let current = root.join("current.json");
    let outside_patch = root.join("escape.patch");
    let decision = root.join("promotion-decision.json");
    let negative = root.join("negative-memory.jsonl");
    let best_patch = root.join("best.patch");
    let out = root.join("final.json");

    write(&outside_patch, "diff --git a/escape b/escape\n");
    write(
        lanes.join("invalid-patch.json"),
        &format!(
            r#"{{"name":"lane-invalid-patch","source":"invalid-patch-source","total":90.0,"bootstrap_ci":{{"ci95_low":90.0,"ci95_high":90.0}},"gate_findings":{{"deterministic":true}},"patch_path":"../{}"}}"#,
            outside_patch.file_name().unwrap().to_string_lossy()
        ),
    );
    write(
        lanes.join("valid.json"),
        &lane_json(
            "lane-valid",
            "valid-source",
            76.0,
            76.0,
            76.0,
            r#"{"deterministic":true}"#,
            Some("diff --git a/v b/v\n"),
        ),
    );
    write(
        &current,
        &current_state_json("current", "current-source", 73.0, 73.0, 73.0),
    );

    run_report(&[
        ("--lanes", lanes.as_path()),
        ("--current-best-state", current.as_path()),
        ("--promotion-decision", decision.as_path()),
        ("--negative-memory", negative.as_path()),
        ("--best-patch", best_patch.as_path()),
        ("--out", out.as_path()),
    ]);

    let decision_json = read_json(&decision);
    assert_eq!(
        json_string(
            json_object(decision_field(&decision_json, "selected")),
            "name"
        ),
        "lane-valid"
    );
    assert!(read_text(&negative).contains("\"reason\":\"invalid_lane_report\""));
    assert_eq!(read_text(&best_patch), "diff --git a/v b/v\n");
}

#[test]
fn reports_invalid_lane_json_without_blocking_clean_lane() {
    let root = unique_dir("reports_invalid_lane_json_without_blocking_clean_lane");
    let lanes = root.join("lanes");
    fs::create_dir_all(&lanes).unwrap();
    let current = root.join("current.json");
    let decision = root.join("promotion-decision.json");
    let negative = root.join("negative-memory.jsonl");
    let best_patch = root.join("best.patch");
    let out = root.join("final.json");

    write(lanes.join("broken.json"), r#"{"name":"broken""#);
    write(
        lanes.join("clean.json"),
        &lane_json(
            "lane-clean",
            "clean-source",
            78.0,
            78.0,
            78.0,
            r#"{"deterministic":true}"#,
            Some("diff --git a/c b/c\n"),
        ),
    );
    write(
        &current,
        &current_state_json("current", "current-source", 73.0, 73.0, 73.0),
    );

    run_report(&[
        ("--lanes", lanes.as_path()),
        ("--current-best-state", current.as_path()),
        ("--promotion-decision", decision.as_path()),
        ("--negative-memory", negative.as_path()),
        ("--best-patch", best_patch.as_path()),
        ("--out", out.as_path()),
    ]);

    let decision_json = read_json(&decision);
    let read_errors = decision_field(&decision_json, "read_errors");
    assert!(format!("{:?}", read_errors).contains("invalid_lane_report"));
    assert!(read_text(&negative).contains("\"reason\":\"invalid_lane_report\""));
    assert_eq!(
        json_string(
            json_object(decision_field(&decision_json, "selected")),
            "name"
        ),
        "lane-clean"
    );
}

#[test]
fn duplicate_lane_names_are_disambiguated_by_source() {
    let root = unique_dir("duplicate_lane_names_are_disambiguated_by_source");
    let lanes = root.join("lanes");
    fs::create_dir_all(&lanes).unwrap();
    let current = root.join("current.json");
    let decision = root.join("promotion-decision.json");
    let negative = root.join("negative-memory.jsonl");
    let best_patch = root.join("best.patch");
    let scoreboard = root.join("scoreboard.tsv");
    let out = root.join("final.json");

    write(
        lanes.join("a.json"),
        &lane_json(
            "lane-same",
            "source-a",
            80.0,
            80.0,
            80.0,
            r#"{"deterministic":true}"#,
            Some("diff --git a/a b/a\n"),
        ),
    );
    write(
        lanes.join("b.json"),
        &lane_json(
            "lane-same",
            "source-b",
            81.0,
            81.0,
            81.0,
            r#"{"deterministic":true}"#,
            Some("diff --git a/b b/b\n"),
        ),
    );
    write(
        &current,
        &current_state_json("current", "current-source", 70.0, 70.0, 70.0),
    );

    run_report(&[
        ("--lanes", lanes.as_path()),
        ("--current-best-state", current.as_path()),
        ("--promotion-decision", decision.as_path()),
        ("--negative-memory", negative.as_path()),
        ("--best-patch", best_patch.as_path()),
        ("--scoreboard", scoreboard.as_path()),
        ("--out", out.as_path()),
    ]);

    let decision_json = read_json(&decision);
    assert_eq!(
        json_string(
            json_object(decision_field(&decision_json, "selected")),
            "source"
        ),
        "source-b"
    );
    assert!(read_text(&scoreboard).contains("source-a"));
    assert!(read_text(&scoreboard).contains("source-b"));
    assert!(read_text(&negative).contains("\"source\":\"source-a\""));
    assert_eq!(read_text(&best_patch), "diff --git a/b b/b\n");
}

#[test]
fn current_candidates_seed_best_state_from_measured_candidate() {
    let root = unique_dir("current_candidates_seed_best_state_from_measured_candidate");
    let lanes = root.join("lanes");
    let candidates = root.join("current-candidates");
    fs::create_dir_all(&lanes).unwrap();
    fs::create_dir_all(&candidates).unwrap();
    let decision = root.join("promotion-decision.json");
    let best_state = root.join("best-state.json");
    let negative = root.join("negative-memory.jsonl");
    let best_patch = root.join("best.patch");
    let out = root.join("final.json");

    write(
        candidates.join("measured-a.json"),
        &lane_json(
            "measured-a",
            "candidate-source-a",
            75.0,
            75.0,
            75.0,
            r#"{"deterministic":true}"#,
            Some("diff --git a/a b/a\n"),
        ),
    );
    write(
        candidates.join("measured-b.json"),
        &lane_json(
            "measured-b",
            "candidate-source-b",
            77.0,
            77.0,
            77.0,
            r#"{"deterministic":true}"#,
            Some("diff --git a/b b/b\n"),
        ),
    );
    write(
        lanes.join("lane-new.json"),
        &lane_json(
            "lane-new",
            "new-source",
            80.0,
            80.0,
            80.0,
            r#"{"deterministic":true}"#,
            Some("diff --git a/n b/n\n"),
        ),
    );

    run_report(&[
        ("--lanes", lanes.as_path()),
        ("--current-candidates", candidates.as_path()),
        ("--promotion-decision", decision.as_path()),
        ("--best-state", best_state.as_path()),
        ("--negative-memory", negative.as_path()),
        ("--best-patch", best_patch.as_path()),
        ("--out", out.as_path()),
    ]);

    let decision_json = read_json(&decision);
    assert_eq!(
        json_string(
            json_object(decision_field(&decision_json, "current")),
            "name"
        ),
        "measured-b"
    );
    assert_eq!(
        json_string(
            json_object(decision_field(&decision_json, "winner")),
            "name"
        ),
        "lane-new"
    );
    assert_eq!(
        json_string(
            json_object(decision_field(&read_json(&best_state), "winner")),
            "name",
        ),
        "lane-new"
    );
    assert_eq!(read_text(&best_patch), "diff --git a/n b/n\n");
}

#[test]
fn missing_patch_blocks_lane_promotion() {
    let root = unique_dir("missing_patch_blocks_lane_promotion");
    let lanes = root.join("lanes");
    fs::create_dir_all(&lanes).unwrap();
    let current = root.join("current.json");
    let decision = root.join("promotion-decision.json");
    let negative = root.join("negative-memory.jsonl");
    let best_patch = root.join("best.patch");
    let out = root.join("final.json");

    write(
        lanes.join("no-patch.json"),
        &lane_json(
            "lane-no-patch",
            "no-patch-source",
            82.0,
            82.0,
            82.0,
            r#"{"deterministic":true}"#,
            None,
        ),
    );
    write(
        &current,
        &current_state_json("current", "current-source", 73.0, 73.0, 73.0),
    );

    run_report(&[
        ("--lanes", lanes.as_path()),
        ("--current-best-state", current.as_path()),
        ("--promotion-decision", decision.as_path()),
        ("--negative-memory", negative.as_path()),
        ("--best-patch", best_patch.as_path()),
        ("--out", out.as_path()),
    ]);

    let decision_json = read_json(&decision);
    assert_eq!(
        json_string(json_object(&decision_json), "decision"),
        "reject"
    );
    assert_eq!(
        json_string(
            json_object(decision_field(&decision_json, "selected")),
            "name"
        ),
        "current"
    );
    assert_eq!(read_text(&best_patch), "");
    assert!(read_text(&negative).contains("\"lane\":\"lane-no-patch\""));
}
