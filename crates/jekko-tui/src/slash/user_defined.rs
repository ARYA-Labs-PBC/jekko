//! User-defined slash commands loader (COWBOY.md I3).
//!
//! Reads `.jankurai/commands/*.md` Claude-style skill files. Each file's name
//! (without `.md`) becomes the command id. The file body must start with a
//! YAML-ish frontmatter block delimited by `---` lines that contains at
//! minimum a `description:` key. Everything after the closing `---` is the
//! command body (passed verbatim to the runtime when the command fires).
//!
//! This module is pure: it touches the filesystem and produces a
//! [`LoadReport`] describing the commands it could parse plus any per-file
//! errors. Runtime wire-up (popup registration, dispatch) is owned by
//! [`crate::slash`] consumers — kept out of here so I1 can land
//! independently.

use std::fs;
use std::path::{Path, PathBuf};

/// A successfully-parsed user-defined slash command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserCommand {
    pub id: String,
    pub description: String,
    pub body: String,
    pub source: PathBuf,
}

/// Result of loading a directory: parsed commands plus any per-file errors.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct LoadReport {
    pub commands: Vec<UserCommand>,
    pub errors: Vec<LoadError>,
}

/// Per-file load failure. Non-fatal: the loader keeps walking siblings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadError {
    pub path: PathBuf,
    pub reason: String,
}

/// Loads `<workspace_root>/.jankurai/commands/*.md`. Missing directory is not
/// an error — the caller gets an empty report.
pub fn load_from_workspace(workspace_root: impl AsRef<Path>) -> LoadReport {
    let dir = workspace_root.as_ref().join(".jankurai").join("commands");
    load_from_dir(dir)
}

/// Loads `*.md` files from `dir`. Output commands sorted by id for stable
/// popup ordering; errors sorted by path for the same reason.
pub fn load_from_dir(dir: impl AsRef<Path>) -> LoadReport {
    let dir = dir.as_ref();
    let mut report = LoadReport::default();

    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        // WHY: missing dir is the common case (no user commands installed),
        // not a user-visible error.
        Err(_) => return report,
    };

    let mut md_paths: Vec<PathBuf> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        match path.extension().and_then(|s| s.to_str()) {
            Some("md") => md_paths.push(path),
            _ => continue,
        }
    }

    md_paths.sort();

    for path in md_paths {
        match parse_file(&path) {
            Ok(cmd) => report.commands.push(cmd),
            Err(reason) => report.errors.push(LoadError { path, reason }),
        }
    }

    report.commands.sort_by(|a, b| a.id.cmp(&b.id));
    report
}

fn parse_file(path: &Path) -> Result<UserCommand, String> {
    let id = path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| "filename is not valid UTF-8".to_string())?
        .to_string();

    if !is_valid_id(&id) {
        return Err(format!(
            "invalid command id `{id}` (lowercase letters, digits, `-`, `_` only)"
        ));
    }

    let raw = fs::read_to_string(path).map_err(|e| format!("read failed: {e}"))?;
    let (description, body) = parse_frontmatter(&raw)?;

    Ok(UserCommand {
        id,
        description,
        body,
        source: path.to_path_buf(),
    })
}

fn parse_frontmatter(input: &str) -> Result<(String, String), String> {
    let mut lines = input.lines();
    let first = lines
        .next()
        .ok_or_else(|| "missing frontmatter".to_string())?;
    if first.trim() != "---" {
        return Err("missing frontmatter (expected `---` on first line)".to_string());
    }

    let mut description: Option<String> = None;
    let mut closed = false;
    let mut body_start: usize = 0;
    let mut consumed = first.len() + 1; // +1 for the newline

    for line in lines.by_ref() {
        let line_len = line.len() + 1; // include trailing newline
        if line.trim() == "---" {
            closed = true;
            body_start = consumed + line_len;
            break;
        }

        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim();
            let value = value.trim();
            if key == "description" {
                description = Some(unquote(value).to_string());
            }
        }
        consumed += line_len;
    }

    if !closed {
        return Err("frontmatter not closed (expected trailing `---`)".to_string());
    }

    let description =
        description.ok_or_else(|| "missing `description` in frontmatter".to_string())?;
    if description.is_empty() {
        return Err("`description` is empty".to_string());
    }

    let body = if body_start >= input.len() {
        String::new()
    } else {
        input[body_start..].trim_start_matches('\n').to_string()
    };

    Ok((description, body))
}

fn unquote(value: &str) -> &str {
    let bytes = value.as_bytes();
    if bytes.len() >= 2
        && ((bytes[0] == b'"' && bytes[bytes.len() - 1] == b'"')
            || (bytes[0] == b'\'' && bytes[bytes.len() - 1] == b'\''))
    {
        &value[1..value.len() - 1]
    } else {
        value
    }
}

fn is_valid_id(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    s.chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write(dir: &Path, name: &str, contents: &str) -> PathBuf {
        let path = dir.join(name);
        fs::write(&path, contents).expect("write fixture");
        path
    }

    #[test]
    fn loads_valid_command() {
        let tmp = TempDir::new().unwrap();
        write(
            tmp.path(),
            "deploy.md",
            "---\ndescription: Ship the current branch\n---\nrun deploy.sh now\n",
        );

        let report = load_from_dir(tmp.path());
        assert!(report.errors.is_empty(), "errors: {:?}", report.errors);
        assert_eq!(report.commands.len(), 1);
        let cmd = &report.commands[0];
        assert_eq!(cmd.id, "deploy");
        assert_eq!(cmd.description, "Ship the current branch");
        assert_eq!(cmd.body, "run deploy.sh now\n");
    }

    #[test]
    fn missing_description_is_error() {
        let tmp = TempDir::new().unwrap();
        let path = write(tmp.path(), "broken.md", "---\nfoo: bar\n---\nbody\n");

        let report = load_from_dir(tmp.path());
        assert!(report.commands.is_empty());
        assert_eq!(report.errors.len(), 1);
        assert_eq!(report.errors[0].path, path);
        assert!(
            report.errors[0].reason.contains("description"),
            "reason: {}",
            report.errors[0].reason
        );
    }

    #[test]
    fn skips_non_md_files() {
        let tmp = TempDir::new().unwrap();
        write(
            tmp.path(),
            "ok.md",
            "---\ndescription: keep me\n---\nbody\n",
        );
        write(tmp.path(), "README.txt", "ignored");
        write(tmp.path(), "notes", "ignored too");
        write(tmp.path(), "script.sh", "#!/bin/sh\n");

        let report = load_from_dir(tmp.path());
        assert!(report.errors.is_empty());
        assert_eq!(report.commands.len(), 1);
        assert_eq!(report.commands[0].id, "ok");
    }

    #[test]
    fn missing_dir_returns_empty_report() {
        let tmp = TempDir::new().unwrap();
        let missing = tmp.path().join("does-not-exist");

        let report = load_from_dir(&missing);
        assert!(report.commands.is_empty());
        assert!(report.errors.is_empty());
    }

    #[test]
    fn invalid_id_is_error() {
        let tmp = TempDir::new().unwrap();
        let path = write(
            tmp.path(),
            "Bad Name!.md",
            "---\ndescription: should fail on id\n---\nbody\n",
        );

        let report = load_from_dir(tmp.path());
        assert!(report.commands.is_empty());
        assert_eq!(report.errors.len(), 1);
        assert_eq!(report.errors[0].path, path);
        assert!(
            report.errors[0].reason.contains("invalid command id"),
            "reason: {}",
            report.errors[0].reason
        );
    }

    #[test]
    fn frontmatter_with_quoted_description() {
        let tmp = TempDir::new().unwrap();
        write(
            tmp.path(),
            "quoted.md",
            "---\ndescription: \"Quoted desc with: colon\"\n---\nbody\n",
        );

        let report = load_from_dir(tmp.path());
        assert!(report.errors.is_empty());
        assert_eq!(report.commands.len(), 1);
        assert_eq!(report.commands[0].description, "Quoted desc with: colon");
    }

    #[test]
    fn sorted_alphabetically() {
        let tmp = TempDir::new().unwrap();
        write(tmp.path(), "zeta.md", "---\ndescription: z\n---\n");
        write(tmp.path(), "alpha.md", "---\ndescription: a\n---\n");
        write(tmp.path(), "mango.md", "---\ndescription: m\n---\n");

        let report = load_from_dir(tmp.path());
        let ids: Vec<&str> = report.commands.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids, vec!["alpha", "mango", "zeta"]);
    }

    #[test]
    fn no_frontmatter_is_error() {
        let tmp = TempDir::new().unwrap();
        let path = write(tmp.path(), "plain.md", "just a body, no frontmatter\n");

        let report = load_from_dir(tmp.path());
        assert!(report.commands.is_empty());
        assert_eq!(report.errors.len(), 1);
        assert_eq!(report.errors[0].path, path);
        assert!(
            report.errors[0].reason.contains("frontmatter"),
            "reason: {}",
            report.errors[0].reason
        );
    }
}
