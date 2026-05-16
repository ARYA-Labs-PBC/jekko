//! SQLite connection wrapper.
//!
//! Mirrors the runtime database behavior:
//! - Opens the database (file path, `:memory:`, or any [`rusqlite`]-compatible URI).
//! - Applies the established pragma set (WAL, foreign keys, busy timeout, ...).
//! - Applies the embedded Drizzle migration journal so existing databases open
//!   cleanly under Rust.

use std::path::Path;

use rusqlite::Connection;

use crate::error::StoreResult;
use crate::migration::{apply_journal, MigrationEntry};

include!(concat!(env!("OUT_DIR"), "/migrations.gen.rs"));

/// Owned SQLite connection wrapper.
///
/// Construct via [`Db::open`] for an on-disk file (or `:memory:`).
#[derive(Debug)]
pub struct Db {
    conn: Connection,
}

impl Db {
    /// Open the database at `path` and apply pragmas + embedded migrations.
    ///
    /// `path` may be a filesystem path or the special string `":memory:"` (in
    /// which case [`rusqlite::Connection::open_in_memory`] is used).
    pub fn open(path: impl AsRef<Path>) -> StoreResult<Self> {
        let path_ref = path.as_ref();
        let conn = if path_ref == Path::new(":memory:") {
            Connection::open_in_memory()?
        } else {
            Connection::open(path_ref)?
        };
        let mut db = Self { conn };
        db.apply_pragmas()?;
        db.migrate()?;
        Ok(db)
    }

    /// Open an in-memory database (convenience).
    pub fn open_in_memory() -> StoreResult<Self> {
        let conn = Connection::open_in_memory()?;
        let mut db = Self { conn };
        db.apply_pragmas()?;
        db.migrate()?;
        Ok(db)
    }

    /// Borrow the underlying connection.
    pub fn connection(&self) -> &Connection {
        &self.conn
    }

    /// Mutably borrow the underlying connection (needed for transactions).
    pub fn connection_mut(&mut self) -> &mut Connection {
        &mut self.conn
    }

    /// Consume the wrapper and return the raw [`Connection`].
    pub fn into_inner(self) -> Connection {
        self.conn
    }

    /// Apply the established SQLite pragma set. Idempotent.
    pub fn apply_pragmas(&self) -> StoreResult<()> {
        // `journal_mode = WAL` is a no-op on
        // `:memory:` connections; rusqlite will just ignore the result.
        self.conn.execute_batch(
            "PRAGMA journal_mode = WAL;\n\
             PRAGMA synchronous = NORMAL;\n\
             PRAGMA busy_timeout = 5000;\n\
             PRAGMA cache_size = -64000;\n\
             PRAGMA foreign_keys = ON;\n\
             PRAGMA wal_checkpoint(PASSIVE);",
        )?;
        Ok(())
    }

    /// Apply the embedded migration journal. Idempotent.
    pub fn migrate(&mut self) -> StoreResult<usize> {
        let entries = embedded_journal();
        apply_journal(&mut self.conn, &entries)
    }
}

/// Convert the build-script-embedded slice into the runtime [`MigrationEntry`] type.
fn embedded_journal() -> Vec<MigrationEntry> {
    MIGRATIONS
        .iter()
        .map(|m| MigrationEntry {
            name: m.name.to_string(),
            timestamp: m.timestamp,
            sql: m.sql.to_string(),
            hash: m.hash.to_string(),
        })
        .collect()
}

/// Inspect the embedded migration count (used by tests/diagnostics).
pub fn embedded_migration_count() -> usize {
    MIGRATIONS.len()
}

/// Iterator over `(name, hash)` of each embedded migration.
pub fn embedded_migrations() -> impl Iterator<Item = (&'static str, &'static str)> {
    MIGRATIONS.iter().map(|m| (m.name, m.hash))
}
