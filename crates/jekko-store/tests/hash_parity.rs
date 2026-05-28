//! Migration-hash parity test.
//!
//! The legacy JS `migrationHash` (see `packages/jekko/src/storage/migration-repair.ts`
//! lines 127-129 / 427-429) is `crypto.createHash("sha256").update(text).digest("hex")`
//! — a plain SHA-256 of the raw file bytes, lower-case hex. The reference
//! digests in this test are obtained via `shasum -a 256 <migration.sql>` on
//! the checked-in files, which uses the same algorithm. Our `migration_hash`
//! must produce byte-identical output, otherwise a JS-written
//! `__drizzle_migrations` row will be rejected when read under Rust (and vice
//! versa). The values below are documented inline so future regenerations are
//! traceable to a concrete file path.

use std::fs;

use jekko_store::migration::migration_hash;

const MIGRATIONS_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../db/migrations");

// `shasum -a 256 db/migrations/<dir>/migration.sql` outputs the same algorithm
// as `crypto.createHash("sha256")` in Node, so these values double as the
// authoritative legacy JS reference.

#[test]
fn hash_matches_known_first_migration() {
    let path = format!(
        "{}/20260127222353_familiar_lady_ursula/migration.sql",
        MIGRATIONS_DIR
    );
    let sql = fs::read_to_string(&path).expect("read migration.sql");
    let hash = migration_hash(&sql);
    assert_eq!(
        hash, "f98a4237676331520cfedf1386987aa578c413a56ddc366a958a6b1c883d6cf9",
        "hash drift from legacy JS migrationHash() for {}",
        path
    );
}

#[test]
fn hash_matches_second_known_migration() {
    let path = format!(
        "{}/20260213144116_wakeful_the_professor/migration.sql",
        MIGRATIONS_DIR
    );
    let sql = fs::read_to_string(&path).expect("read migration.sql");
    let hash = migration_hash(&sql);
    assert_eq!(
        hash, "8cfce0061cf6ed2815bf281f1f94c9f83f58ca5541f527405c23e85f4c68eea4",
        "hash drift from legacy JS migrationHash() for {}",
        path
    );
}

#[test]
fn embedded_hashes_match_recompute() {
    // Every embedded migration's stored hash must match a fresh compute over
    // its own SQL. This guards against build-script drift.
    for (name, hash) in jekko_store::db::embedded_migrations() {
        let path = format!("{}/{name}/migration.sql", MIGRATIONS_DIR);
        let sql = fs::read_to_string(&path).unwrap_or_else(|err| panic!("read {path}: {err}"));
        let recomputed = migration_hash(&sql);
        assert_eq!(hash, recomputed, "hash drift for {name}");
    }
}
