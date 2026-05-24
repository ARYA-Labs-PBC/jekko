use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde_yaml::Value;

use super::query::{recursive_key_exists, yaml_path};

pub(super) fn discover_super_runbooks(root: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    for dir in ["agent/zyal", "docs/ZYAL/examples"] {
        let path = root.join(dir);
        if !path.is_dir() {
            continue;
        }
        for entry in fs::read_dir(&path).with_context(|| format!("read {}", path.display()))? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("zyal") {
                continue;
            }
            let raw =
                fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
            let body = zyal_yaml_body(&raw).unwrap_or(raw.clone());
            let yaml = serde_yaml::from_str::<Value>(&body).ok();
            if is_superreasoning_runbook(&raw, yaml.as_ref()) {
                out.push(path);
            }
        }
    }
    out.sort();
    Ok(out)
}

pub(super) fn is_superreasoning_runbook(raw: &str, yaml: Option<&Value>) -> bool {
    if let Some(value) = yaml {
        if recursive_key_exists(value, "superreasoning")
            || recursive_key_exists(value, "super_reasoning")
            || yaml_path(value, &["hero_judge", "super_reasoning"]).is_some()
        {
            return true;
        }
    }
    let lower = raw.to_ascii_lowercase();
    lower.contains("superreasoning")
        || lower.contains("super_reasoning")
        || lower.contains("zyal.superreasoning")
}

pub(super) fn zyal_yaml_body(text: &str) -> Result<String> {
    let lines = text.lines().collect::<Vec<_>>();
    let Some((sentinel_idx, first)) = lines.iter().enumerate().find(|(_, line)| {
        let trimmed = line.trim();
        !trimmed.is_empty() && !trimmed.starts_with('#')
    }) else {
        anyhow::bail!("empty ZYAL document");
    };
    if !first.starts_with("<<<ZYAL ") {
        return Ok(text.to_string());
    }
    let mut body = Vec::new();
    for line in lines.into_iter().skip(sentinel_idx + 1) {
        if line.starts_with("<<<END_ZYAL ") {
            return Ok(body.join("\n"));
        }
        body.push(line);
    }
    anyhow::bail!("missing END_ZYAL sentinel")
}
