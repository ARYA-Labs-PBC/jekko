use std::path::Path;

use anyhow::{anyhow, Result};

pub(super) fn validate_daemon_profile(source: &Path, raw: &str) -> Result<()> {
    if !raw.contains("<<<ZYAL v1:daemon") {
        return Err(anyhow!(
            "daemon profile missing daemon sentinel in {}",
            source.display()
        ));
    }
    if !raw.contains("<<<END_ZYAL") {
        return Err(anyhow!(
            "daemon profile missing END_ZYAL sentinel in {}",
            source.display()
        ));
    }
    Ok(())
}

pub(super) fn validate_runbook_profile(source: &Path, raw: &str) -> Result<()> {
    if !raw.contains("<<<ZYAL v1:") {
        return Err(anyhow!(
            "runbook profile missing ZYAL sentinel in {}",
            source.display()
        ));
    }
    if !raw.contains("<<<END_ZYAL") {
        return Err(anyhow!(
            "runbook profile missing END_ZYAL sentinel in {}",
            source.display()
        ));
    }
    Ok(())
}
