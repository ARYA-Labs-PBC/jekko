use anyhow::{bail, Context, Result};
use serde_yaml::{Mapping, Value};
use std::fs;

use crate::shared::repo_root;

const WORKFLOW_REL: &str = ".github/workflows/pr-standards.yml";
const EXPECTED_WORKFLOW_NAME: &str = "pr-standards";
const EXPECTED_TRIGGER_TYPES: &[&str] = &["opened", "edited", "synchronize"];
const CHECKOUT_USE: &str = "actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683";
const TOOLCHAIN_USE: &str = "dtolnay/rust-toolchain@29eef336d9b2848a0b548edc03f92a220660cdb8";

pub fn run() -> Result<()> {
    let root = repo_root()?;
    let path = root.join(WORKFLOW_REL);
    let text = fs::read_to_string(&path)
        .with_context(|| format!("read workflow contract {}", path.display()))?;
    let workflow: Value =
        serde_yaml::from_str(&text).with_context(|| format!("parse {}", path.display()))?;
    validate_workflow(&workflow)?;
    println!("pr-workflow-contract: checked {}", path.display());
    Ok(())
}

fn validate_workflow(workflow: &Value) -> Result<()> {
    let workflow_map = mapping(workflow, "workflow root")?;
    assert_workflow_root_keys(workflow_map)?;
    assert_string(
        workflow_map,
        "name",
        EXPECTED_WORKFLOW_NAME,
        "workflow name",
    )?;
    assert_exact_strings(
        &on_trigger_types(workflow_map)?,
        EXPECTED_TRIGGER_TYPES,
        "pull_request_target.types",
    )?;
    assert_permissions(
        mapping(
            value_for_key(workflow_map, "permissions", "workflow permissions")?,
            "workflow permissions",
        )?,
        &[("contents", "read")],
        "workflow permissions",
    )?;

    let jobs = mapping(value_for_key(workflow_map, "jobs", "jobs")?, "jobs")?;
    assert_job(
        jobs,
        "check-standards",
        "Check PR standards",
        r#"export GH_TOKEN="${{ secrets.GITHUB_TOKEN }}"
export GITHUB_TOKEN="${{ secrets.GITHUB_TOKEN }}"
export GITHUB_REPOSITORY="${{ github.repository }}"
export GITHUB_BASE_REF="${{ github.base_ref }}"
export GITHUB_HEAD_REF="${{ github.head_ref }}"
export GITHUB_EVENT_PATH="${{ github.event_path }}"
bash ops/ci/pr-standards.sh
"#,
        "check-standards",
    )?;
    assert_job(
        jobs,
        "check-compliance",
        "Check PR template compliance",
        r#"export GH_TOKEN="${{ secrets.GITHUB_TOKEN }}"
export GITHUB_TOKEN="${{ secrets.GITHUB_TOKEN }}"
export GITHUB_REPOSITORY="${{ github.repository }}"
export GITHUB_BASE_REF="${{ github.base_ref }}"
export GITHUB_HEAD_REF="${{ github.head_ref }}"
export GITHUB_EVENT_PATH="${{ github.event_path }}"
bash ops/ci/pr-compliance.sh
"#,
        "check-compliance",
    )?;
    Ok(())
}

fn assert_job(
    jobs: &Mapping,
    name: &str,
    expected_step_name: &str,
    expected_run: &str,
    expected_concurrency_suffix: &str,
) -> Result<()> {
    let job = mapping(
        value_for_key(jobs, name, &format!("job {name}"))?,
        &format!("job {name}"),
    )?;
    assert_exact_keys(
        job,
        &[
            "runs-on",
            "timeout-minutes",
            "concurrency",
            "permissions",
            "steps",
        ],
        &format!("job {name}"),
    )?;
    assert_string(
        job,
        "runs-on",
        "ubuntu-latest",
        &format!("job {name} runs-on"),
    )?;
    assert_number(
        job,
        "timeout-minutes",
        15,
        &format!("job {name} timeout-minutes"),
    )?;
    assert_permissions(
        mapping(
            value_for_key(job, "permissions", &format!("job {name} permissions"))?,
            &format!("job {name} permissions"),
        )?,
        &[
            ("contents", "read"),
            ("pull-requests", "write"),
            ("issues", "write"),
        ],
        &format!("job {name} permissions"),
    )?;

    let concurrency = mapping(
        value_for_key(job, "concurrency", &format!("job {name} concurrency"))?,
        &format!("job {name} concurrency"),
    )?;
    assert_string(
        concurrency,
        "group",
        &format!("${{{{ github.workflow }}}}-${{{{ github.ref }}}}-{expected_concurrency_suffix}"),
        &format!("job {name} concurrency.group"),
    )?;
    assert_bool(
        concurrency,
        "cancel-in-progress",
        true,
        &format!("job {name} concurrency.cancel-in-progress"),
    )?;

    let steps = sequence(
        value_for_key(job, "steps", &format!("job {name} steps"))?,
        &format!("job {name} steps"),
    )?;
    if steps.len() != 3 {
        bail!("job {name} expected 3 steps, found {}", steps.len());
    }

    let checkout = mapping(&steps[0], &format!("job {name} step 0"))?;
    assert_exact_keys(
        steps[0].as_mapping().expect("step 0 mapping"),
        &["uses"],
        &format!("job {name} step 0"),
    )?;
    assert_string(
        checkout,
        "uses",
        CHECKOUT_USE,
        &format!("job {name} checkout action"),
    )?;

    let toolchain = mapping(&steps[1], &format!("job {name} step 1"))?;
    assert_exact_keys(
        steps[1].as_mapping().expect("step 1 mapping"),
        &["uses"],
        &format!("job {name} step 1"),
    )?;
    assert_string(
        toolchain,
        "uses",
        TOOLCHAIN_USE,
        &format!("job {name} toolchain action"),
    )?;

    let script = mapping(&steps[2], &format!("job {name} step 2"))?;
    assert_exact_keys(
        steps[2].as_mapping().expect("step 2 mapping"),
        &["name", "run"],
        &format!("job {name} step 2"),
    )?;
    assert_string(
        script,
        "name",
        expected_step_name,
        &format!("job {name} step name"),
    )?;
    assert_string(
        script,
        "run",
        expected_run,
        &format!("job {name} run command"),
    )?;
    Ok(())
}

fn assert_permissions(map: &Mapping, expected: &[(&str, &str)], label: &str) -> Result<()> {
    assert_exact_pairs(map, expected, label)
}

fn assert_exact_pairs(map: &Mapping, expected: &[(&str, &str)], label: &str) -> Result<()> {
    if map.len() != expected.len() {
        bail!(
            "{label} expected {} entries, found {}",
            expected.len(),
            map.len()
        );
    }
    for (key, value) in expected {
        assert_string(map, key, value, label)?;
    }
    Ok(())
}

fn assert_exact_strings(actual: &[String], expected: &[&str], label: &str) -> Result<()> {
    let mut actual = actual.to_vec();
    let mut expected = expected.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    actual.sort();
    expected.sort();
    if actual != expected {
        bail!("{label} mismatch: expected {expected:?}, found {actual:?}");
    }
    Ok(())
}

fn assert_exact_keys(map: &Mapping, expected: &[&str], label: &str) -> Result<()> {
    if map.len() != expected.len() {
        bail!(
            "{label} expected {} entries, found {}",
            expected.len(),
            map.len()
        );
    }
    for key in expected {
        if !map.keys().any(|candidate| candidate.as_str() == Some(*key)) {
            bail!("{label} missing key {key}");
        }
    }
    Ok(())
}

fn assert_workflow_root_keys(workflow: &Mapping) -> Result<()> {
    if workflow.len() != 4 {
        bail!("workflow root expected 4 entries, found {}", workflow.len());
    }
    for key in ["name", "permissions", "jobs"] {
        if !workflow
            .keys()
            .any(|candidate| candidate.as_str() == Some(key))
        {
            bail!("workflow root missing key {key}");
        }
    }
    if !workflow
        .keys()
        .any(|candidate| candidate.as_bool() == Some(true) || candidate.as_str() == Some("on"))
    {
        bail!("workflow root missing key on");
    }
    Ok(())
}

fn assert_string(map: &Mapping, key: &str, expected: &str, label: &str) -> Result<()> {
    let actual = value_to_string(value_for_key(map, key, label)?, &format!("{label}.{key}"))?;
    if actual != expected {
        bail!("{label}.{key} mismatch: expected {expected:?}, found {actual:?}");
    }
    Ok(())
}

fn assert_number(map: &Mapping, key: &str, expected: i64, label: &str) -> Result<()> {
    let actual = value_for_key(map, key, label)?
        .as_i64()
        .with_context(|| format!("{label}.{key} is not an integer"))?;
    if actual != expected {
        bail!("{label}.{key} mismatch: expected {expected}, found {actual}");
    }
    Ok(())
}

fn assert_bool(map: &Mapping, key: &str, expected: bool, label: &str) -> Result<()> {
    let actual = value_for_key(map, key, label)?
        .as_bool()
        .with_context(|| format!("{label}.{key} is not a boolean"))?;
    if actual != expected {
        bail!("{label}.{key} mismatch: expected {expected}, found {actual}");
    }
    Ok(())
}

fn on_trigger_types(workflow: &Mapping) -> Result<Vec<String>> {
    let trigger = mapping(
        value_for_key_any(workflow, &["on"], "workflow triggers")?,
        "workflow triggers",
    )?;
    let pull_request_target = mapping(
        value_for_key(
            trigger,
            "pull_request_target",
            "pull_request_target trigger",
        )?,
        "pull_request_target trigger",
    )?;
    let types = sequence(
        value_for_key(pull_request_target, "types", "pull_request_target.types")?,
        "pull_request_target.types",
    )?;
    Ok(types
        .iter()
        .map(|value| value_to_string(value, "pull_request_target.types"))
        .collect::<Result<Vec<_>>>()?)
}

fn mapping<'a>(value: &'a Value, label: &str) -> Result<&'a Mapping> {
    value
        .as_mapping()
        .with_context(|| format!("{label} is not a mapping"))
}

fn sequence<'a>(value: &'a Value, label: &str) -> Result<&'a [Value]> {
    value
        .as_sequence()
        .map(Vec::as_slice)
        .with_context(|| format!("{label} is not a sequence"))
}

fn value_for_key<'a>(map: &'a Mapping, key: &str, label: &str) -> Result<&'a Value> {
    map.iter()
        .find(|(candidate, _)| candidate.as_str() == Some(key))
        .map(|(_, value)| value)
        .with_context(|| format!("missing {label}.{key}"))
}

fn value_for_key_any<'a>(map: &'a Mapping, keys: &[&str], label: &str) -> Result<&'a Value> {
    for key in keys {
        if let Some(value) = map
            .iter()
            .find(|(candidate, _)| candidate.as_str() == Some(*key))
            .map(|(_, value)| value)
        {
            return Ok(value);
        }
        if *key == "on" {
            if let Some(value) = map
                .iter()
                .find(|(candidate, _)| matches!(candidate, Value::Bool(true)))
                .map(|(_, value)| value)
            {
                return Ok(value);
            }
        }
    }
    bail!("missing {label}");
}

fn value_to_string(value: &Value, label: &str) -> Result<String> {
    value
        .as_str()
        .map(ToOwned::to_owned)
        .with_context(|| format!("{label} is not a string"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn workflow_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.github/workflows/pr-standards.yml")
    }

    fn load_workflow() -> Value {
        let text = fs::read_to_string(workflow_path()).expect("read workflow fixture");
        serde_yaml::from_str(&text).expect("parse workflow fixture")
    }

    #[test]
    fn accepts_expected_workflow_contract() {
        validate_workflow(&load_workflow()).expect("workflow contract");
    }

    #[test]
    fn rejects_missing_step_env() {
        let mut workflow = load_workflow();
        let map = workflow.as_mapping_mut().expect("workflow mapping");
        let jobs = map
            .get_mut(&Value::String("jobs".into()))
            .and_then(Value::as_mapping_mut)
            .expect("jobs mapping");
        let job = jobs
            .get_mut(&Value::String("check-standards".into()))
            .and_then(Value::as_mapping_mut)
            .expect("check-standards job mapping");
        let steps = job
            .get_mut(&Value::String("steps".into()))
            .and_then(Value::as_sequence_mut)
            .expect("steps sequence");
        let script = steps[2].as_mapping_mut().expect("script step mapping");
        script.insert(
            Value::String("run".into()),
            Value::String("bash ops/ci/pr-standards.sh".into()),
        );

        validate_workflow(&workflow).expect_err("expected failure");
    }
}
