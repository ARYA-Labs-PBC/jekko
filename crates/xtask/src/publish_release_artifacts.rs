use crate::{publish_docker_image, publish_release_package, publish_release_registry};
use anyhow::Result;
use std::path::Path;

pub fn run(repo_root: &Path, version: &str, channel: &str) -> Result<()> {
    publish_release_package::run_all(repo_root, &repo_root.join("dist"), channel)?;
    publish_docker_image::run(version, channel)?;
    publish_release_registry::run(repo_root, version)?;
    Ok(())
}
