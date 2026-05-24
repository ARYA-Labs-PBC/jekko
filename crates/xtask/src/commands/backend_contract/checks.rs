use std::collections::BTreeMap;

use anyhow::{bail, Result};

use crate::commands::parity_diff::SetDiff;

pub(super) fn check_subset(
    label: &str,
    actual: &[String],
    expected: &[String],
    strict: bool,
) -> Result<()> {
    let diff = SetDiff::compute(actual.to_vec(), expected.to_vec());
    if diff.removed.is_empty() {
        println!(
            "backend-contract: {} ✓ {} expected item(s) covered, {} extra current item(s)",
            label,
            expected.len(),
            diff.added.len()
        );
        return Ok(());
    }

    println!(
        "backend-contract: {} missing {} expected item(s) and has {} extra current item(s)",
        label,
        diff.removed.len(),
        diff.added.len()
    );
    for item in &diff.removed {
        println!("  - {item}");
    }
    for item in &diff.added {
        println!("  + {item}");
    }
    if strict {
        bail!("backend-contract: {label} missing expected item(s)");
    }
    Ok(())
}

pub(super) fn check_openapi(
    actual: &BTreeMap<String, Vec<String>>,
    expected: &BTreeMap<String, Vec<String>>,
    strict: bool,
) -> Result<()> {
    let mut missing_paths = Vec::new();
    let mut missing_methods = Vec::new();
    for (path, methods) in expected {
        match actual.get(path) {
            Some(current) => {
                let diff = SetDiff::compute(current.clone(), methods.clone());
                if !diff.removed.is_empty() {
                    missing_methods.push((path.clone(), diff.removed));
                }
            }
            None => missing_paths.push(path.clone()),
        }
    }

    if missing_paths.is_empty() && missing_methods.is_empty() {
        println!(
            "backend-contract: OpenAPI ✓ {} expected path(s) covered, {} total current path(s)",
            expected.len(),
            actual.len()
        );
        return Ok(());
    }

    if !missing_paths.is_empty() {
        println!("backend-contract: OpenAPI missing paths:");
        for path in &missing_paths {
            println!("  - {path}");
        }
    }
    if !missing_methods.is_empty() {
        println!("backend-contract: OpenAPI missing methods:");
        for (path, methods) in &missing_methods {
            println!("  - {path}: {}", methods.join(", "));
        }
    }

    if strict {
        bail!("backend-contract: OpenAPI missing expected path(s) or method(s)");
    }
    Ok(())
}

pub(super) fn check_migration_count(actual: usize, expected: usize, strict: bool) -> Result<()> {
    if actual >= expected {
        println!(
            "backend-contract: migrations ✓ current {} >= expected {}",
            actual, expected
        );
        return Ok(());
    }

    println!(
        "backend-contract: migrations missing {} embedded migration(s) (current {}, expected {})",
        expected - actual,
        actual,
        expected
    );
    if strict {
        bail!("backend-contract: migration count shrank");
    }
    Ok(())
}
