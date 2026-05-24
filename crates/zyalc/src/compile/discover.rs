use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;

pub(super) fn discover(root: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    let candidates = ["agent/zyal", "agent/workflows"];
    for c in candidates {
        let p = root.join(c);
        if p.is_file() && p.extension().and_then(|e| e.to_str()) == Some("zyal") {
            out.push(p);
        } else if p.is_dir() {
            for entry in fs::read_dir(&p)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("zyal") {
                    out.push(path);
                }
            }
        }
    }
    Ok(out)
}
