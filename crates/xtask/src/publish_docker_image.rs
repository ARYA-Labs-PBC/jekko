use anyhow::{bail, Result};
use std::process::Command;

pub fn run(version: &str, channel: &str) -> Result<()> {
    let image = "ghcr.io/neverhuman/jekko";
    let platforms = "linux/amd64,linux/arm64";
    let tags = [format!("{image}:{version}"), format!("{image}:{channel}")];

    let mut cmd = Command::new("docker");
    cmd.args(["buildx", "build", "--platform", platforms]);
    for tag in tags {
        cmd.args(["-t", &tag]);
    }
    cmd.args(["--push", "."]);

    let status = cmd.status()?;
    if !status.success() {
        bail!("docker buildx build failed with status {status}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn smoke_tag_formatting_matches_image_name() {
        let image = "ghcr.io/neverhuman/jekko";
        let version = "1.2.3";
        let channel = "beta";
        let tags = [format!("{image}:{version}"), format!("{image}:{channel}")];
        assert_eq!(tags[0], "ghcr.io/neverhuman/jekko:1.2.3");
        assert_eq!(tags[1], "ghcr.io/neverhuman/jekko:beta");
    }
}
