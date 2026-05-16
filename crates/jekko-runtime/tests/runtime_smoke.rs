//! End-to-end smoke tests for the runtime crate.
//!
//! Exercises the public API surface the rest of the workspace will use:
//!
//! - Session create/append/list against the in-memory store.
//! - Tool catalog serialization.
//! - Permission ask + reply round-trip across the bus.
//! - PTY echo + file/glob/grep on a temp dir.

use std::sync::Arc;

use jekko_core::session::SessionId;
use jekko_runtime::{
    bus::Bus,
    file,
    permission::{new_request_id, PermissionDecision, PermissionRequest, PermissionService},
    pty::{PtySession, PtySpec},
    ripgrep,
    session::{AppendMessageInput, CreateSessionInput, SessionService},
    snapshot,
    status::{Status, StatusService},
    tool::{default_registry, BashTool, ReadTool, Tool, ToolContext},
};

#[tokio::test]
async fn session_create_append_list() {
    let svc = SessionService::new();
    let info = svc
        .create(CreateSessionInput {
            project_id: "proj_abc".into(),
            workspace_id: None,
            parent_id: None,
            directory: std::env::temp_dir().to_string_lossy().into_owned(),
            title: Some("integration".into()),
        })
        .await
        .unwrap();
    assert!(info.id.as_str().starts_with("session_"));

    for i in 0..3 {
        svc.append(AppendMessageInput {
            session_id: info.id.clone(),
            role: "user".into(),
            data: serde_json::json!({ "i": i }),
        })
        .await
        .unwrap();
    }

    let messages = svc.messages(&info.id).await.unwrap();
    assert_eq!(messages.len(), 3);

    let listed = svc.list("proj_abc").await.unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, info.id);
}

#[tokio::test]
async fn status_transitions_publish_events() {
    let bus = Arc::new(Bus::new());
    let svc = StatusService::new(bus.clone());
    let mut sub = bus.subscribe("session.status").await;
    svc.set("session_1", Status::Busy).await;
    let env = sub.recv().await.unwrap();
    assert_eq!(env.properties["status"]["type"], "busy");
    svc.set("session_1", Status::Idle).await;
    let env = sub.recv().await.unwrap();
    assert_eq!(env.properties["status"]["type"], "idle");
}

#[tokio::test]
async fn tool_schemas_are_serializable() {
    let registry = default_registry();
    let catalog = registry.catalog();
    let json = serde_json::to_string(&catalog).unwrap();
    assert!(json.contains("\"id\":\"bash\""));
    assert!(json.contains("\"id\":\"read\""));
    assert!(json.contains("\"id\":\"glob\""));
}

#[tokio::test]
async fn permission_ask_round_trips_over_bus() {
    let bus = Arc::new(Bus::new());
    let svc = Arc::new(PermissionService::new(bus.clone()));
    let mut sub = bus.subscribe("permission.asked").await;

    let req = PermissionRequest {
        id: new_request_id(),
        session_id: "session_x".into(),
        permission: "bash".into(),
        patterns: vec!["ls".into()],
        metadata: serde_json::json!({}),
        always: vec!["ls".into()],
    };
    let req_id = req.id.clone();
    let svc_clone = svc.clone();
    let h = tokio::spawn(async move { svc_clone.ask(req, vec![]).await });

    let env = sub.recv().await.unwrap();
    assert_eq!(env.kind, "permission.asked");
    svc.reply(&req_id, jekko_runtime::permission::PermissionReply::Once)
        .await
        .unwrap();
    let reply = h.await.unwrap().unwrap();
    assert_eq!(reply, jekko_runtime::permission::PermissionReply::Once);
}

#[tokio::test]
async fn bash_tool_executes_with_permission_gate() {
    let bus = Arc::new(Bus::new());
    let svc = Arc::new(PermissionService::new(bus));
    // Pre-approve everything so the gate auto-allows.
    svc.set_approved(vec![jekko_runtime::permission::PermissionRule {
        permission: "bash".into(),
        pattern: "*".into(),
        action: PermissionDecision::Allow,
    }])
    .await;

    let mut ctx = ToolContext::bare(".");
    ctx.permissions = Some(svc.clone());
    let out = BashTool
        .execute(serde_json::json!({ "command": "printf hi" }), ctx)
        .await
        .unwrap();
    assert_eq!(out.output, "hi");
}

#[tokio::test]
async fn read_tool_reads_temp_file() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("x.txt");
    std::fs::write(&p, "alpha\nbeta\n").unwrap();
    let out = ReadTool
        .execute(
            serde_json::json!({ "filePath": p.to_string_lossy() }),
            ToolContext::bare(dir.path()),
        )
        .await
        .unwrap();
    assert!(out.output.contains("alpha"));
    assert!(out.output.contains("beta"));
}

#[tokio::test]
async fn file_glob_and_grep_against_temp_dir() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("a")).unwrap();
    std::fs::write(dir.path().join("a/x.rs"), b"// hello\n").unwrap();
    std::fs::write(dir.path().join("b.rs"), b"// hello\n").unwrap();

    let hits = file::glob(dir.path(), "**/*.rs").unwrap();
    assert_eq!(hits.len(), 2);

    let matches = ripgrep::grep(dir.path(), "hello").await.unwrap();
    assert!(!matches.is_empty());
}

#[tokio::test]
async fn snapshot_changes_with_edits() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.txt"), b"first").unwrap();
    let s1 = snapshot::hash_tree(dir.path()).await.unwrap();
    std::fs::write(dir.path().join("a.txt"), b"second").unwrap();
    let s2 = snapshot::hash_tree(dir.path()).await.unwrap();
    assert_ne!(s1.hash, s2.hash);
}

#[test]
fn session_id_through_runtime() {
    // sanity that we link against jekko-core
    let id: SessionId = "session_smoke".parse().unwrap();
    assert_eq!(id.as_str(), "session_smoke");
}

#[test]
fn pty_echo_round_trip() {
    let session = PtySession::spawn(&PtySpec {
        command: "cat".to_string(),
        args: vec![],
        cols: 80,
        rows: 24,
    })
    .unwrap();
    session.write(b"hello world\n").unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    let mut buf = Vec::new();
    while std::time::Instant::now() < deadline {
        if let Ok(chunk) = session.read(64) {
            buf.extend_from_slice(&chunk);
            if std::str::from_utf8(&buf)
                .unwrap_or("")
                .contains("hello world")
            {
                break;
            }
            if chunk.is_empty() {
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
        }
    }
    assert!(
        std::str::from_utf8(&buf)
            .unwrap_or("")
            .contains("hello world"),
        "buf={:?}",
        String::from_utf8_lossy(&buf)
    );
    let _ = session.kill();
}
