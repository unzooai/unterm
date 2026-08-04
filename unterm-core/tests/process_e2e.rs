//! End-to-end tests against the real `unterm-core` binary.
//!
//! Everything in `src/lib.rs`'s test module exercises the server
//! in-process; these spawn the actual executable, so they cover what
//! those cannot: discovery publication, the cross-process instance
//! lock, and shutdown leaving no state behind. `UNTERM_STATE_DIR`
//! points every child at a scratch directory — the user's live Core,
//! if any, must never be touched by a test run.

use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

#[derive(Debug, serde::Deserialize)]
struct Discovery {
    endpoint: String,
    token: String,
    pid: u32,
    product_version: String,
}

fn scratch_state_dir(label: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "unterm-core-e2e-{label}-{}",
        uuid::Uuid::new_v4()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn spawn_core(state_dir: &std::path::Path) -> Child {
    Command::new(env!("CARGO_BIN_EXE_unterm-core"))
        .env("UNTERM_STATE_DIR", state_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn unterm-core binary")
}

fn read_discovery(state_dir: &std::path::Path) -> Option<Discovery> {
    let raw = std::fs::read(state_dir.join("core.json")).ok()?;
    serde_json::from_slice(&raw).ok()
}

fn wait_for_discovery(state_dir: &std::path::Path, timeout: Duration) -> Discovery {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if let Some(discovery) = read_discovery(state_dir) {
            return discovery;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    panic!("unterm-core never published discovery in {state_dir:?}");
}

fn request(
    stream: &mut TcpStream,
    token: &str,
    method: &str,
    params: serde_json::Value,
) -> serde_json::Value {
    let frame = serde_json::json!({
        "id": uuid::Uuid::new_v4().to_string(),
        "method": method,
        "token": token,
        "params": params,
    });
    writeln!(stream, "{frame}").unwrap();
    stream.flush().unwrap();
    let mut line = String::new();
    BufReader::new(stream.try_clone().unwrap())
        .read_line(&mut line)
        .unwrap();
    serde_json::from_str(&line).unwrap()
}

fn wait_for_exit(child: &mut Child, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        match child.try_wait() {
            Ok(Some(_)) => return true,
            Ok(None) => std::thread::sleep(Duration::from_millis(25)),
            Err(_) => return false,
        }
    }
    false
}

#[test]
fn real_process_serves_sessions_and_cleans_up() {
    let state_dir = scratch_state_dir("single");
    let mut child = spawn_core(&state_dir);

    let discovery = wait_for_discovery(&state_dir, Duration::from_secs(10));
    assert_eq!(discovery.pid, child.id());
    assert!(!discovery.product_version.is_empty());

    let mut stream = TcpStream::connect(&discovery.endpoint).unwrap();
    stream.set_nodelay(true).unwrap();

    let info = request(&mut stream, &discovery.token, "core.info", serde_json::Value::Null);
    assert_eq!(info["ok"], true, "core.info failed: {info}");
    assert_eq!(info["result"]["process_role"], "core");

    let argv = if cfg!(windows) { ["cmd.exe"] } else { ["sh"] };
    let created = request(
        &mut stream,
        &discovery.token,
        "session.create",
        serde_json::json!({"cols": 80, "rows": 24, "argv": argv}),
    );
    assert_eq!(created["ok"], true, "session.create failed: {created}");
    let pane_id = created["result"]["id"].as_u64().unwrap();

    let closed = request(
        &mut stream,
        &discovery.token,
        "session.close",
        serde_json::json!({"pane_id": pane_id}),
    );
    assert_eq!(closed["ok"], true);

    let _ = request(
        &mut stream,
        &discovery.token,
        "core.shutdown",
        serde_json::Value::Null,
    );
    assert!(
        wait_for_exit(&mut child, Duration::from_secs(10)),
        "core did not exit after core.shutdown"
    );
    assert!(
        read_discovery(&state_dir).is_none(),
        "shutdown must clear the discovery record"
    );

    let _ = std::fs::remove_dir_all(&state_dir);
}

#[test]
fn concurrent_launches_yield_exactly_one_core() {
    let state_dir = scratch_state_dir("race");
    let mut children: Vec<Child> = (0..8).map(|_| spawn_core(&state_dir)).collect();

    let discovery = wait_for_discovery(&state_dir, Duration::from_secs(10));
    let winner_pid = discovery.pid;
    assert!(
        children.iter().any(|child| child.id() == winner_pid),
        "discovery pid {winner_pid} is not one of the spawned children"
    );

    // Losers exit on their own; give the whole field time to settle,
    // then insist the winner is the only survivor.
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let mut alive: Vec<u32> = Vec::new();
        for child in children.iter_mut() {
            if matches!(child.try_wait(), Ok(None)) {
                alive.push(child.id());
            }
        }
        if alive == vec![winner_pid] {
            break;
        }
        if Instant::now() > deadline {
            panic!("expected only winner {winner_pid} alive, still alive: {alive:?}");
        }
        std::thread::sleep(Duration::from_millis(50));
    }

    // The record must still be the winner's after every loser exited:
    // a loser must never overwrite discovery on its way out.
    let discovery = read_discovery(&state_dir).expect("discovery vanished");
    assert_eq!(discovery.pid, winner_pid);

    let mut stream = TcpStream::connect(&discovery.endpoint).unwrap();
    let health = request(
        &mut stream,
        &discovery.token,
        "core.health",
        serde_json::Value::Null,
    );
    assert_eq!(health["result"]["status"], "ready");

    let _ = request(
        &mut stream,
        &discovery.token,
        "core.shutdown",
        serde_json::Value::Null,
    );
    for child in &mut children {
        if child.id() == winner_pid {
            assert!(
                wait_for_exit(child, Duration::from_secs(10)),
                "winner did not exit after shutdown"
            );
        }
    }

    let _ = std::fs::remove_dir_all(&state_dir);
}
