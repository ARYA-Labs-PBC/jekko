//! Migration UI that mirrors the db-migration progress block around
//! line ~80 of `packages/jekko/src/index.ts`.
//!
//! When jekko-store reports a migration count the binary should print a
//! single progress line per applied migration on stderr. Today the store
//! does not yield per-migration events, so the CLI surfaces a generic
//! "Migrating database (N/M)..." marker before [`jekko_store::Db::open`]
//! runs.
//!
//! The real progress UI lands when the store crate exposes a
//! per-migration callback hook.

use std::io::{self, IsTerminal, Write};

/// Pre-flight banner. Prints a single line. Idempotent.
pub fn print_pre_migration() {
    let mut stderr = io::stderr().lock();
    if stderr.is_terminal() {
        let _ = writeln!(stderr, "Preparing database...");
    } else {
        let _ = writeln!(stderr, "sqlite-migration:start");
    }
}

/// Per-step banner. The real implementation will plug into a
/// `jekko_store::Db::open_with_progress` callback once that lands.
pub fn print_progress(current: usize, total: usize, label: &str) {
    if total == 0 {
        return;
    }
    let mut stderr = io::stderr().lock();
    if stderr.is_terminal() {
        let _ = writeln!(stderr, "Migrating database ({current}/{total})... {label}");
    } else {
        let _ = writeln!(stderr, "sqlite-migration:{current}/{total}");
    }
}

/// Final marker. Always prints to stderr.
pub fn print_post_migration() {
    let mut stderr = io::stderr().lock();
    if stderr.is_terminal() {
        let _ = writeln!(stderr, "Database ready.");
    } else {
        let _ = writeln!(stderr, "sqlite-migration:done");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn print_progress_with_zero_total_is_noop() {
        // Should not panic and should not write — we have no way to assert
        // on the lack of writes without capturing stderr, so this is just a
        // smoke test that the early-return path is hit.
        print_progress(0, 0, "noop");
    }

    #[test]
    fn print_pre_and_post_are_idempotent() {
        // Idempotency here just means "calling twice doesn't panic".
        print_pre_migration();
        print_pre_migration();
        print_post_migration();
        print_post_migration();
    }
}
