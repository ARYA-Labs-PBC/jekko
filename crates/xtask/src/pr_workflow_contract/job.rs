use anyhow::{bail, Result};
use serde_yaml::Mapping;

use super::assertions::{
    assert_bool, assert_exact_keys, assert_number, assert_permissions, assert_string, mapping,
    sequence, value_for_key,
};
use super::{CHECKOUT_USE, TOOLCHAIN_USE};

pub(super) fn assert_job(
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
    assert_job_header(job, name, expected_concurrency_suffix)?;
    assert_job_steps(job, name, expected_step_name, expected_run)
}

fn assert_job_header(job: &Mapping, name: &str, expected_concurrency_suffix: &str) -> Result<()> {
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
    )
}

fn assert_job_steps(
    job: &Mapping,
    name: &str,
    expected_step_name: &str,
    expected_run: &str,
) -> Result<()> {
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
    )
}
