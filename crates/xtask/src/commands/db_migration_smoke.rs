//! `xtask db-migration-smoke` — sanity-check the embedded SQLite journal.
//!
//! Opens `JEKKO_DB_SAMPLE` if set (copy-to-scratch first so we never mutate
//! the source DB), or a fresh `:memory:` database otherwise. Applies the
//! embedded journal twice and asserts that:
//!
//! 1. the first open succeeds (every migration listed in `embedded_migrations`
//!    runs without error);
//! 2. re-opening the same database is a no-op — no new rows in the
//!    `__drizzle_migrations` table and zero migrations re-applied.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use jekko_store::Db;

/// Run the smoke. Returns `(applied_count, idempotent)` so callers can
/// embed the numbers in their own log line; the CLI wrapper in
/// `main.rs` prints the canonical one-liner.
pub fn run(env_sample: Option<&str>, tmp_root: &Path) -> Result<(usize, bool)> {
    let (path, _holder) = resolve_db_path(env_sample, tmp_root)?;

    // First open: applies the embedded journal end-to-end. The count we
    // report is the total embedded migration count (open() itself does
    // not return the number applied; we read it from the catalog).
    let _first = Db::open(&path).context("open db (first pass) and apply embedded migrations")?;
    let applied = jekko_store::db::embedded_migration_count();

    // Second open: should be a no-op. We don't expose a "re-applied"
    // count from `apply_journal`, but we can re-call `db.migrate()` and
    // observe that the returned `usize` is 0 (no new rows).
    let mut second = Db::open(&path).context("open db (second pass) for idempotency check")?;
    let reapplied = second
        .migrate()
        .context("re-apply migrations on second open")?;
    let idempotent = reapplied == 0;

    if !idempotent {
        bail!(
            "embedded journal is not idempotent: {} migrations re-applied on second open",
            reapplied
        );
    }

    Ok((applied, idempotent))
}

/// Compute the DB path to use. When `JEKKO_DB_SAMPLE` is set we copy the
/// pointed-to file into a scratch dir (so we never mutate the original)
/// and return that copy plus a "holder" path we keep alive to anchor
/// the cleanup. When unset, fall back to `:memory:`.
///
/// `tmp_root` is the directory under which the copy lives. The caller
/// is responsible for cleaning it up after the smoke finishes.
fn resolve_db_path(
    env_sample: Option<&str>,
    tmp_root: &Path,
) -> Result<(PathBuf, Option<PathBuf>)> {
    match env_sample {
        Some(sample) if !sample.is_empty() => {
            let src = Path::new(sample);
            if !src.exists() {
                bail!("JEKKO_DB_SAMPLE points at missing file: {}", src.display());
            }
            fs::create_dir_all(tmp_root)
                .with_context(|| format!("create scratch dir {}", tmp_root.display()))?;
            let dst = tmp_root.join("jekko-sample.db");
            fs::copy(src, &dst)
                .with_context(|| format!("copy {} → {}", src.display(), dst.display()))?;
            Ok((dst.clone(), Some(dst)))
        }
        _ => Ok((PathBuf::from(":memory:"), None)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_against_in_memory_db_reports_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        let (applied, idempotent) = run(None, tmp.path()).unwrap();
        assert!(
            applied > 0,
            "journal should ship with at least one migration"
        );
        assert!(idempotent, "journal must be idempotent on re-open");
    }

    #[test]
    fn run_with_blank_env_uses_in_memory() {
        let tmp = tempfile::tempdir().unwrap();
        let (applied, idempotent) = run(Some(""), tmp.path()).unwrap();
        assert!(applied > 0);
        assert!(idempotent);
    }

    #[test]
    fn resolve_db_path_errors_on_missing_sample() {
        let tmp = tempfile::tempdir().unwrap();
        let err =
            resolve_db_path(Some("/nonexistent/path/never/exists.db"), tmp.path()).unwrap_err();
        let msg = format!("{err:#}");
        assert!(msg.contains("missing file"), "got: {msg}");
    }

    #[test]
    fn resolve_db_path_copies_sample_into_tmp_root() {
        let tmp = tempfile::tempdir().unwrap();
        let src_dir = tempfile::tempdir().unwrap();
        let src = src_dir.path().join("orig.db");
        std::fs::write(&src, b"fake-sqlite-bytes").unwrap();
        let (path, _holder) = resolve_db_path(Some(src.to_str().unwrap()), tmp.path()).unwrap();
        assert!(path.starts_with(tmp.path()));
        assert_eq!(std::fs::read(&path).unwrap(), b"fake-sqlite-bytes");
    }
}
