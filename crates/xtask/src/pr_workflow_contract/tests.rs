use std::fs;
use std::path::PathBuf;

use serde_yaml::{Mapping, Value};

use super::validate::validate_workflow;

fn workflow_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.github/workflows/pr-standards.yml")
}

fn load_workflow() -> Value {
    let text = fs::read_to_string(workflow_path()).expect("read workflow fixture");
    serde_yaml::from_str(&text).expect("parse workflow fixture")
}

fn workflow_root_mut(workflow: &mut Value) -> &mut Mapping {
    workflow.as_mapping_mut().expect("workflow mapping")
}

fn workflow_env_mut(workflow: &mut Value) -> &mut Mapping {
    workflow_root_mut(workflow)
        .get_mut(Value::String("env".into()))
        .and_then(Value::as_mapping_mut)
        .expect("workflow env mapping")
}

fn standard_job_steps_mut(workflow: &mut Value) -> &mut [Value] {
    workflow_root_mut(workflow)
        .get_mut(Value::String("jobs".into()))
        .and_then(Value::as_mapping_mut)
        .expect("jobs mapping")
        .get_mut(Value::String("check-standards".into()))
        .and_then(Value::as_mapping_mut)
        .expect("check-standards job mapping")
        .get_mut(Value::String("steps".into()))
        .and_then(Value::as_sequence_mut)
        .expect("steps sequence")
}

#[test]
fn accepts_expected_workflow_contract() {
    validate_workflow(&load_workflow()).expect("workflow contract");
}

#[test]
fn rejects_missing_required_env_key() {
    let mut workflow = load_workflow();
    workflow_env_mut(&mut workflow).remove(Value::String("GH_REPO".into()));

    validate_workflow(&workflow).expect_err("expected failure");
}

#[test]
fn rejects_inline_step_exports() {
    let mut workflow = load_workflow();
    let steps = standard_job_steps_mut(&mut workflow);
    let script = steps[2].as_mapping_mut().expect("script step mapping");
    script.insert(
        Value::String("run".into()),
        Value::String(
            r#"export GH_TOKEN="${{ github.token }}"
export GITHUB_TOKEN="${{ github.token }}"
export GH_REPO="${{ github.repository }}"
export GITHUB_REPOSITORY="${{ github.repository }}"
export GITHUB_BASE_REF="${{ github.base_ref }}"
export GITHUB_HEAD_REF="${{ github.head_ref }}"
export GITHUB_EVENT_PATH="${{ github.event_path }}"
bash ops/ci/pr-policy.sh standards
"#
            .into(),
        ),
    );

    validate_workflow(&workflow).expect_err("expected failure");
}

#[test]
fn rejects_old_wrapper_invocation() {
    let mut workflow = load_workflow();
    let steps = standard_job_steps_mut(&mut workflow);
    let script = steps[2].as_mapping_mut().expect("script step mapping");
    script.insert(
        Value::String("run".into()),
        Value::String("bash ops/ci/pr-standards.sh".into()),
    );

    validate_workflow(&workflow).expect_err("expected failure");
}
