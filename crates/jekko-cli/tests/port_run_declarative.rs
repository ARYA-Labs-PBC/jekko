//! ARY-2358 integration test: `jekko port-run --declarative`.
//!
//! Proves the declarative ZYAL phase-body issues a REAL `tools/call` to an
//! attached stdio MCP server and records the server's result as the phase
//! summary — with NO reasoning provider in the loop (ADR-020 §3). This is
//! the honest evidence behind ARY-2358 AC3: "the .zyal runbook issues
//! mcp_calls back to AARA to perform the actual [compute]".
//!
//! The test ships a tiny Python stdio MCP echo server that stands in for
//! AARA's MCP surface. It implements initialize + tools/list + tools/call,
//! returning a per-phase `evidence_id` so the test can assert the result
//! actually round-tripped (not a synthetic stub summary).

use std::io::Write;
use std::path::PathBuf;

use assert_cmd::Command;
use serde_json::Value;

/// Minimal stdio MCP server: initialize + tools/list + tools/call(echo).
const ECHO_SERVER: &str = r#"
import json, sys
def send(o):
    sys.stdout.write(json.dumps(o) + "\n"); sys.stdout.flush()
for line in sys.stdin:
    line = line.strip()
    if not line: continue
    try: req = json.loads(line)
    except Exception: continue
    m = req.get("method"); rid = req.get("id")
    if m == "initialize":
        send({"jsonrpc":"2.0","id":rid,"result":{"protocolVersion":"2024-11-05","capabilities":{"tools":{}},"serverInfo":{"name":"echo","version":"0.1"}}})
    elif m == "notifications/initialized":
        pass
    elif m == "tools/list":
        send({"jsonrpc":"2.0","id":rid,"result":{"tools":[{"name":"echo","description":"echo","inputSchema":{"type":"object"}}]}})
    elif m == "tools/call":
        p = req.get("params",{}); args = p.get("arguments",{})
        send({"jsonrpc":"2.0","id":rid,"result":{"content":[{"type":"text","text":json.dumps({"echoed":args,"evidence_id":"aara-evidence-"+str(args.get("phase","x"))})}],"isError":False}})
    else:
        send({"jsonrpc":"2.0","id":rid,"error":{"code":-32601,"message":"method not found"}})
"#;

fn write_nine_phase_manifest(dir: &std::path::Path, server: &str) -> PathBuf {
    let names = [
        "decompose",
        "plan_roles",
        "adversarial",
        "adv_evidence",
        "purple_team",
        "pt_evidence",
        "user_acceptance",
        "closeout_verify",
        "closeout",
    ];
    let mut phases = Vec::new();
    let mut prev: Option<String> = None;
    for (i, nm) in names.iter().enumerate() {
        let id = format!("p{i:02}-{nm}");
        let depends = match &prev {
            Some(p) => serde_json::json!([p]),
            None => serde_json::json!([]),
        };
        phases.push(serde_json::json!({
            "id": id, "name": nm, "objective": format!("{nm} stage"),
            "depends_on": depends,
            "mcp_call": {"server": server, "tool": "echo", "arguments": {"phase": nm}}
        }));
        prev = Some(id);
    }
    let manifest = serde_json::json!({
        "id": "ary2358-decl-test", "name": "declarative test",
        "objective": "prove declarative tools/call", "phases": phases
    });
    let path = dir.join("wave.json");
    std::fs::write(&path, serde_json::to_string_pretty(&manifest).unwrap()).unwrap();
    path
}

#[test]
fn declarative_phase_body_issues_real_tools_call() {
    let tmp = tempfile::tempdir().expect("tempdir");
    // Isolate JEKKO_HOME so the test's attach does not touch a dev's real config.
    let jekko_home = tmp.path().join("jekko-home");
    std::fs::create_dir_all(&jekko_home).unwrap();

    // Write the echo server script.
    let server_py = tmp.path().join("echo_server.py");
    let mut f = std::fs::File::create(&server_py).unwrap();
    f.write_all(ECHO_SERVER.as_bytes()).unwrap();
    drop(f);

    // 1. Attach the echo server under the isolated home.
    Command::cargo_bin("jekko")
        .unwrap()
        .env("JEKKO_HOME", &jekko_home)
        .args([
            "mcp",
            "attach",
            "aara-echo",
            "python3",
            "--",
            server_py.to_str().unwrap(),
        ])
        .assert()
        .success();

    // 2. Run the declarative manifest.
    let manifest = write_nine_phase_manifest(tmp.path(), "aara-echo");
    let db = tmp.path().join("sup.sqlite");
    let out = Command::cargo_bin("jekko")
        .unwrap()
        .env("JEKKO_HOME", &jekko_home)
        .args([
            "port-run",
            "--super",
            manifest.to_str().unwrap(),
            "--declarative",
            "--db",
            db.to_str().unwrap(),
        ])
        .output()
        .expect("port-run --declarative");
    assert!(
        out.status.success(),
        "declarative port-run failed: stderr=\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("complete (declarative)"),
        "expected declarative completion line, got:\n{stdout}"
    );

    // 3. Read status; assert each phase summary carries the REAL tools/call
    //    result (the evidence_id), not a synthetic scaffold/stub summary.
    let run_id = stdout
        .lines()
        .find_map(|l| l.strip_prefix("run `").and_then(|r| r.split('`').next()))
        .expect("run id in output")
        .to_string();

    let status = Command::cargo_bin("jekko")
        .unwrap()
        .env("JEKKO_HOME", &jekko_home)
        .args([
            "port-run",
            "--status",
            &run_id,
            "--db",
            db.to_str().unwrap(),
        ])
        .output()
        .expect("port-run --status");
    let status_json: Value = serde_json::from_slice(&status.stdout).expect("status json");
    let phases = status_json
        .get("phases")
        .and_then(|v| v.as_array())
        .expect("phases array");
    assert_eq!(phases.len(), 9, "expected 9 phases");
    for p in phases {
        let summary = p.get("summary").and_then(|v| v.as_str()).unwrap_or("");
        assert!(
            summary.contains("aara-evidence-"),
            "phase {:?} summary lacks real tools/call evidence: {summary}",
            p.get("phase_id")
        );
        assert!(
            !summary.contains("scaffold-mode"),
            "phase {:?} fell through to scaffold stub: {summary}",
            p.get("phase_id")
        );
        assert_eq!(p.get("status").and_then(|v| v.as_str()), Some("complete"));
    }
}

#[test]
fn declarative_rejects_phase_without_mcp_call() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let jekko_home = tmp.path().join("jekko-home");
    std::fs::create_dir_all(&jekko_home).unwrap();
    // 9-phase manifest where ONE phase has no mcp_call.
    let names = ["a", "b", "c", "d", "e", "f", "g", "h", "i"];
    let mut phases = Vec::new();
    let mut prev: Option<String> = None;
    for (i, nm) in names.iter().enumerate() {
        let id = format!("p{i:02}-{nm}");
        let depends = match &prev {
            Some(p) => serde_json::json!([p]),
            None => serde_json::json!([]),
        };
        let mut ph = serde_json::json!({
            "id": id, "name": nm, "objective": "x", "depends_on": depends});
        // first phase deliberately MISSING mcp_call
        if i != 0 {
            ph["mcp_call"] = serde_json::json!({"server":"nope","tool":"echo","arguments":{}});
        }
        phases.push(ph);
        prev = Some(id);
    }
    let manifest =
        serde_json::json!({"id":"decl-missing","name":"x","objective":"x","phases":phases});
    let path = tmp.path().join("m.json");
    std::fs::write(&path, serde_json::to_string(&manifest).unwrap()).unwrap();
    let db = tmp.path().join("s.sqlite");
    let out = Command::cargo_bin("jekko")
        .unwrap()
        .env("JEKKO_HOME", &jekko_home)
        .args([
            "port-run",
            "--super",
            path.to_str().unwrap(),
            "--declarative",
            "--db",
            db.to_str().unwrap(),
        ])
        .output()
        .expect("run");
    // The phase without an mcp_call must be marked Failed (hard error in
    // declarative mode), so the run halts — exit is success at the walker
    // level but the failing phase is recorded. Assert the error text.
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        combined.contains("has no mcp_call") || combined.contains("failed"),
        "expected a no-mcp_call failure, got:\n{combined}"
    );
}
