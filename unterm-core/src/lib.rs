//! Independent per-user Core process for the Unterm control plane.
//!
//! Hosts next-core terminal sessions behind an authenticated local IPC
//! boundary so they can outlive any GUI process. Identity and version
//! compatibility come from `unterm-protocol`; the terminal engine is
//! `unterm-engine`'s next-core. This is the M1 service entry point that
//! issue #12 requires: no GUI needed to create a PTY or query a screen.

use anyhow::{Context, Result};
use portable_pty::CommandBuilder;
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream, ToSocketAddrs};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use unterm_engine::{
    CreateSessionRequest, InputEngine, ScreenEngine, SessionEngine,
};
use unterm_protocol::{BuildHandshake, ProcessRole};

/// Discovery record other processes read to find and authenticate to
/// the running Core.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryInfo {
    pub endpoint: String,
    pub token: String,
    pub pid: u32,
    pub product_version: String,
}

pub fn discovery_path() -> Option<std::path::PathBuf> {
    dirs_next::data_local_dir().map(|dir| dir.join("Unterm").join("core.json"))
}

pub fn read_discovery() -> Result<Option<DiscoveryInfo>> {
    let Some(path) = discovery_path() else {
        return Ok(None);
    };
    if !path.exists() {
        return Ok(None);
    }
    Ok(Some(serde_json::from_slice(
        &std::fs::read(path).context("read core discovery")?,
    )?))
}

pub fn write_discovery(endpoint: &str, token: &str) -> Result<()> {
    let Some(path) = discovery_path() else {
        return Ok(());
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let info = DiscoveryInfo {
        endpoint: endpoint.into(),
        token: token.into(),
        pid: std::process::id(),
        product_version: unterm_protocol::PRODUCT_VERSION.into(),
    };
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(&info)?)?;
    std::fs::rename(tmp, path)?;
    Ok(())
}

pub fn clear_discovery() -> Result<()> {
    if let Some(path) = discovery_path() {
        if path.exists() {
            std::fs::remove_file(path)?;
        }
    }
    Ok(())
}

/// Held for the lifetime of the owning Core process. The OS releases
/// the underlying lock when the process exits (cleanly or not), so a
/// crashed Core never leaves a stale lock behind.
pub struct InstanceLock {
    _file: std::fs::File,
}

pub fn instance_lock_path() -> Option<std::path::PathBuf> {
    discovery_path().map(|path| path.with_file_name("core.lock"))
}

/// Try to become the single Core instance for this user.
/// Returns `Ok(None)` when another live Core already holds the lock.
pub fn try_acquire_instance_lock() -> Result<Option<InstanceLock>> {
    let path = instance_lock_path()
        .ok_or_else(|| anyhow::anyhow!("no local data directory for core lock"))?;
    try_acquire_instance_lock_at(&path)
}

pub fn try_acquire_instance_lock_at(path: &std::path::Path) -> Result<Option<InstanceLock>> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    Ok(open_lock_file(path)?.map(|file| InstanceLock { _file: file }))
}

#[cfg(windows)]
fn open_lock_file(path: &std::path::Path) -> Result<Option<std::fs::File>> {
    use std::os::windows::fs::OpenOptionsExt;
    const ERROR_SHARING_VIOLATION: i32 = 32;
    match std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .share_mode(0)
        .open(path)
    {
        Ok(file) => Ok(Some(file)),
        Err(err) if err.raw_os_error() == Some(ERROR_SHARING_VIOLATION) => Ok(None),
        Err(err) => Err(err).context("open core instance lock"),
    }
}

#[cfg(unix)]
fn open_lock_file(path: &std::path::Path) -> Result<Option<std::fs::File>> {
    use std::os::unix::io::AsRawFd;
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(path)
        .context("open core instance lock")?;
    let rc = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
    if rc == 0 {
        Ok(Some(file))
    } else {
        let err = std::io::Error::last_os_error();
        if err.raw_os_error() == Some(libc::EWOULDBLOCK) {
            Ok(None)
        } else {
            Err(err).context("flock core instance lock")
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct Request {
    pub id: Option<String>,
    pub method: String,
    pub token: Option<String>,
    #[serde(default)]
    pub params: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Response<T> {
    pub id: Option<String>,
    pub ok: bool,
    pub result: Option<T>,
    pub error: Option<CoreError>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CoreError {
    pub code: String,
    pub message: String,
}

pub fn response_ok<T: Serialize>(id: Option<String>, result: T) -> Response<T> {
    Response {
        id,
        ok: true,
        result: Some(result),
        error: None,
    }
}

pub fn response_error<T: Serialize>(
    id: Option<String>,
    code: &'static str,
    message: impl Into<String>,
) -> Response<T> {
    Response {
        id,
        ok: false,
        result: None,
        error: Some(CoreError {
            code: code.into(),
            message: message.into(),
        }),
    }
}

/// Authenticated local server shared by GUI, CLI and automation clients.
pub struct CoreServer {
    listener: TcpListener,
    token: String,
    started_at: String,
    running: Arc<AtomicBool>,
    draining: Arc<AtomicBool>,
}

impl CoreServer {
    pub fn bind<A: ToSocketAddrs>(address: A, token: impl Into<String>) -> Result<Self> {
        let listener = TcpListener::bind(address).context("bind unterm-core listener")?;
        listener
            .set_nonblocking(true)
            .context("set core listener nonblocking")?;
        Ok(Self {
            listener,
            token: token.into(),
            started_at: chrono::Utc::now().to_rfc3339(),
            running: Arc::new(AtomicBool::new(true)),
            draining: Arc::new(AtomicBool::new(false)),
        })
    }

    pub fn endpoint(&self) -> Result<std::net::SocketAddr> {
        Ok(self.listener.local_addr()?)
    }

    pub fn run(&self) -> Result<()> {
        while self.running.load(Ordering::Acquire) {
            match self.listener.accept() {
                Ok((stream, _)) => {
                    stream
                        .set_nonblocking(false)
                        .context("set core client blocking")?;
                    let token = self.token.clone();
                    let started_at = self.started_at.clone();
                    let running = self.running.clone();
                    let draining = self.draining.clone();
                    std::thread::spawn(move || {
                        if let Err(err) =
                            handle_stream(stream, &token, &started_at, &running, &draining)
                        {
                            eprintln!("unterm-core client error: {err:#}");
                        }
                    });
                }
                Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(10))
                }
                Err(err) => return Err(err).context("accept unterm-core client"),
            }
        }
        Ok(())
    }
}

fn handle_stream(
    mut stream: TcpStream,
    token: &str,
    started_at: &str,
    running: &AtomicBool,
    draining: &AtomicBool,
) -> Result<()> {
    let reader = BufReader::new(stream.try_clone()?);
    for line in reader.lines() {
        let line = line?;
        let request: Request = match serde_json::from_str(&line) {
            Ok(request) => request,
            Err(err) => {
                writeln!(
                    stream,
                    "{}",
                    serde_json::to_string(&response_error::<()>(
                        None,
                        "invalid_request",
                        err.to_string()
                    ))?
                )?;
                stream.flush()?;
                continue;
            }
        };
        if request.token.as_deref() != Some(token) {
            writeln!(
                stream,
                "{}",
                serde_json::to_string(&response_error::<()>(
                    request.id,
                    "unauthenticated",
                    "invalid core token"
                ))?
            )?;
            stream.flush()?;
            continue;
        }
        let should_stop = request.method == "core.shutdown";
        if request.method == "session.create" && draining.load(Ordering::Acquire) {
            writeln!(
                stream,
                "{}",
                serde_json::to_string(&response_error::<()>(
                    request.id,
                    "draining",
                    "core is draining; new sessions are rejected"
                ))?
            )?;
            stream.flush()?;
            continue;
        }
        let response = dispatch(request, started_at, draining);
        writeln!(stream, "{response}")?;
        stream.flush()?;
        if should_stop {
            running.store(false, Ordering::Release);
            break;
        }
    }
    Ok(())
}

fn dispatch(request: Request, started_at: &str, draining: &AtomicBool) -> String {
    match dispatch_inner(&request, started_at, draining) {
        Ok(response) => response,
        Err(err) => serde_json::to_string(&response_error::<()>(
            request.id,
            "internal_error",
            format!("{err:#}"),
        ))
        .unwrap_or_else(|_| r#"{"ok":false}"#.into()),
    }
}

fn dispatch_inner(
    request: &Request,
    started_at: &str,
    draining: &AtomicBool,
) -> Result<String> {
    let engine = unterm_engine::next_core();
    let id = request.id.clone();
    let response = match request.method.as_str() {
        "core.info" => serde_json::to_string(&response_ok(
            id,
            BuildHandshake::current(ProcessRole::Core, std::process::id(), started_at),
        ))?,
        "core.health" | "core.readiness" => {
            let status = if draining.load(Ordering::Acquire) {
                "draining"
            } else {
                "ready"
            };
            serde_json::to_string(&response_ok(id, serde_json::json!({"status": status})))?
        }
        "core.drain" => {
            draining.store(true, Ordering::Release);
            serde_json::to_string(&response_ok(
                id,
                serde_json::json!({"status":"draining"}),
            ))?
        }
        "core.shutdown" => serde_json::to_string(&response_ok(
            id,
            serde_json::json!({"status":"stopping"}),
        ))?,
        "session.create" => {
            let cols = request
                .params
                .get("cols")
                .and_then(|v| v.as_u64())
                .unwrap_or(120) as usize;
            let rows = request
                .params
                .get("rows")
                .and_then(|v| v.as_u64())
                .unwrap_or(30) as usize;
            let command_dir = request
                .params
                .get("cwd")
                .and_then(|v| v.as_str())
                .filter(|dir| !dir.is_empty())
                .map(str::to_owned);
            let command = request
                .params
                .get("argv")
                .and_then(|v| v.as_array())
                .map(|argv| {
                    argv.iter()
                        .filter_map(|item| item.as_str().map(std::ffi::OsString::from))
                        .collect::<Vec<_>>()
                })
                .filter(|argv| !argv.is_empty())
                .map(CommandBuilder::from_argv);
            let session = engine.create_session(CreateSessionRequest {
                cols,
                rows,
                command_dir,
                command,
                env: Vec::new(),
                launch_policy: Default::default(),
            })?;
            serde_json::to_string(&response_ok(id, session))?
        }
        "session.list" => {
            serde_json::to_string(&response_ok(id, engine.list_sessions()?))?
        }
        "session.write" => {
            let pane_id = required_pane_id(request)?;
            let data = request
                .params
                .get("data")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            engine.write_input(pane_id, data)?;
            serde_json::to_string(&response_ok(
                id,
                serde_json::json!({"written": true}),
            ))?
        }
        "session.screen" => {
            let pane_id = required_pane_id(request)?;
            serde_json::to_string(&response_ok(id, engine.read_screen(pane_id)?))?
        }
        "session.frame" => {
            let pane_id = required_pane_id(request)?;
            let since_revision = request
                .params
                .get("since_revision")
                .and_then(|v| v.as_u64());
            serde_json::to_string(&response_ok(
                id,
                engine.read_render_frame(pane_id, since_revision)?,
            ))?
        }
        "session.resize" => {
            let pane_id = required_pane_id(request)?;
            let cols = request
                .params
                .get("cols")
                .and_then(|v| v.as_u64())
                .unwrap_or(120) as usize;
            let rows = request
                .params
                .get("rows")
                .and_then(|v| v.as_u64())
                .unwrap_or(30) as usize;
            engine.resize_session(pane_id, cols, rows)?;
            serde_json::to_string(&response_ok(
                id,
                serde_json::json!({"resized": true}),
            ))?
        }
        "session.close" => {
            let pane_id = required_pane_id(request)?;
            engine.destroy_session(pane_id)?;
            serde_json::to_string(&response_ok(
                id,
                serde_json::json!({"status":"closed"}),
            ))?
        }
        method => serde_json::to_string(&response_error::<()>(
            id,
            "method_not_found",
            method,
        ))?,
    };
    Ok(response)
}

fn required_pane_id(request: &Request) -> Result<usize> {
    request
        .params
        .get("pane_id")
        .and_then(|v| v.as_u64())
        .map(|pane_id| pane_id as usize)
        .ok_or_else(|| anyhow::anyhow!("pane_id is required"))
}

pub struct CoreClient {
    stream: TcpStream,
    token: String,
}

impl CoreClient {
    pub fn connect<A: ToSocketAddrs>(address: A, token: impl Into<String>) -> Result<Self> {
        Ok(Self {
            stream: TcpStream::connect(address).context("connect unterm-core")?,
            token: token.into(),
        })
    }

    pub fn request<T: for<'de> Deserialize<'de>>(&mut self, method: &str) -> Result<Response<T>> {
        self.request_with_params(method, serde_json::Value::Null)
    }

    pub fn request_with_params<T: for<'de> Deserialize<'de>>(
        &mut self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<Response<T>> {
        let request = serde_json::json!({
            "id": uuid::Uuid::new_v4().to_string(),
            "method": method,
            "token": self.token,
            "params": params,
        });
        writeln!(self.stream, "{request}")?;
        self.stream.flush()?;
        let mut line = String::new();
        BufReader::new(self.stream.try_clone()?).read_line(&mut line)?;
        Ok(serde_json::from_str(&line).context("decode unterm-core response")?)
    }

    /// Query the Core's identity and fail loudly on version skew.
    /// Version-mismatched client/core pairs must not talk past the
    /// handshake: silently degraded sessions are worse than a clear
    /// startup error.
    pub fn handshake(&mut self) -> Result<BuildHandshake> {
        let info: Response<BuildHandshake> = self.request("core.info")?;
        let info = info
            .result
            .ok_or_else(|| anyhow::anyhow!("unterm-core returned no identity"))?;
        let compatibility = info.compatibility();
        if !compatibility.is_usable() {
            anyhow::bail!(
                "unterm-core is not compatible ({}): core pid {} runs {} ({}), this client is {} ({})",
                compatibility.error_code().unwrap_or("incompatible"),
                info.pid,
                info.product_version,
                info.protocol_version,
                unterm_protocol::PRODUCT_VERSION,
                unterm_protocol::PROTOCOL_VERSION,
            );
        }
        Ok(info)
    }
}

/// Ensure the per-user Core process is available and return its
/// discovery record. GUI, CLI and MCP entry points share this path.
pub fn ensure_running() -> Result<DiscoveryInfo> {
    if let Some(info) = read_discovery()? {
        if let Ok(mut client) = CoreClient::connect(&info.endpoint, &info.token) {
            // Version skew is a hard error: do not fall through and
            // spawn a second core alongside a live-but-incompatible one.
            client.handshake()?;
            let health: Response<serde_json::Value> = client.request("core.health")?;
            if health.ok {
                return Ok(info);
            }
        }
    }
    let current = std::env::current_exe().context("resolve unterm executable path")?;
    let core_name = if cfg!(windows) {
        "unterm-core.exe"
    } else {
        "unterm-core"
    };
    let core_path = current.with_file_name(core_name);
    let mut command = std::process::Command::new(&core_path);
    command
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
        .spawn()
        .with_context(|| format!("start unterm-core at {}", core_path.display()))?;
    for _ in 0..100 {
        if let Some(info) = read_discovery()? {
            if let Ok(mut client) = CoreClient::connect(&info.endpoint, &info.token) {
                client.handshake()?;
                let health: Response<serde_json::Value> = client.request("core.health")?;
                if health.ok {
                    return Ok(info);
                }
            }
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    anyhow::bail!("unterm-core did not become ready")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn start_server(token: &str) -> (std::net::SocketAddr, std::thread::JoinHandle<()>) {
        let server = CoreServer::bind(("127.0.0.1", 0), token).unwrap();
        let endpoint = server.endpoint().unwrap();
        let worker = std::thread::spawn(move || server.run().unwrap());
        std::thread::sleep(Duration::from_millis(20));
        (endpoint, worker)
    }

    #[test]
    fn handshake_reports_core_role_and_compatibility() {
        let (endpoint, worker) = start_server("handshake-token");
        let mut client = CoreClient::connect(endpoint, "handshake-token").unwrap();
        let info = client.handshake().unwrap();
        assert_eq!(info.process_role, ProcessRole::Core);
        assert_eq!(info.product_version, unterm_protocol::PRODUCT_VERSION);
        assert!(!info.build_commit.is_empty());
        assert!(!info.started_at.is_empty());
        let _: Response<serde_json::Value> = client.request("core.shutdown").unwrap();
        worker.join().unwrap();
    }

    #[test]
    fn rejects_wrong_token() {
        let (endpoint, worker) = start_server("right-token");
        let mut client = CoreClient::connect(endpoint, "wrong-token").unwrap();
        let info: Response<BuildHandshake> = client.request("core.info").unwrap();
        assert!(!info.ok);
        assert_eq!(info.error.unwrap().code, "unauthenticated");
        let mut owner = CoreClient::connect(endpoint, "right-token").unwrap();
        let _: Response<serde_json::Value> = owner.request("core.shutdown").unwrap();
        worker.join().unwrap();
    }

    #[test]
    fn instance_lock_is_exclusive_and_released_on_drop() {
        let path = std::env::temp_dir().join(format!(
            "unterm-core-lock-test-{}.lock",
            uuid::Uuid::new_v4()
        ));
        let first = try_acquire_instance_lock_at(&path).unwrap();
        assert!(first.is_some(), "first acquire should win the lock");
        let second = try_acquire_instance_lock_at(&path).unwrap();
        assert!(second.is_none(), "second acquire must observe the held lock");
        drop(first);
        let third = try_acquire_instance_lock_at(&path).unwrap();
        assert!(third.is_some(), "lock must be reacquirable after release");
        drop(third);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn session_round_trip_through_core_ipc() {
        let (endpoint, worker) = start_server("session-token");
        let mut client = CoreClient::connect(endpoint, "session-token").unwrap();
        let argv: Vec<&str> = if cfg!(windows) {
            vec!["cmd.exe"]
        } else {
            vec!["sh"]
        };
        let created: Response<serde_json::Value> = client
            .request_with_params(
                "session.create",
                serde_json::json!({"cols": 80, "rows": 24, "argv": argv}),
            )
            .unwrap();
        assert!(created.ok, "session.create failed: {:?}", created.error);
        let pane_id = created.result.unwrap()["id"].as_u64().unwrap();

        let written: Response<serde_json::Value> = client
            .request_with_params(
                "session.write",
                serde_json::json!({"pane_id": pane_id, "data": "echo core-ready\r\n"}),
            )
            .unwrap();
        assert!(written.ok);

        // The shell needs a moment to echo. Poll rather than sleep once.
        let mut saw_output = false;
        for _ in 0..50 {
            let screen: Response<serde_json::Value> = client
                .request_with_params(
                    "session.screen",
                    serde_json::json!({"pane_id": pane_id}),
                )
                .unwrap();
            let result = screen.result.unwrap_or_default();
            let text = result["lines"]
                .as_array()
                .map(|lines| {
                    lines
                        .iter()
                        .filter_map(|line| line.as_str())
                        .collect::<Vec<_>>()
                        .join("\n")
                })
                .unwrap_or_default();
            if text.contains("core-ready") {
                saw_output = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        assert!(saw_output, "shell output never reached the core screen");

        let frame: Response<serde_json::Value> = client
            .request_with_params(
                "session.frame",
                serde_json::json!({"pane_id": pane_id}),
            )
            .unwrap();
        let revision = frame.result.unwrap()["revision"].as_u64().unwrap();
        assert!(revision > 0);

        let listed: Response<serde_json::Value> = client.request("session.list").unwrap();
        assert!(listed.ok);

        let closed: Response<serde_json::Value> = client
            .request_with_params(
                "session.close",
                serde_json::json!({"pane_id": pane_id}),
            )
            .unwrap();
        assert!(closed.ok);

        let _: Response<serde_json::Value> = client.request("core.shutdown").unwrap();
        worker.join().unwrap();
    }

    #[test]
    fn drain_rejects_new_sessions_but_keeps_existing() {
        let (endpoint, worker) = start_server("drain-token");
        let mut client = CoreClient::connect(endpoint, "drain-token").unwrap();
        let argv: Vec<&str> = if cfg!(windows) {
            vec!["cmd.exe"]
        } else {
            vec!["sh"]
        };
        let created: Response<serde_json::Value> = client
            .request_with_params(
                "session.create",
                serde_json::json!({"cols": 80, "rows": 24, "argv": argv}),
            )
            .unwrap();
        assert!(created.ok);
        let pane_id = created.result.unwrap()["id"].as_u64().unwrap();

        let drained: Response<serde_json::Value> = client.request("core.drain").unwrap();
        assert_eq!(
            drained.result.unwrap()["status"].as_str(),
            Some("draining")
        );
        let health: Response<serde_json::Value> = client.request("core.health").unwrap();
        assert_eq!(health.result.unwrap()["status"].as_str(), Some("draining"));

        let rejected: Response<serde_json::Value> = client
            .request_with_params(
                "session.create",
                serde_json::json!({"cols": 80, "rows": 24, "argv": argv}),
            )
            .unwrap();
        assert!(!rejected.ok);
        assert_eq!(rejected.error.unwrap().code, "draining");

        // The pre-drain session keeps working.
        let screen: Response<serde_json::Value> = client
            .request_with_params(
                "session.screen",
                serde_json::json!({"pane_id": pane_id}),
            )
            .unwrap();
        assert!(screen.ok);

        let closed: Response<serde_json::Value> = client
            .request_with_params(
                "session.close",
                serde_json::json!({"pane_id": pane_id}),
            )
            .unwrap();
        assert!(closed.ok);
        let _: Response<serde_json::Value> = client.request("core.shutdown").unwrap();
        worker.join().unwrap();
    }
}
