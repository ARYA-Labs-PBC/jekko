//! End-to-end smoke test: open an in-memory DB, apply the embedded journal,
//! assert idempotence on re-open.

use jekko_store::db::{embedded_migration_count, Db};
use jekko_store::migration::{applied_names, MIGRATIONS_TABLE};
use tempfile::tempdir;

#[test]
fn opens_in_memory_and_applies_migrations() {
    let db = Db::open_in_memory().expect("open in-memory db");

    let conn = db.connection();
    let count = embedded_migration_count();
    assert!(count > 0, "no migrations embedded");
    let applied = applied_names(conn).expect("read journal");
    assert_eq!(
        applied.len(),
        count,
        "expected all {count} migrations applied, got {}",
        applied.len()
    );

    // Sanity: foreign_keys is on.
    let fk: i64 = conn
        .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
        .expect("read foreign_keys pragma");
    assert_eq!(fk, 1);

    // Core tables created by the first migration.
    for table in [
        "project",
        "session",
        "message",
        "part",
        "permission",
        "session_share",
        "workspace",
        "account",
        "event",
    ] {
        let exists: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name = ?1",
                rusqlite::params![table],
                |row| row.get(0),
            )
            .unwrap_or_else(|err| panic!("query {table}: {err}"));
        assert_eq!(exists, 1, "table {table} missing after migrations");
    }

    // Journal table is also present.
    let exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name = ?1",
            rusqlite::params![MIGRATIONS_TABLE],
            |row| row.get(0),
        )
        .expect("query migrations table");
    assert_eq!(exists, 1);
}

#[test]
fn migration_is_idempotent_on_reopen() {
    let tmp = tempdir().expect("tempdir");
    let path = tmp.path().join("jekko.db");

    {
        let _db = Db::open(&path).expect("first open");
    }

    let db = Db::open(&path).expect("second open");
    let applied = applied_names(db.connection()).expect("journal");
    assert_eq!(
        applied.len(),
        embedded_migration_count(),
        "expected idempotent re-open"
    );

    // Third open — still stable.
    drop(db);
    let db = Db::open(&path).expect("third open");
    let applied = applied_names(db.connection()).expect("journal");
    assert_eq!(applied.len(), embedded_migration_count());
}
