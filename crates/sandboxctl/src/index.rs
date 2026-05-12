//! Per-sandbox index stored at `<sandbox_root>/index.json`. Maps `run_id` to
//! metadata (lane, backend, workspace path, status). Writes use an `O_EXCL`
//! lock file alongside to avoid concurrent corruption.

use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    pub run_id: String,
    pub lane_name: String,
    pub backend: String,
    pub root: PathBuf,
    pub created_at: String,
    pub status: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct IndexDoc {
    #[serde(default)]
    entries: Vec<Entry>,
}

pub fn insert(sandbox_root: &Path, entry: &Entry) -> Result<()> {
    with_lock(sandbox_root, |doc| {
        doc.entries.retain(|e| e.run_id != entry.run_id);
        doc.entries.push(entry.clone());
        Ok(())
    })
}

pub fn remove(sandbox_root: &Path, run_id: &str) -> Result<()> {
    with_lock(sandbox_root, |doc| {
        doc.entries.retain(|e| e.run_id != run_id);
        Ok(())
    })
}

pub fn find(sandbox_root: &Path, run_id: &str) -> Result<Option<Entry>> {
    let doc = read(sandbox_root)?;
    Ok(doc.entries.into_iter().find(|e| e.run_id == run_id))
}

pub fn all(sandbox_root: &Path) -> Result<Vec<Entry>> {
    let doc = read(sandbox_root)?;
    Ok(doc.entries)
}

fn read(sandbox_root: &Path) -> Result<IndexDoc> {
    let path = sandbox_root.join("index.json");
    if !path.exists() {
        return Ok(IndexDoc::default());
    }
    let mut f = File::open(&path).with_context(|| format!("open {}", path.display()))?;
    let mut buf = String::new();
    f.read_to_string(&mut buf)
        .with_context(|| format!("read {}", path.display()))?;
    let doc = serde_json::from_str(&buf)
        .with_context(|| format!("parse {}", path.display()))?;
    Ok(doc)
}

fn with_lock<F>(sandbox_root: &Path, edit: F) -> Result<()>
where
    F: FnOnce(&mut IndexDoc) -> Result<()>,
{
    fs::create_dir_all(sandbox_root)
        .with_context(|| format!("mkdir {}", sandbox_root.display()))?;
    let lock_path = sandbox_root.join("index.lock");
    let _lock = acquire_lock(&lock_path)?;
    let mut doc = read(sandbox_root)?;
    edit(&mut doc)?;
    let tmp = sandbox_root.join("index.json.tmp");
    {
        let mut f = File::create(&tmp).with_context(|| format!("create {}", tmp.display()))?;
        let bytes = serde_json::to_vec_pretty(&doc)?;
        f.write_all(&bytes)?;
        f.flush()?;
    }
    let final_path = sandbox_root.join("index.json");
    fs::rename(&tmp, &final_path)
        .with_context(|| format!("rename {} → {}", tmp.display(), final_path.display()))?;
    let _ = fs::remove_file(&lock_path);
    Ok(())
}

fn acquire_lock(path: &Path) -> Result<LockGuard> {
    let mut attempts = 0;
    loop {
        match OpenOptions::new().write(true).create_new(true).open(path) {
            Ok(_) => return Ok(LockGuard(path.to_path_buf())),
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
                attempts += 1;
                if attempts > 50 {
                    anyhow::bail!("could not acquire index lock {} after 50 tries", path.display());
                }
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            Err(err) => return Err(err).context(format!("create lock {}", path.display())),
        }
    }
}

struct LockGuard(PathBuf);

impl Drop for LockGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}
