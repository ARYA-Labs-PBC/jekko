use anyhow::{Context, Result};
use std::process::Command as ProcessCommand;

use crate::current_github_event_context;

pub fn run() -> Result<()> {
    let context = current_github_event_context()?;
    let repo = std::env::var("GITHUB_REPOSITORY").context("reading GITHUB_REPOSITORY")?;
    let number = context
        .field("target.number")
        .context("missing target.number")?;
    let association = context
        .field("target.author.association")
        .context("missing target.author.association")?;

    if should_label_contributor(&association) {
        gh_api([
            "--method",
            "POST",
            &format!("/repos/{repo}/issues/{number}/labels"),
            "-f",
            "labels=[\"contributor\"]",
        ])?;
        println!("Applied contributor label to #{number}");
    } else {
        println!("No contributor label needed for #{number} ({association})");
    }

    Ok(())
}

fn should_label_contributor(association: &str) -> bool {
    association == "CONTRIBUTOR"
}

fn gh_api<const N: usize>(args: [&str; N]) -> Result<()> {
    let output = ProcessCommand::new("gh")
        .args(["api"])
        .args(args)
        .output()
        .context("running gh api")?;
    if !output.status.success() {
        anyhow::bail!("gh api {:?} failed with status {}", args, output.status);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contributor_association_labels_only_contributors() {
        assert!(should_label_contributor("CONTRIBUTOR"));
        assert!(!should_label_contributor("MEMBER"));
    }
}
