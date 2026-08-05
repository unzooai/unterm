use serde_json::{json, Value};
use std::io::Write;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn wait_for<T>(what: &str, timeout: Duration, mut probe: impl FnMut() -> Option<T>) -> T {
    let deadline = Instant::now() + timeout;
    loop {
        if let Some(value) = probe() {
            return value;
        }
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// The owner-restart cycle M0-02 promises: a drained bridge exits and
/// leaves no lifecycle record behind, so the owner's respawn comes up
/// as a fresh, active registration instead of inheriting stale state.
#[test]
fn drained_bridge_exits_cleanly_and_owner_restart_registers_fresh() {
    let home = tempfile::tempdir().unwrap();
    let state = home.path().join(".unterm");
    let bridges = state.join("bridges");
    std::fs::create_dir_all(&bridges).unwrap();

    let spawn_bridge = || {
        Command::new(env!("CARGO_BIN_EXE_unterm-cli"))
            .arg("mcp-stdio")
            .env("UNTERM_STATE_DIR", &state)
            .env("USERPROFILE", home.path())
            .env("HOME", home.path())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap()
    };

    let mut first = spawn_bridge();
    let first_pid = first.id();
    let first_record = bridges.join(format!("{}.json", first_pid));
    wait_for("first bridge registration", Duration::from_secs(10), || {
        first_record.exists().then_some(())
    });

    // What the GUI does to an incompatible bridge: flip its record to
    // draining. The bridge notices on its next request.
    let mut record: Value =
        serde_json::from_slice(&std::fs::read(&first_record).unwrap()).unwrap();
    record["state"] = json!("draining");
    record["drain_reason"] = json!("product_version_mismatch: test-requested drain");
    std::fs::write(&first_record, serde_json::to_vec(&record).unwrap()).unwrap();

    writeln!(
        first.stdin.take().unwrap(),
        "{}",
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}})
    )
    .unwrap();
    let output = first.wait_with_output().unwrap();
    assert!(output.status.success());
    let response: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(response["error"]["code"], -32010);

    // Exit must have unregistered the record — a leftover would make
    // the enforcer think the old bridge is still squatting.
    wait_for("first record removal", Duration::from_secs(10), || {
        (!first_record.exists()).then_some(())
    });

    // The owner restarts the configured binary; the replacement must
    // register fresh and active under its own pid.
    let mut second = spawn_bridge();
    let second_record = bridges.join(format!("{}.json", second.id()));
    wait_for("second bridge registration", Duration::from_secs(10), || {
        second_record.exists().then_some(())
    });
    let record: Value =
        serde_json::from_slice(&std::fs::read(&second_record).unwrap()).unwrap();
    assert_eq!(record["state"], "active");
    assert_ne!(first_pid, second.id());

    drop(second.stdin.take());
    let _ = second.wait();
}

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
