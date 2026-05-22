use anyhow::{Context, Result};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::shared::repo_root;

use super::{BaselineDiffRow, BaselineDiffStatus};

pub(crate) fn byte_diff(baseline: &[u8], rust: &[u8]) -> (usize, usize) {
    let prefix = baseline.len().min(rust.len());
    let mismatched = baseline
        .iter()
        .zip(rust.iter())
        .take(prefix)
        .filter(|(a, b)| a != b)
        .count();
    let extra = baseline.len().max(rust.len()) - prefix;
    let total = baseline.len().max(rust.len());
    (mismatched + extra, total)
}

pub(crate) fn mismatch_percent(bytes_diff: usize, bytes_total: usize) -> f64 {
    if bytes_total == 0 {
        return 0.0;
    }
    (bytes_diff as f64 / bytes_total as f64) * 100.0
}

pub(crate) fn collect_capture_keys(root: &Path) -> Result<BTreeMap<String, PathBuf>> {
    let mut out = BTreeMap::new();
    if !root.exists() {
        return Ok(out);
    }
    for screen_entry in
        fs::read_dir(root).with_context(|| format!("read capture root {}", root.display()))?
    {
        let screen_entry = screen_entry?;
        if !screen_entry.file_type()?.is_dir() {
            continue;
        }
        let screen_name = match screen_entry.file_name().into_string() {
            Ok(s) => s,
            Err(_) => continue,
        };
        let screen_path = screen_entry.path();
        for file_entry in fs::read_dir(&screen_path)
            .with_context(|| format!("read screen dir {}", screen_path.display()))?
        {
            let file_entry = file_entry?;
            if !file_entry.file_type()?.is_file() {
                continue;
            }
            let file_name = match file_entry.file_name().into_string() {
                Ok(s) => s,
                Err(_) => continue,
            };
            let Some(stem) = file_name.strip_suffix(".txt") else {
                continue;
            };
            if stem.ends_with("-boot-timeout") {
                continue;
            }
            let key = format!("{screen_name}/{stem}");
            out.insert(key, file_entry.path());
        }
    }
    Ok(out)
}

pub(crate) fn baseline_diff_rows(
    baseline_root: &Path,
    rust_root: &Path,
) -> Result<Vec<BaselineDiffRow>> {
    let baseline_map = collect_capture_keys(baseline_root)?;
    let rust_map = collect_capture_keys(rust_root)?;

    let mut keys: Vec<&String> = baseline_map.keys().chain(rust_map.keys()).collect();
    keys.sort();
    keys.dedup();

    let mut rows = Vec::with_capacity(keys.len());
    for key in keys {
        let baseline_path = baseline_map.get(key);
        let rust_path = rust_map.get(key);
        let baseline_present = baseline_path.is_some();
        let rust_present = rust_path.is_some();

        let (bytes_diff, bytes_total, status) = match (baseline_path, rust_path) {
            (Some(b), Some(r)) => {
                let baseline_bytes = fs::read(b)
                    .with_context(|| format!("read baseline capture {}", b.display()))?;
                let rust_bytes =
                    fs::read(r).with_context(|| format!("read rust capture {}", r.display()))?;
                let (bd, bt) = byte_diff(&baseline_bytes, &rust_bytes);
                let status = if bd == 0 {
                    BaselineDiffStatus::Ok
                } else {
                    BaselineDiffStatus::Diff
                };
                (bd, bt, status)
            }
            (Some(b), None) => {
                let baseline_bytes = fs::read(b)
                    .with_context(|| format!("read baseline capture {}", b.display()))?;
                let total = baseline_bytes.len();
                (total, total, BaselineDiffStatus::Missing)
            }
            (None, Some(r)) => {
                let rust_bytes =
                    fs::read(r).with_context(|| format!("read rust capture {}", r.display()))?;
                let total = rust_bytes.len();
                (total, total, BaselineDiffStatus::Missing)
            }
            (None, None) => (0, 0, BaselineDiffStatus::Missing),
        };

        let mismatch_pct = mismatch_percent(bytes_diff, bytes_total);
        rows.push(BaselineDiffRow {
            key: key.clone(),
            baseline_present,
            rust_present,
            bytes_diff,
            bytes_total,
            mismatch_pct,
            status,
        });
    }
    Ok(rows)
}

pub(crate) fn resolve_capture_root(arg: &Path) -> Result<PathBuf> {
    if arg.is_absolute() {
        return Ok(arg.to_path_buf());
    }
    Ok(repo_root()?.join(arg))
}
