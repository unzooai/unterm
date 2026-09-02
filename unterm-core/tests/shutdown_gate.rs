//! A front end that is quitting must not resurrect the Core it just stopped.
//!
//! The bug this covers, seen on a real Linux desktop: closing the last
//! window logged "stopped the unterm-core this process started" and then,
//! in the same breath, "unterm-core replaced (now pid N)". The window's own
//! frame worker noticed the connection drop, could not tell a deliberate
//! shutdown from a crash, and healed it the only way it knows -- by starting
//! another Core. That Core outlived the process that had just taken its
//! predecessor down. On Windows the orphan then keeps `unterm-core.exe`
//! open, which is why an install could not replace the file.
//!
//! This lives in its own file, and therefore its own test process, on
//! purpose: `begin_shutdown` is a one-way process-global latch, so closing
//! it would change what every later test in the same binary means --
//! including `a_killed_core_is_replaced_without_restarting_the_client`,
//! which asserts the exact opposite behaviour.

use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

#[derive(serde::Deserialize)]
struct Discovery {
    endpoint: String,
    token: String,
    pid: u32,
}

struct CoreGuard(Child);

impl Drop for CoreGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn spawn_core(state_dir: &std::path::Path) -> CoreGuard {
    Command::new(env!("CARGO_BIN_EXE_unterm-core"))
        .env("UNTERM_STATE_DIR", state_dir)
        .env("USERPROFILE", state_dir)
        .env("HOME", state_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(CoreGuard)
        .expect("spawn unterm-core binary")
}

fn read_discovery(state_dir: &std::path::Path) -> Option<Discovery> {
    serde_json::from_slice(&std::fs::read(state_dir.join("core.json")).ok()?).ok()
}

fn wait_for_discovery(state_dir: &std::path::Path, timeout: Duration) -> Discovery {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if let Some(found) = read_discovery(state_dir) {
            return found;
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

fn wait_until(timeout: Duration, mut condition: impl FnMut() -> bool) -> bool {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if condition() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    false
}

/// Is some process serving as a Core in this state dir right now?
///
/// Reading the record is not enough: a Core that exited leaves its record
/// behind, so the record alone cannot tell "still running" from "died and
/// nobody cleaned up". Dialling it can.
fn a_core_is_serving(state_dir: &std::path::Path) -> Option<u32> {
    let discovery = read_discovery(state_dir)?;
    let mut stream = TcpStream::connect(&discovery.endpoint).ok()?;
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .ok()?;
    let health = request(
        &mut stream,
        &discovery.token,
        "core.health",
        serde_json::Value::Null,
    );
    health["result"]["status"].as_str()?;
    Some(discovery.pid)
}

#[test]
fn a_quitting_front_end_does_not_restart_the_core_it_stopped() {
    let state_dir = std::env::temp_dir().join(format!(
        "unterm-core-e2e-quit-gate-{}",
        uuid::Uuid::new_v4()
    ));
    std::fs::create_dir_all(&state_dir).unwrap();
    // The reconnect path resolves discovery through the *test process's*
    // own view of where state lives, so this has to be set in-process.
    std::env::set_var("UNTERM_STATE_DIR", &state_dir);
    std::env::set_var("HOME", &state_dir);
    std::env::set_var("USERPROFILE", &state_dir);

    // `ensure_running` -- the call both resurrection paths make -- looks for
    // the Core binary *next to the running executable*. For a test binary
    // that is `target/debug/deps/`, where no Core lives, so without this the
    // resurrection could never happen and the test would pass for the wrong
    // reason. Put one there.
    let core_binary = std::path::Path::new(env!("CARGO_BIN_EXE_unterm-core"));
    let beside_the_test = std::env::current_exe()
        .expect("resolve test executable")
        .with_file_name(core_binary.file_name().unwrap());
    // Hard link rather than copy: a copy is a new inode, and on macOS the
    // first exec of one stalls in Gatekeeper for minutes -- long past any
    // window this test is willing to wait, which would once again make it
    // pass for the wrong reason. A link is the same file that Cargo already
    // built and ran.
    // Remade every run, never reused: Cargo writes a *new* file when the Core
    // is rebuilt, so a link left from a previous run points at the previous
    // binary. Keeping it would quietly test a stale Core.
    let _ = std::fs::remove_file(&beside_the_test);
    std::fs::hard_link(core_binary, &beside_the_test)
        .or_else(|_| std::fs::copy(core_binary, &beside_the_test).map(|_| ()))
        .expect("stage a core beside the test binary");

    let mut core = spawn_core(&state_dir);
    let discovery = wait_for_discovery(&state_dir, Duration::from_secs(10));
    let original_pid = discovery.pid;

    // A window's-eye view of that Core, with the worker that does the
    // resurrecting actually running.
    let cache = unterm_core::FrameCache::start(discovery.endpoint.clone(), discovery.token.clone())
        .expect("frame cache should attach to the core");
    assert!(
        wait_until(Duration::from_secs(10), || cache.is_live()),
        "the cache never attached to the core it was pointed at"
    );

    // The quit: the gate closes first, exactly as `stop_core_if_ours` does,
    // and only then does the Core go down.
    unterm_core::begin_shutdown();
    assert!(unterm_core::is_shutting_down());

    let mut stream = TcpStream::connect(&discovery.endpoint).unwrap();
    let _ = request(
        &mut stream,
        &discovery.token,
        "core.shutdown",
        serde_json::Value::Null,
    );
    drop(stream);
    assert!(
        wait_until(Duration::from_secs(10), || matches!(
            core.0.try_wait(),
            Ok(Some(_))
        )),
        "the core did not exit after core.shutdown"
    );

    // Long enough for the worker to notice, back off, and try again
    // several times -- the window in which the bug used to strike.
    std::thread::sleep(Duration::from_secs(6));

    match a_core_is_serving(&state_dir) {
        None => {}
        Some(pid) if pid == original_pid => {
            panic!("the core we shut down (pid {pid}) is somehow still serving")
        }
        Some(pid) => panic!(
            "a replacement core (pid {pid}, was {original_pid}) was started while this \
             process was quitting; it would outlive us, and on Windows it would hold \
             unterm-core.exe open against the next install"
        ),
    }

    drop(cache);
    let _ = std::fs::remove_dir_all(&state_dir);
}
