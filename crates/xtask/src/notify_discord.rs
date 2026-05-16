use anyhow::{Context, Result};
use serde_json::json;
use std::process::Command as ProcessCommand;

use crate::current_github_event_context;

const DEFAULT_RELEASE_NAME: &str = "release";
const MISSING_FIELD_DEFAULT: &str = "";

pub fn run() -> Result<()> {
    let webhook = std::env::var("DISCORD_WEBHOOK").context("DISCORD_WEBHOOK must be set")?;
    let context = current_github_event_context()?;
    let release_name = match context.field("release.name") {
        Some(value) => value,
        None => match context.field("release.tag_name") {
            Some(value) => value,
            None => DEFAULT_RELEASE_NAME.to_string(),
        },
    };
    let tag_name = match context.field("release.tag_name") {
        Some(value) => value,
        None => MISSING_FIELD_DEFAULT.to_string(),
    };
    let release_url = match context.field("release.html_url") {
        Some(value) => value,
        None => MISSING_FIELD_DEFAULT.to_string(),
    };

    let content = format_release_content(&release_name, &tag_name, &release_url);
    let payload = json!({ "content": content }).to_string();

    let output = ProcessCommand::new("curl")
        .args([
            "-fsSL",
            "-H",
            "Content-Type: application/json",
            "-d",
            &payload,
            &webhook,
        ])
        .output()
        .context("running curl for Discord webhook")?;
    if !output.status.success() {
        anyhow::bail!("curl failed with status {}", output.status);
    }
    Ok(())
}

fn format_release_content(release_name: &str, tag_name: &str, release_url: &str) -> String {
    if release_url.is_empty() {
        format!("Published {release_name} ({tag_name})")
    } else {
        format!("Published {release_name} ({tag_name}) - {release_url}")
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn formats_release_message_with_url() {
        let content = super::format_release_content("v1.2.3", "v1.2.3", "https://example.com");
        assert_eq!(content, "Published v1.2.3 (v1.2.3) - https://example.com");
        assert_eq!(
            super::format_release_content("release", "", ""),
            "Published release ()"
        );
    }
}
