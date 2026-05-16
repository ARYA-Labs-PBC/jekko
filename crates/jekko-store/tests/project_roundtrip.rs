//! Round-trip a project row through the SQLite store.

use jekko_store::db::Db;
use jekko_store::project::{self, ProjectCommands, ProjectRow};

fn sample(id: &str) -> ProjectRow {
    ProjectRow {
        id: id.to_string(),
        worktree: "/tmp/x".to_string(),
        vcs: Some("git".to_string()),
        name: Some("Cool Project".to_string()),
        icon_url: Some("https://x".to_string()),
        icon_url_override: None,
        icon_color: Some("#abcdef".to_string()),
        time_created: 10,
        time_updated: 20,
        time_initialized: Some(15),
        sandboxes: vec!["macos".to_string(), "linux".to_string()],
        commands: Some(ProjectCommands {
            start: Some("npm start".to_string()),
        }),
    }
}

#[test]
fn project_create_read() {
    let db = Db::open_in_memory().unwrap();
    let row = sample("proj-1");
    project::upsert(db.connection(), &row).expect("upsert");
    let got = project::get(db.connection(), "proj-1")
        .expect("query")
        .expect("row present");
    assert_eq!(got, row);
}

#[test]
fn project_update_via_upsert() {
    let db = Db::open_in_memory().unwrap();
    let mut row = sample("proj-2");
    project::upsert(db.connection(), &row).expect("upsert");

    row.name = Some("renamed".to_string());
    row.sandboxes = vec!["macos".to_string()];
    row.time_updated = 999;
    project::upsert(db.connection(), &row).expect("update");

    let got = project::get(db.connection(), "proj-2")
        .expect("query")
        .expect("row");
    assert_eq!(got.name, Some("renamed".to_string()));
    assert_eq!(got.sandboxes, vec!["macos".to_string()]);
    assert_eq!(got.time_updated, 999);
}

#[test]
fn project_delete() {
    let db = Db::open_in_memory().unwrap();
    project::upsert(db.connection(), &sample("p")).expect("upsert");
    let n = project::delete(db.connection(), "p").expect("delete");
    assert_eq!(n, 1);
    assert!(project::get(db.connection(), "p").expect("query").is_none());
}

#[test]
fn project_list_orders_by_time_created() {
    let db = Db::open_in_memory().unwrap();
    let mut a = sample("a");
    a.time_created = 1;
    let mut b = sample("b");
    b.time_created = 2;
    project::upsert(db.connection(), &b).unwrap();
    project::upsert(db.connection(), &a).unwrap();
    let rows = project::list(db.connection()).unwrap();
    assert_eq!(
        rows.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(),
        vec!["a", "b"]
    );
}
