//! Canonical SQLite schema for the supervisor store.
//!
//! The schema lives in a dedicated submodule so the parent `store` module
//! stays focused on connection lifecycle and query implementations. Hosts
//! that want to embed the raw DDL (e.g. for migrations) can import
//! [`SCHEMA`] via the public re-export from `crate::store::SCHEMA`.

/// Canonical SQLite schema for the supervisor store.
///
/// Eight tables:
/// - `zyal_super_runs`        — one row per workflow run
/// - `zyal_super_phases`      — phase status + summary per run
/// - `zyal_super_tasks`       — materialized tasks per phase
/// - `zyal_super_memory`      — append-only memory rows
/// - `zyal_super_evidence`    — append-only evidence rows
/// - `zyal_super_repo_symbols`— per-run repo graph symbols
/// - `zyal_super_repo_edges`  — per-run repo graph edges
/// - `zyal_super_signoffs`    — sign-off verdicts per phase
pub const SCHEMA: &str = r#"
    PRAGMA foreign_keys = ON;

    CREATE TABLE IF NOT EXISTS zyal_super_runs (
        run_id        TEXT PRIMARY KEY,
        workflow_id   TEXT NOT NULL,
        name          TEXT NOT NULL,
        objective     TEXT NOT NULL,
        manifest_json TEXT NOT NULL,
        status        TEXT NOT NULL DEFAULT 'running',
        created_at    TEXT NOT NULL,
        updated_at    TEXT NOT NULL
    );

    CREATE TABLE IF NOT EXISTS zyal_super_phases (
        run_id         TEXT NOT NULL,
        phase_id       TEXT NOT NULL,
        name           TEXT NOT NULL,
        objective      TEXT NOT NULL,
        depends_on_json TEXT NOT NULL DEFAULT '[]',
        status         TEXT NOT NULL DEFAULT 'pending',
        summary        TEXT NOT NULL DEFAULT '',
        started_at     TEXT,
        completed_at   TEXT,
        updated_at     TEXT NOT NULL,
        PRIMARY KEY (run_id, phase_id),
        FOREIGN KEY (run_id) REFERENCES zyal_super_runs(run_id) ON DELETE CASCADE
    );

    CREATE TABLE IF NOT EXISTS zyal_super_tasks (
        run_id      TEXT NOT NULL,
        task_id     TEXT NOT NULL,
        phase_id    TEXT NOT NULL,
        title       TEXT NOT NULL,
        status      TEXT NOT NULL DEFAULT 'pending',
        owner       TEXT,
        lease_until INTEGER,
        scope_json  TEXT NOT NULL DEFAULT '{}',
        summary     TEXT NOT NULL DEFAULT '',
        created_at  TEXT NOT NULL,
        updated_at  TEXT NOT NULL,
        PRIMARY KEY (run_id, task_id),
        FOREIGN KEY (run_id, phase_id) REFERENCES zyal_super_phases(run_id, phase_id) ON DELETE CASCADE
    );

    CREATE TABLE IF NOT EXISTS zyal_super_memory (
        id         INTEGER PRIMARY KEY AUTOINCREMENT,
        run_id     TEXT NOT NULL,
        phase_id   TEXT,
        task_id    TEXT,
        scope      TEXT NOT NULL,
        kind       TEXT NOT NULL,
        body       TEXT NOT NULL,
        tags_json  TEXT NOT NULL DEFAULT '[]',
        source_ref TEXT,
        created_at TEXT NOT NULL,
        FOREIGN KEY (run_id) REFERENCES zyal_super_runs(run_id) ON DELETE CASCADE
    );

    CREATE TABLE IF NOT EXISTS zyal_super_evidence (
        id           INTEGER PRIMARY KEY AUTOINCREMENT,
        run_id       TEXT NOT NULL,
        phase_id     TEXT,
        task_id      TEXT,
        kind         TEXT NOT NULL,
        uri          TEXT,
        content_hash TEXT,
        payload_json TEXT NOT NULL DEFAULT '{}',
        created_at   TEXT NOT NULL,
        FOREIGN KEY (run_id) REFERENCES zyal_super_runs(run_id) ON DELETE CASCADE
    );

    CREATE TABLE IF NOT EXISTS zyal_super_repo_symbols (
        run_id        TEXT NOT NULL,
        symbol_id     TEXT NOT NULL,
        kind          TEXT NOT NULL,
        path          TEXT NOT NULL,
        name          TEXT NOT NULL,
        span_json     TEXT NOT NULL DEFAULT '{}',
        metadata_json TEXT NOT NULL DEFAULT '{}',
        updated_at    TEXT NOT NULL,
        PRIMARY KEY (run_id, symbol_id),
        FOREIGN KEY (run_id) REFERENCES zyal_super_runs(run_id) ON DELETE CASCADE
    );

    CREATE TABLE IF NOT EXISTS zyal_super_repo_edges (
        run_id        TEXT NOT NULL,
        edge_id       TEXT NOT NULL,
        src_symbol_id TEXT NOT NULL,
        dst_symbol_id TEXT NOT NULL,
        kind          TEXT NOT NULL,
        weight        REAL NOT NULL DEFAULT 1.0,
        evidence_json TEXT NOT NULL DEFAULT '{}',
        updated_at    TEXT NOT NULL,
        PRIMARY KEY (run_id, edge_id),
        FOREIGN KEY (run_id) REFERENCES zyal_super_runs(run_id) ON DELETE CASCADE
    );

    CREATE TABLE IF NOT EXISTS zyal_super_signoffs (
        id         INTEGER PRIMARY KEY AUTOINCREMENT,
        run_id     TEXT NOT NULL,
        phase_id   TEXT NOT NULL,
        kind       TEXT NOT NULL,
        agent      TEXT NOT NULL,
        verdict    TEXT NOT NULL,
        notes      TEXT NOT NULL DEFAULT '',
        created_at TEXT NOT NULL,
        FOREIGN KEY (run_id, phase_id) REFERENCES zyal_super_phases(run_id, phase_id) ON DELETE CASCADE
    );

    CREATE INDEX IF NOT EXISTS zyal_super_phase_status_idx
        ON zyal_super_phases(run_id, status);
    CREATE INDEX IF NOT EXISTS zyal_super_task_status_idx
        ON zyal_super_tasks(run_id, phase_id, status);
    CREATE INDEX IF NOT EXISTS zyal_super_memory_lookup_idx
        ON zyal_super_memory(run_id, scope, kind);
    CREATE INDEX IF NOT EXISTS zyal_super_evidence_lookup_idx
        ON zyal_super_evidence(run_id, kind, content_hash);
    CREATE INDEX IF NOT EXISTS zyal_super_repo_symbol_path_idx
        ON zyal_super_repo_symbols(run_id, path);
    CREATE INDEX IF NOT EXISTS zyal_super_repo_edge_kind_idx
        ON zyal_super_repo_edges(run_id, kind);
"#;
