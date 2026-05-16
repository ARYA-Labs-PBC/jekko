use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command as ProcessCommand;

const DEFAULT_REPOSITORY: &str = "neverhuman/jekko";
const DEFAULT_SHA: &str = "HEAD";
const DEFAULT_BUMP: &str = "";
const DEFAULT_RELEASE_NOTES: &str = "No notable changes";
const DEFAULT_TEMP_DIR: &str = "/tmp";

#[derive(Debug, Deserialize)]
struct GitHubReleaseView {
    #[serde(rename = "tagName")]
    tag_name: String,
    #[serde(rename = "databaseId")]
    database_id: u64,
}

pub fn run() -> Result<()> {
    let repo = match env::var("GH_REPO") {
        Ok(value) => value,
        Err(_) => DEFAULT_REPOSITORY.to_string(),
    };
    let channel = resolve_channel()?;
    let preview = channel != "latest";
    let version = resolve_version(&channel, preview)?;
    let sha = match env::var("GITHUB_SHA") {
        Ok(value) => value,
        Err(_) => match git_sha() {
            Ok(value) => value,
            Err(_) => DEFAULT_SHA.to_string(),
        },
    };

    let mut output = vec![format!("version={version}")];

    if !preview {
        let notes = release_notes();
        let release = create_release(&version, &sha, &notes)?;
        output.push(format!("release={}", release.database_id));
        output.push(format!("tag={}", release.tag_name));
    } else if channel == "beta" {
        let release = create_beta_release(&version, &repo)?;
        output.push(format!("release={}", release.database_id));
        output.push(format!("tag={}", release.tag_name));
    }

    output.push(format!("repo={repo}"));
    write_outputs(&output)?;
    for line in output {
        println!("{line}");
    }
    Ok(())
}

fn resolve_channel() -> Result<String> {
    if let Ok(channel) = env::var("JEKKO_CHANNEL") {
        return Ok(channel);
    }
    if env::var("JEKKO_BUMP").is_ok() {
        return Ok("latest".to_string());
    }
    if let Ok(version) = env::var("JEKKO_VERSION") {
        if !version.starts_with("0.0.0-") {
            return Ok("latest".to_string());
        }
    }
    git_branch()
}

fn resolve_version(channel: &str, preview: bool) -> Result<String> {
    if let Ok(version) = env::var("JEKKO_VERSION") {
        return Ok(version);
    }

    if preview {
        let stamp = chrono::Utc::now().format("%Y%m%d%H%M").to_string();
        return Ok(format!("0.0.0-{channel}-{stamp}"));
    }

    let latest = npm_latest_version()?;
    let (major, minor, patch) = parse_version(&latest)?;
    let bump = match env::var("JEKKO_BUMP") {
        Ok(value) => value,
        Err(_) => DEFAULT_BUMP.to_string(),
    };
    match bump.to_lowercase().as_str() {
        "major" => Ok(format!("{}.0.0", major + 1)),
        "minor" => Ok(format!("{}.{}.0", major, minor + 1)),
        _ => Ok(format!("{}.{}.{}", major, minor, patch + 1)),
    }
}

fn parse_version(version: &str) -> Result<(u64, u64, u64)> {
    let mut parts = version.split('.');
    let major = parts
        .next()
        .context("missing major version")?
        .parse::<u64>()
        .with_context(|| format!("parse major version from {version}"))?;
    let minor = parts
        .next()
        .context("missing minor version")?
        .parse::<u64>()
        .with_context(|| format!("parse minor version from {version}"))?;
    let patch_piece = parts.next().context("missing patch version")?;
    let patch_text = patch_piece
        .split_once('-')
        .map(|(head, _)| head)
        .unwrap_or(patch_piece);
    let patch = patch_text
        .parse::<u64>()
        .with_context(|| format!("parse patch version from {version}"))?;
    Ok((major, minor, patch))
}

fn npm_latest_version() -> Result<String> {
    let output = ProcessCommand::new("curl")
        .args(["-fsSL", "https://registry.npmjs.org/jekko-ai/latest"])
        .output()
        .context("fetch latest jekko-ai version from npm registry")?;
    if !output.status.success() {
        bail!("curl failed with status {}", output.status);
    }
    let parsed: serde_json::Value =
        serde_json::from_slice(&output.stdout).context("parse npm registry response")?;
    parsed
        .get("version")
        .and_then(serde_json::Value::as_str)
        .map(|value| value.to_string())
        .context("npm registry response missing version")
}

fn git_branch() -> Result<String> {
    let output = ProcessCommand::new("git")
        .args(["branch", "--show-current"])
        .output()
        .context("run `git branch --show-current`")?;
    if !output.status.success() {
        bail!(
            "git branch --show-current failed with status {}",
            output.status
        );
    }
    Ok(String::from_utf8(output.stdout)
        .context("decode git branch output")?
        .trim()
        .to_string())
}

fn git_sha() -> Result<String> {
    let output = ProcessCommand::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .context("run `git rev-parse HEAD`")?;
    if !output.status.success() {
        bail!("git rev-parse HEAD failed with status {}", output.status);
    }
    Ok(String::from_utf8(output.stdout)
        .context("decode git sha output")?
        .trim()
        .to_string())
}

fn release_notes() -> String {
    match fs::read_to_string(PathBuf::from("UPCOMING_CHANGELOG.md")) {
        Ok(value) => value,
        Err(_) => DEFAULT_RELEASE_NOTES.to_string(),
    }
}

fn create_release(version: &str, sha: &str, notes: &str) -> Result<GitHubReleaseView> {
    let temp_dir = match env::var("RUNNER_TEMP") {
        Ok(value) => value,
        Err(_) => DEFAULT_TEMP_DIR.to_string(),
    };
    let notes_file = PathBuf::from(temp_dir).join("jekko-release-notes.txt");
    fs::write(&notes_file, notes).with_context(|| format!("write {}", notes_file.display()))?;

    let status = ProcessCommand::new("gh")
        .args([
            "release",
            "create",
            &format!("v{version}"),
            "-d",
            "--target",
            sha,
            "--title",
            &format!("v{version}"),
            "--notes-file",
            notes_file
                .to_str()
                .context("notes file path is not valid UTF-8")?,
        ])
        .status()
        .context("run gh release create")?;
    if !status.success() {
        bail!("gh release create failed with status {}", status);
    }

    view_release(version, None)
}

fn create_beta_release(version: &str, repo: &str) -> Result<GitHubReleaseView> {
    let status = ProcessCommand::new("gh")
        .args([
            "release",
            "create",
            &format!("v{version}"),
            "-d",
            "--title",
            &format!("v{version}"),
            "--repo",
            repo,
        ])
        .status()
        .context("run gh release create for beta")?;
    if !status.success() {
        bail!("gh release create for beta failed with status {}", status);
    }

    view_release(version, Some(repo))
}

fn view_release(version: &str, repo: Option<&str>) -> Result<GitHubReleaseView> {
    let mut cmd = ProcessCommand::new("gh");
    cmd.args([
        "release",
        "view",
        &format!("v{version}"),
        "--json",
        "tagName,databaseId",
    ]);
    if let Some(repo) = repo {
        cmd.args(["--repo", repo]);
    }
    let output = cmd.output().context("run gh release view")?;
    if !output.status.success() {
        bail!("gh release view failed with status {}", output.status);
    }
    serde_json::from_slice(&output.stdout).context("parse gh release view output")
}

fn write_outputs(lines: &[String]) -> Result<()> {
    let Some(path) = env::var_os("GITHUB_OUTPUT") else {
        return Ok(());
    };
    fs::write(&path, lines.join("\n")).context("write GITHUB_OUTPUT")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_version() {
        assert_eq!(parse_version("1.2.3").unwrap(), (1, 2, 3));
    }

    #[test]
    fn parses_prerelease_patch() {
        assert_eq!(parse_version("1.2.3-beta.4").unwrap(), (1, 2, 3));
    }

    #[test]
    fn bumps_versions() {
        assert_eq!(bump_version("1.2.3", "major"), "2.0.0");
        assert_eq!(bump_version("1.2.3", "minor"), "1.3.0");
        assert_eq!(bump_version("1.2.3", "patch"), "1.2.4");
    }

    fn bump_version(latest: &str, bump: &str) -> String {
        let (major, minor, patch) = parse_version(latest).unwrap();
        match bump {
            "major" => format!("{}.0.0", major + 1),
            "minor" => format!("{}.{}.0", major, minor + 1),
            _ => format!("{}.{}.{}", major, minor, patch + 1),
        }
    }
}
