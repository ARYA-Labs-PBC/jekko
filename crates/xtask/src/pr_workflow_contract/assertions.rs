use anyhow::{bail, Context, Result};
use serde_yaml::{Mapping, Value};

pub(super) fn assert_permissions(
    map: &Mapping,
    expected: &[(&str, &str)],
    label: &str,
) -> Result<()> {
    assert_exact_pairs(map, expected, label)
}

pub(super) fn assert_exact_pairs(
    map: &Mapping,
    expected: &[(&str, &str)],
    label: &str,
) -> Result<()> {
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

pub(super) fn assert_exact_strings(
    actual: &[String],
    expected: &[&str],
    label: &str,
) -> Result<()> {
    let mut actual = actual.to_vec();
    let mut expected = expected.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    actual.sort();
    expected.sort();
    if actual != expected {
        bail!("{label} mismatch: expected {expected:?}, found {actual:?}");
    }
    Ok(())
}

pub(super) fn assert_exact_keys(map: &Mapping, expected: &[&str], label: &str) -> Result<()> {
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

pub(super) fn assert_workflow_root_keys(workflow: &Mapping) -> Result<()> {
    if workflow.len() != 5 {
        bail!("workflow root expected 5 entries, found {}", workflow.len());
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

pub(super) fn assert_string(map: &Mapping, key: &str, expected: &str, label: &str) -> Result<()> {
    let actual = value_to_string(value_for_key(map, key, label)?, &format!("{label}.{key}"))?;
    if actual != expected {
        bail!("{label}.{key} mismatch: expected {expected:?}, found {actual:?}");
    }
    Ok(())
}

pub(super) fn assert_number(map: &Mapping, key: &str, expected: i64, label: &str) -> Result<()> {
    let actual = value_for_key(map, key, label)?
        .as_i64()
        .with_context(|| format!("{label}.{key} is not an integer"))?;
    if actual != expected {
        bail!("{label}.{key} mismatch: expected {expected}, found {actual}");
    }
    Ok(())
}

pub(super) fn assert_bool(map: &Mapping, key: &str, expected: bool, label: &str) -> Result<()> {
    let actual = value_for_key(map, key, label)?
        .as_bool()
        .with_context(|| format!("{label}.{key} is not a boolean"))?;
    if actual != expected {
        bail!("{label}.{key} mismatch: expected {expected}, found {actual}");
    }
    Ok(())
}

pub(super) fn on_trigger_types(workflow: &Mapping) -> Result<Vec<String>> {
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
    types
        .iter()
        .map(|value| value_to_string(value, "pull_request_target.types"))
        .collect::<Result<Vec<_>>>()
}

pub(super) fn mapping<'a>(value: &'a Value, label: &str) -> Result<&'a Mapping> {
    value
        .as_mapping()
        .with_context(|| format!("{label} is not a mapping"))
}

pub(super) fn sequence<'a>(value: &'a Value, label: &str) -> Result<&'a [Value]> {
    value
        .as_sequence()
        .map(Vec::as_slice)
        .with_context(|| format!("{label} is not a sequence"))
}

pub(super) fn value_for_key<'a>(map: &'a Mapping, key: &str, label: &str) -> Result<&'a Value> {
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
