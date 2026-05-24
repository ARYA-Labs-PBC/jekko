use anyhow::Result;
use serde_yaml::Value;

use super::assertions::{
    assert_exact_strings, assert_permissions, assert_string, assert_workflow_root_keys, mapping,
    on_trigger_types, value_for_key,
};
use super::job::assert_job;
use super::{EXPECTED_TRIGGER_TYPES, EXPECTED_WORKFLOW_NAME};

pub(super) fn validate_workflow(workflow: &Value) -> Result<()> {
    let workflow_map = mapping(workflow, "workflow root")?;
    assert_workflow_root_keys(workflow_map)?;
    assert_string(
        workflow_map,
        "name",
        EXPECTED_WORKFLOW_NAME,
        "workflow name",
    )?;
    assert_permissions(
        mapping(
            value_for_key(workflow_map, "env", "workflow env")?,
            "workflow env",
        )?,
        &[
            ("GH_TOKEN", "${{ github.token }}"),
            ("GITHUB_TOKEN", "${{ github.token }}"),
            ("GH_REPO", "${{ github.repository }}"),
            ("GITHUB_REPOSITORY", "${{ github.repository }}"),
            ("GITHUB_BASE_REF", "${{ github.base_ref }}"),
            ("GITHUB_HEAD_REF", "${{ github.head_ref }}"),
            ("GITHUB_EVENT_PATH", "${{ github.event_path }}"),
        ],
        "workflow env",
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
        "bash ops/ci/pr-policy.sh standards",
        "check-standards",
    )?;
    assert_job(
        jobs,
        "check-compliance",
        "Check PR template compliance",
        "bash ops/ci/pr-policy.sh compliance",
        "check-compliance",
    )?;
    Ok(())
}
