//! Round-trip a session row through the SQLite store.

use jekko_store::db::Db;
use jekko_store::project::{self, ProjectRow};
use jekko_store::session::{self, SessionRow};

fn fresh_db() -> Db {
    Db::open_in_memory().expect("open in-memory db")
}

fn seed_project(db: &Db, id: &str) {
    let row = ProjectRow {
        id: id.to_string(),
        worktree: "/tmp/proj".to_string(),
        vcs: Some("git".to_string()),
        name: Some("test".to_string()),
        icon_url: None,
        icon_url_override: None,
        icon_color: None,
        time_created: 1,
        time_updated: 1,
        time_initialized: None,
        sandboxes: vec!["default".to_string()],
        commands: None,
    };
    project::upsert(db.connection(), &row).expect("seed project");
}

fn sample_session(id: &str, project_id: &str) -> SessionRow {
    SessionRow {
        id: id.to_string(),
        project_id: project_id.to_string(),
        workspace_id: None,
        parent_id: None,
        slug: "slug".to_string(),
        directory: "/tmp/wt".to_string(),
        path: Some("/tmp/wt".to_string()),
        title: "hello".to_string(),
        version: "1.0.0".to_string(),
        share_url: None,
        summary_additions: Some(2),
        summary_deletions: Some(0),
        summary_files: Some(1),
        summary_diffs: Some(serde_json::json!([{"path":"a.md","additions":2}])),
        revert: Some(serde_json::json!({"messageID":"m1"})),
        permission: Some(serde_json::json!({"edit":{"*":"allow"}})),
        agent: Some("default".to_string()),
        model: Some(serde_json::json!({"id":"claude","providerID":"anthropic"})),
        time_created: 100,
        time_updated: 100,
        time_compacting: None,
        time_archived: None,
    }
}

#[test]
fn session_create_read() {
    let db = fresh_db();
    seed_project(&db, "p1");
    let row = sample_session("s1", "p1");
    session::upsert(db.connection(), &row).expect("insert");
    let got = session::get(db.connection(), "s1")
        .expect("query")
        .expect("present");
    assert_eq!(got, row);
}

#[test]
fn session_update_via_upsert() {
    let db = fresh_db();
    seed_project(&db, "p1");
    let mut row = sample_session("s2", "p1");
    session::upsert(db.connection(), &row).expect("insert");

    row.title = "renamed".to_string();
    row.time_updated = 200;
    row.summary_additions = Some(99);
    session::upsert(db.connection(), &row).expect("update");

    let got = session::get(db.connection(), "s2")
        .expect("query")
        .expect("present");
    assert_eq!(got.title, "renamed");
    assert_eq!(got.summary_additions, Some(99));
    assert_eq!(got.time_updated, 200);
}

#[test]
fn session_delete() {
    let db = fresh_db();
    seed_project(&db, "p1");
    let row = sample_session("s3", "p1");
    session::upsert(db.connection(), &row).expect("insert");
    let removed = session::delete(db.connection(), "s3").expect("delete");
    assert_eq!(removed, 1);
    let got = session::get(db.connection(), "s3").expect("query");
    assert!(got.is_none());
}

#[test]
fn session_list_for_project() {
    let db = fresh_db();
    seed_project(&db, "p1");
    seed_project(&db, "p2");
    session::upsert(db.connection(), &sample_session("s-a", "p1")).expect("insert a");
    let mut other = sample_session("s-b", "p1");
    other.time_created = 200;
    session::upsert(db.connection(), &other).expect("insert b");
    session::upsert(db.connection(), &sample_session("s-c", "p2")).expect("insert c");

    let rows = session::list_for_project(db.connection(), "p1").expect("list");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].id, "s-a");
    assert_eq!(rows[1].id, "s-b");
}
