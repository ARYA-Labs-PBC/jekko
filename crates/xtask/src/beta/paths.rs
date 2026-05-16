use anyhow::{bail, Context, Result};
use std::env;
use std::path::{Path, PathBuf};

pub(super) fn host_binary_path(root: &Path) -> Result<String> {
    let target_dir = match env::var("CARGO_TARGET_DIR") {
        Ok(value) => value,
        Err(_) => "target".to_string(),
    };
    let exe = if cfg!(windows) { "jekko.exe" } else { "jekko" };
    Ok(root
        .join(target_dir)
        .join("debug")
        .join(exe)
        .display()
        .to_string())
}

pub(super) fn host_path(host_bin: &str) -> Result<String> {
    let mut path = match env::var_os("PATH") {
        Some(value) => value,
        None => std::ffi::OsString::new(),
    };
    let mut new_path = env::split_paths(&path).collect::<Vec<PathBuf>>();
    new_path.insert(0, PathBuf::from(host_bin).parent().unwrap().to_path_buf());
    path = env::join_paths(new_path).context("join PATH")?;
    if path.is_empty() {
        bail!("empty PATH after prepending host binary dir");
    }
    Ok(path.to_string_lossy().into_owned())
}
