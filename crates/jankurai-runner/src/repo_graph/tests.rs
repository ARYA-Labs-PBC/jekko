use std::fs;

use tempfile::tempdir;

use super::*;

#[test]
fn builds_file_test_and_import_edges() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nname='x'\nversion='0.1.0'\n",
    )
    .unwrap();
    fs::create_dir_all(dir.path().join("src")).unwrap();
    fs::write(
        dir.path().join("src/ping.rs"),
        "use std::fmt;\nmod codec;\npub struct Ping;\npub enum Reply { Pong }\npub fn ping() { helper(); }\nfn helper() {}\nimpl Ping { pub fn run(&self) { ping(); self.private(); } fn private(&self) {} }\n",
    )
    .unwrap();
    fs::create_dir_all(dir.path().join("tests")).unwrap();
    fs::write(dir.path().join("tests/ping.rs"), "#[test]\nfn ping() {}\n").unwrap();
    fs::create_dir_all(dir.path().join("docs")).unwrap();
    fs::write(dir.path().join("docs/spec.md"), "spec").unwrap();

    let graph = build_repo_graph(dir.path()).unwrap();
    let summary = graph.summary();
    assert_eq!(summary.get("test").copied(), Some(1));
    assert!(summary.get("doc").copied().unwrap_or(0) >= 1);
    assert!(!graph.tests_covering("src/ping.rs").is_empty());
    assert!(graph.edges.iter().any(|edge| edge.kind == "imports"));
    assert!(graph.nodes.iter().any(|node| node.kind == "function"));
    assert!(graph.nodes.iter().any(|node| node.kind == "struct"));
    assert!(graph.nodes.iter().any(|node| node.kind == "enum"));
    assert!(graph.nodes.iter().any(|node| node.kind == "method"));
    assert!(graph.edges.iter().any(|edge| edge.kind == "calls"));
}
