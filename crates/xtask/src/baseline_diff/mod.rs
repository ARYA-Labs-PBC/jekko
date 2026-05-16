use anyhow::Result;
use serde::Serialize;
use std::path::Path;

mod diff;
mod report;

#[cfg(test)]
mod tests;

pub(crate) use diff::baseline_diff_rows;
pub(crate) use report::emit_report;

#[cfg(test)]
pub(crate) use diff::{byte_diff, collect_capture_keys, mismatch_percent};

#[derive(Copy, Clone, Debug, Eq, PartialEq, clap::ValueEnum)]
pub(crate) enum BaselineDiffFormat {
    Text,
    Json,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum BaselineDiffStatus {
    Ok,
    Diff,
    Missing,
}

impl BaselineDiffStatus {
    pub(crate) fn label(self) -> &'static str {
        match self {
            BaselineDiffStatus::Ok => "OK",
            BaselineDiffStatus::Diff => "DIFF",
            BaselineDiffStatus::Missing => "MISSING",
        }
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct BaselineDiffRow {
    pub(crate) key: String,
    pub(crate) baseline_present: bool,
    pub(crate) rust_present: bool,
    pub(crate) bytes_diff: usize,
    pub(crate) bytes_total: usize,
    pub(crate) mismatch_pct: f64,
    pub(crate) status: BaselineDiffStatus,
}

pub(crate) fn baseline_diff(
    baseline_arg: &Path,
    rust_arg: &Path,
    format: BaselineDiffFormat,
    threshold: Option<f64>,
) -> Result<()> {
    let baseline_root = diff::resolve_capture_root(baseline_arg)?;
    let rust_root = diff::resolve_capture_root(rust_arg)?;

    let rows = baseline_diff_rows(&baseline_root, &rust_root)?;
    emit_report(&baseline_root, &rust_root, &rows, format, threshold)
}
