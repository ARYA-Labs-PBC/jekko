use anyhow::Result;
use std::path::Path;

pub fn run(_repo_root: &Path, version: &str, channel: &str) -> Result<()> {
    println!(
        "publish-release-artifacts: binary-only release flow for v{version} ({channel}); use `xtask release package` and `xtask release attach`"
    );
    Ok(())
}
