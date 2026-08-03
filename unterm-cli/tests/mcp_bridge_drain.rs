use serde_json::{json, Value};
use std::io::Write;
use std::process::{Command, Stdio};

#[test]
fn mismatched_running_product_drains_instead_of_claiming_the_gui_is_absent() {
    let home = tempfile::tempdir().unwrap();
    let state = home.path().join(".unterm");
    std::fs::create_dir_all(state.join("instances")).unwrap();
    let stale = json!({
        "id": "alpha",
        "mcp_port": 9,
        "http_port": 0,
        "auth_token": "not-used-because-version-check-runs-first",
        "pid": std::process::id(),
        "started_at": "2026-08-03T00:00:00Z",
        "version": "0.57.4",
        "product_version": "0.57.4",
        "build_commit": "legacy",
        "protocol_version": "1.0.0",
        "data_schema_version": 1,
        "process_role": "gui",
    });
    std::fs::write(
        state.join("active.json"),
        serde_json::to_vec(&stale).unwrap(),
    )
    .unwrap();

    let mut child = Command::new(env!("CARGO_BIN_EXE_unterm-cli"))
        .arg("mcp-stdio")
        .env("UNTERM_STATE_DIR", &state)
        .env("USERPROFILE", home.path())
        .env("HOME", home.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    writeln!(
        child.stdin.take().unwrap(),
        "{}",
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}})
    )
    .unwrap();
    let output = child.wait_with_output().unwrap();

    assert!(output.status.success());
    let response: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(response["error"]["code"], -32010);
    let message = response["error"]["message"].as_str().unwrap();
    assert!(
        message.starts_with("product_version_mismatch:"),
        "{message}"
    );
    assert!(!message.contains("GUI is not running"), "{message}");
    let diagnostic = String::from_utf8_lossy(&output.stderr);
    assert!(diagnostic.contains("entering drain"), "{diagnostic}");
}
