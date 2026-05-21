//! Workspace file index for the inline composer `@` mention popup.
//!
//! Walks a root directory respecting `.gitignore`, global git ignores, and
//! hidden-file filters. Stores workspace-relative paths so the popup can
//! display them concisely. Search is a tiered scorer: exact-prefix on the
//! basename beats substring, which beats subsequence on the full path.

use std::path::{Path, PathBuf};

use ignore::WalkBuilder;

const BINARY_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "bmp", "ico", "webp", "tif", "tiff", "svg", "so", "dylib", "a",
    "o", "rlib", "bin", "exe", "dll", "class", "jar", "wasm", "pdf", "zip", "tar", "gz", "tgz",
    "bz2", "xz", "7z", "rar", "mp3", "mp4", "mov", "avi", "mkv", "wav", "flac", "ogg", "ttf",
    "otf", "woff", "woff2", "eot",
];

#[derive(Debug, Clone)]
pub struct FileIndex {
    paths: Vec<PathBuf>,
    root: PathBuf,
}

impl FileIndex {
    pub fn build(root: impl AsRef<Path>, max_entries: usize) -> Self {
        let root = root.as_ref().to_path_buf();
        let mut paths: Vec<PathBuf> = Vec::new();

        let walker = WalkBuilder::new(&root)
            .standard_filters(true)
            .hidden(true)
            .build();

        for entry in walker.flatten() {
            if paths.len() >= max_entries {
                break;
            }
            let path = entry.path();
            if path == root {
                continue;
            }
            let is_dir = entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
            if is_dir {
                continue;
            }
            if is_binary_ext(path) {
                continue;
            }
            let rel = match path.strip_prefix(&root) {
                Ok(r) => r.to_path_buf(),
                Err(_) => continue,
            };
            paths.push(rel);
        }

        Self { paths, root }
    }

    pub fn search(&self, query: &str, limit: usize) -> Vec<&Path> {
        if limit == 0 {
            return Vec::new();
        }
        if query.is_empty() {
            return self.paths.iter().take(limit).map(|p| p.as_path()).collect();
        }

        let q = query.to_lowercase();
        let mut scored: Vec<(u8, usize, &Path)> = Vec::new();
        for p in &self.paths {
            let path_str = p.to_string_lossy().to_lowercase();
            let Some(basename) = p.file_name().map(|s| s.to_string_lossy().to_lowercase()) else {
                continue;
            };

            let tier = if basename.starts_with(&q) {
                0u8
            } else if basename.contains(&q) {
                1u8
            } else if is_subsequence(&q, &path_str) {
                2u8
            } else {
                continue;
            };
            scored.push((tier, path_str.len(), p.as_path()));
        }

        scored.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
        scored.into_iter().take(limit).map(|(_, _, p)| p).collect()
    }

    pub fn len(&self) -> usize {
        self.paths.len()
    }

    pub fn is_empty(&self) -> bool {
        self.paths.is_empty()
    }

    pub fn root(&self) -> &Path {
        &self.root
    }
}

fn is_binary_ext(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| {
            let lower = e.to_ascii_lowercase();
            BINARY_EXTENSIONS.iter().any(|b| *b == lower)
        })
        .unwrap_or(false)
}

fn is_subsequence(needle: &str, haystack: &str) -> bool {
    let mut it = haystack.chars();
    for n in needle.chars() {
        match it.find(|h| *h == n) {
            Some(_) => continue,
            None => return false,
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write(root: &Path, rel: &str, body: &str) {
        let full = root.join(rel);
        if let Some(parent) = full.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(full, body).unwrap();
    }

    #[test]
    fn build_walks_root_and_caps_at_max_entries() {
        let dir = TempDir::new().unwrap();
        for i in 0..20 {
            write(dir.path(), &format!("file_{i:02}.rs"), "x");
        }
        let idx = FileIndex::build(dir.path(), 5);
        assert_eq!(idx.len(), 5, "should cap at max_entries");
    }

    #[test]
    fn search_prefix_beats_substring() {
        let dir = TempDir::new().unwrap();
        write(dir.path(), "src/runner.rs", "x");
        write(dir.path(), "alpha/run_helper.rs", "x");
        write(dir.path(), "beta/has_run_inside.rs", "x");

        let idx = FileIndex::build(dir.path(), 1000);
        let hits = idx.search("run", 10);
        assert!(!hits.is_empty());
        let first = hits[0].to_string_lossy();
        assert!(
            first.ends_with("runner.rs") || first.ends_with("run_helper.rs"),
            "prefix match should win, got {first}"
        );
        // The runner.rs basename starts with "run", so it should outrank
        // anything that only contains it mid-string.
        assert_eq!(hits[0].file_name().unwrap(), "runner.rs");
    }

    #[test]
    fn search_empty_query_returns_some() {
        let dir = TempDir::new().unwrap();
        write(dir.path(), "a.rs", "x");
        write(dir.path(), "b.rs", "x");
        let idx = FileIndex::build(dir.path(), 1000);
        let hits = idx.search("", 10);
        assert_eq!(hits.len(), 2);
    }

    #[test]
    fn search_skips_ignored_files() {
        let dir = TempDir::new().unwrap();
        // .gitignore needs a git root for the ignore crate to honor it
        // reliably; touching `.git` triggers the same code path.
        fs::create_dir(dir.path().join(".git")).unwrap();
        write(dir.path(), ".gitignore", "secret.txt\n");
        write(dir.path(), "secret.txt", "shh");
        write(dir.path(), "public.rs", "x");

        let idx = FileIndex::build(dir.path(), 1000);
        let names: Vec<String> = idx
            .search("", 100)
            .into_iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect();
        assert!(
            names.iter().any(|n| n == "public.rs"),
            "public.rs should be indexed: {names:?}"
        );
        assert!(
            names.iter().all(|n| n != "secret.txt"),
            "secret.txt must be ignored: {names:?}"
        );
    }

    #[test]
    fn search_is_case_insensitive() {
        let dir = TempDir::new().unwrap();
        write(dir.path(), "Cargo.toml", "x");
        write(dir.path(), "src/MAIN.rs", "x");
        let idx = FileIndex::build(dir.path(), 1000);
        let hits_lower = idx.search("cargo", 5);
        let hits_upper = idx.search("CARGO", 5);
        assert!(!hits_lower.is_empty());
        assert_eq!(
            hits_lower
                .iter()
                .map(|p| p.to_string_lossy().into_owned())
                .collect::<Vec<_>>(),
            hits_upper
                .iter()
                .map(|p| p.to_string_lossy().into_owned())
                .collect::<Vec<_>>(),
        );
        let main_hits = idx.search("main", 5);
        assert!(
            main_hits
                .iter()
                .any(|p| p.file_name().unwrap() == "MAIN.rs"),
            "case-insensitive match should find MAIN.rs"
        );
    }

    #[test]
    fn search_skips_binary_extensions() {
        let dir = TempDir::new().unwrap();
        write(dir.path(), "logo.png", "x");
        write(dir.path(), "lib.so", "x");
        write(dir.path(), "code.rs", "x");
        let idx = FileIndex::build(dir.path(), 1000);
        let names: Vec<String> = idx
            .search("", 100)
            .into_iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect();
        assert!(names.iter().any(|n| n == "code.rs"));
        assert!(names.iter().all(|n| !n.ends_with(".png")));
        assert!(names.iter().all(|n| !n.ends_with(".so")));
    }
}
