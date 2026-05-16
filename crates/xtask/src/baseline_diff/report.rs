use anyhow::{bail, Context, Result};
use std::path::Path;

use super::{BaselineDiffFormat, BaselineDiffRow, BaselineDiffStatus};

pub(crate) fn emit_report(
    baseline_root: &Path,
    rust_root: &Path,
    rows: &[BaselineDiffRow],
    format: BaselineDiffFormat,
    threshold: Option<f64>,
) -> Result<()> {
    match format {
        BaselineDiffFormat::Text => print_text(baseline_root, rust_root, rows),
        BaselineDiffFormat::Json => print_json(rows)?,
    }

    if let Some(t) = threshold {
        enforce_threshold(rows, t)?;
    }

    Ok(())
}

fn print_text(baseline_root: &Path, rust_root: &Path, rows: &[BaselineDiffRow]) {
    println!(
        "# baseline-diff baseline={} rust={}",
        baseline_root.display(),
        rust_root.display()
    );
    for row in rows {
        println!(
            "{:32}  bytes_diff={:<7} mismatch={:>6.2}%  status={}",
            row.key,
            row.bytes_diff,
            row.mismatch_pct,
            row.status.label(),
        );
    }
    let summary_ok = rows
        .iter()
        .filter(|r| r.status == BaselineDiffStatus::Ok)
        .count();
    let summary_diff = rows
        .iter()
        .filter(|r| r.status == BaselineDiffStatus::Diff)
        .count();
    let summary_missing = rows
        .iter()
        .filter(|r| r.status == BaselineDiffStatus::Missing)
        .count();
    println!(
        "# summary: total={} ok={} diff={} missing={}",
        rows.len(),
        summary_ok,
        summary_diff,
        summary_missing,
    );
}

fn print_json(rows: &[BaselineDiffRow]) -> Result<()> {
    let json =
        serde_json::to_string_pretty(rows).context("serialise baseline-diff rows as JSON")?;
    println!("{json}");
    Ok(())
}

fn enforce_threshold(rows: &[BaselineDiffRow], threshold: f64) -> Result<()> {
    let over: Vec<&BaselineDiffRow> = rows.iter().filter(|r| r.mismatch_pct > threshold).collect();
    if over.is_empty() {
        return Ok(());
    }
    for row in &over {
        eprintln!(
            "baseline-diff: {} mismatch={:.2}% exceeds threshold {:.2}%",
            row.key, row.mismatch_pct, threshold,
        );
    }
    bail!(
        "{} captures exceed mismatch threshold {:.2}%",
        over.len(),
        threshold,
    );
}
