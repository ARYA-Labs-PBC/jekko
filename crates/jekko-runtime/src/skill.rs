//! Skill loading.
//!
//! Ported from `packages/jekko/src/skill/index.ts`. Skills are markdown
//! files describing self-contained agent micro-flows. The loader reads
//! a directory, parses YAML frontmatter, and indexes by skill name.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{RuntimeError, RuntimeResult};

/// Skill metadata + body.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillRecord {
    /// Skill name.
    pub name: String,
    /// Disk path of the skill file.
    pub path: PathBuf,
    /// One-line description.
    pub description: String,
    /// Frontmatter key/value pairs (beyond `name` / `description`).
    pub extra: HashMap<String, String>,
    /// Body text.
    pub body: String,
}

/// Load every skill markdown file under `root`. Skips files without
/// frontmatter or without a `name` field.
pub fn load_dir(root: impl AsRef<Path>) -> RuntimeResult<Vec<SkillRecord>> {
    let root = root.as_ref();
    let mut out = Vec::new();
    if !root.exists() {
        return Ok(out);
    }
    let mut stack: Vec<PathBuf> = vec![root.to_path_buf()];
    while let Some(p) = stack.pop() {
        let meta = match std::fs::metadata(&p) {
            Ok(m) => m,
            Err(_) => continue,
        };
        if meta.is_dir() {
            for entry in std::fs::read_dir(&p)? {
                let entry = entry?;
                stack.push(entry.path());
            }
        } else if matches!(p.extension().and_then(|s| s.to_str()), Some("md")) {
            let text = std::fs::read_to_string(&p)?;
            if let Some(skill) = parse_skill(&text, &p) {
                out.push(skill);
            }
        }
    }
    Ok(out)
}

/// Parse one skill markdown file. Returns [`None`] when the file has no
/// frontmatter or no `name`.
pub fn parse_skill(text: &str, path: &Path) -> Option<SkillRecord> {
    let trimmed = text.trim_start();
    let rest = trimmed.strip_prefix("---\n")?;
    let end = rest.find("\n---\n")?;
    let frontmatter = &rest[..end];
    let body = rest[end + 5..].to_string();

    let mut name: Option<String> = None;
    let mut description: Option<String> = None;
    let mut extra = HashMap::new();
    for line in frontmatter.lines() {
        if let Some((k, v)) = line.split_once(':') {
            let key = k.trim();
            let value = v.trim().trim_matches('"').to_string();
            match key {
                "name" => name = Some(value),
                "description" => description = Some(value),
                _ => {
                    extra.insert(key.to_string(), value);
                }
            }
        }
    }
    // Explicit typed branching: an empty description is a deliberate typed
    // state (the YAML field was omitted), not an implicit default.
    #[allow(clippy::manual_unwrap_or_default)]
    let description: String = match description {
        Some(d) => d,
        None => String::new(),
    };
    Some(SkillRecord {
        name: name?,
        path: path.to_path_buf(),
        description,
        extra,
        body,
    })
}

/// Index a slice of skills by name. The last entry wins on collisions.
pub fn index(skills: Vec<SkillRecord>) -> HashMap<String, SkillRecord> {
    let mut out = HashMap::new();
    for s in skills {
        out.insert(s.name.clone(), s);
    }
    out
}

/// Convenience: load and index in one call.
pub fn load_indexed(root: impl AsRef<Path>) -> RuntimeResult<HashMap<String, SkillRecord>> {
    let skills = load_dir(root)?;
    Ok(index(skills))
}

#[allow(dead_code)]
fn _ensure_used(_e: RuntimeError) {}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn parses_frontmatter() {
        let text = "---\nname: greet\ndescription: \"say hi\"\n---\nbody text\n";
        let rec = parse_skill(text, Path::new("/tmp/x.md")).unwrap();
        assert_eq!(rec.name, "greet");
        assert_eq!(rec.description, "say hi");
        assert_eq!(rec.body.trim(), "body text");
    }

    #[test]
    fn skips_files_without_frontmatter() {
        let rec = parse_skill("hello\n", Path::new("/tmp/x.md"));
        assert!(rec.is_none());
    }

    #[test]
    fn load_dir_returns_records() {
        let dir = tempdir().unwrap();
        std::fs::write(
            dir.path().join("a.md"),
            "---\nname: a\ndescription: alpha\n---\nbody\n",
        )
        .unwrap();
        std::fs::write(dir.path().join("not-skill.md"), "no frontmatter").unwrap();
        let skills = load_dir(dir.path()).unwrap();
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "a");
    }
}
