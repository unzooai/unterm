use anyhow::{Context, Result};
use portable_pty::{native_pty_system, ChildKiller, CommandBuilder, MasterPty, PtySize, PtySystem};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io::Result as IoResult;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream, ToSocketAddrs};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::time::Duration;

pub const PRODUCT_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const PROTOCOL_VERSION: &str = "1";
pub const DATA_SCHEMA_VERSION: &str = "1";
pub const BUILD_COMMIT: &str = match option_env!("UNTERM_BUILD_COMMIT") {
    Some(commit) => commit,
    None => "dev",
};

static STARTED_AT: std::sync::OnceLock<u64> = std::sync::OnceLock::new();

/// Unix timestamp of when this process initialised its Core state.
/// Pinned on first access; `CoreServer::bind` touches it so the value
/// reflects server start rather than first client query.
pub fn started_at() -> u64 {
    *STARTED_AT.get_or_init(|| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|elapsed| elapsed.as_secs())
            .unwrap_or(0)
    })
}

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
        product_version: PRODUCT_VERSION.into(),
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

/// Ensure the per-user Core process is available and return its discovery
/// record. GUI and standalone mux entry points use the same lifecycle path.
pub fn ensure_running() -> Result<DiscoveryInfo> {
    if let Some(info) = read_discovery()? {
        if let Ok(mut client) = CoreClient::connect(&info.endpoint, &info.token) {
            // Protocol skew is a hard error: do not fall through and
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
        command.creation_flags(0x08000000);
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreInfo {
    pub product: String,
    pub product_version: String,
    #[serde(default)]
    pub build_commit: String,
    pub protocol_version: String,
    pub data_schema_version: String,
    pub process_role: String,
    pub pid: u32,
    #[serde(default)]
    pub started_at: u64,
}

impl CoreInfo {
    pub fn current() -> Self {
        Self {
            product: "unterm-core".into(),
            product_version: PRODUCT_VERSION.into(),
            build_commit: BUILD_COMMIT.into(),
            protocol_version: PROTOCOL_VERSION.into(),
            data_schema_version: DATA_SCHEMA_VERSION.into(),
            process_role: "core".into(),
            pid: std::process::id(),
            started_at: started_at(),
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub id: String,
    pub shell: String,
    pub cwd: String,
    pub status: String,
    pub pid: Option<u32>,
}

#[derive(Default)]
struct SessionStore {
    sessions: BTreeMap<String, RuntimeSession>,
}

struct RuntimeSession {
    info: SessionInfo,
    master: Option<Box<dyn MasterPty + Send>>,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    killer: Option<Box<dyn ChildKiller + Send + Sync>>,
    output: Arc<Mutex<Vec<u8>>>,
    input: Option<Arc<Mutex<Vec<u8>>>>,
    terminal: Arc<Mutex<wezterm_term::Terminal>>,
}

struct SharedWriter(Arc<Mutex<Box<dyn Write + Send>>>);

struct NullWriter;
impl Write for NullWriter {
    fn write(&mut self, bytes: &[u8]) -> IoResult<usize> { Ok(bytes.len()) }
    fn flush(&mut self) -> IoResult<()> { Ok(()) }
}

#[derive(Debug)]
struct CoreTermConfig;

impl wezterm_term::TerminalConfiguration for CoreTermConfig {
    fn color_palette(&self) -> wezterm_term::color::ColorPalette {
        Default::default()
    }
}

impl Write for SharedWriter {
    fn write(&mut self, bytes: &[u8]) -> IoResult<usize> {
        self.0
            .lock()
            .map_err(|_| std::io::Error::other("writer poisoned"))?
            .write(bytes)
    }
    fn flush(&mut self) -> IoResult<()> {
        self.0
            .lock()
            .map_err(|_| std::io::Error::other("writer poisoned"))?
            .flush()
    }
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
    running: Arc<AtomicBool>,
    draining: Arc<AtomicBool>,
    sessions: Arc<Mutex<SessionStore>>,
}

impl CoreServer {
    pub fn bind<A: ToSocketAddrs>(address: A, token: impl Into<String>) -> Result<Self> {
        started_at();
        let listener = TcpListener::bind(address).context("bind unterm-core listener")?;
        listener
            .set_nonblocking(true)
            .context("set core listener nonblocking")?;
        Ok(Self {
            listener,
            token: token.into(),
            running: Arc::new(AtomicBool::new(true)),
            draining: Arc::new(AtomicBool::new(false)),
            sessions: Arc::new(Mutex::new(SessionStore::default())),
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
                    let running = self.running.clone();
                    let draining = self.draining.clone();
                    let sessions = self.sessions.clone();
                    std::thread::spawn(move || {
                        if let Err(err) =
                            handle_stream(stream, &token, &running, &draining, &sessions)
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
    running: &AtomicBool,
    draining: &AtomicBool,
    sessions: &Arc<Mutex<SessionStore>>,
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
        let creates_session = matches!(
            request.method.as_str(),
            "session.create" | "session.external.create" | "session.serial.create"
        );
        if creates_session && draining.load(Ordering::Acquire) {
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
        let response = match request.method.as_str() {
            "core.info" => serde_json::to_string(&response_ok(request.id, CoreInfo::current()))?,
            "core.health" | "core.readiness" => {
                let status = if draining.load(Ordering::Acquire) {
                    "draining"
                } else {
                    "ready"
                };
                serde_json::to_string(&response_ok(
                    request.id,
                    serde_json::json!({"status": status}),
                ))?
            }
            "core.drain" => {
                draining.store(true, Ordering::Release);
                serde_json::to_string(&response_ok(
                    request.id,
                    serde_json::json!({"status":"draining"}),
                ))?
            }
            "core.shutdown" => serde_json::to_string(&response_ok(
                request.id,
                serde_json::json!({"status":"stopping"}),
            ))?,
            "session.list" => {
                let store = sessions
                    .lock()
                    .map_err(|_| anyhow::anyhow!("session store poisoned"))?;
                serde_json::to_string(&response_ok(
                    request.id,
                    store
                        .sessions
                        .values()
                        .map(|session| session.info.clone())
                        .collect::<Vec<_>>(),
                ))?
            }
            "session.external.create" => {
                let mut store = sessions
                    .lock()
                    .map_err(|_| anyhow::anyhow!("session store poisoned"))?;
                let id = uuid::Uuid::new_v4().to_string();
                let cwd = request.params.get("cwd").and_then(|v| v.as_str()).unwrap_or("").to_owned();
                let shell = request.params.get("shell").and_then(|v| v.as_str()).unwrap_or("external").to_owned();
                let size = wezterm_term::TerminalSize {
                    rows: request.params.get("rows").and_then(|v| v.as_u64()).unwrap_or(30) as usize,
                    cols: request.params.get("cols").and_then(|v| v.as_u64()).unwrap_or(120) as usize,
                    ..Default::default()
                };
                let output = Arc::new(Mutex::new(Vec::new()));
                let input = Arc::new(Mutex::new(Vec::new()));
                let terminal = Arc::new(Mutex::new(wezterm_term::Terminal::new(
                    size,
                    Arc::new(CoreTermConfig),
                    "unterm-core",
                    PRODUCT_VERSION,
                    Box::new(NullWriter),
                )));
                let session = SessionInfo { id: id.clone(), shell, cwd, status: "running".into(), pid: None };
                store.sessions.insert(id, RuntimeSession {
                    info: session.clone(),
                    master: None,
                    writer: Arc::new(Mutex::new(Box::new(NullWriter))),
                    killer: None,
                    output,
                    input: Some(input),
                    terminal,
                });
                serde_json::to_string(&response_ok(request.id, session))?
            }
            "session.feed" => {
                let store = sessions.lock().map_err(|_| anyhow::anyhow!("session store poisoned"))?;
                let id = request.params.get("id").and_then(|v| v.as_str()).unwrap_or_default();
                let data = request.params.get("data").and_then(|v| v.as_str()).unwrap_or_default();
                let fed = if let Some(session) = store.sessions.get(id) {
                    if let Ok(bytes) = data.as_bytes().get(..).map(|b| b.to_vec()).ok_or(()) {
                        if let Ok(mut term) = session.terminal.lock() { term.advance_bytes(&bytes); }
                        if let Ok(mut output) = session.output.lock() {
                            output.extend_from_slice(&bytes);
                            if output.len() > 65536 { let trim = output.len() - 65536; output.drain(..trim); }
                        }
                        true
                    } else { false }
                } else { false };
                serde_json::to_string(&response_ok(request.id, serde_json::json!({"fed": fed})))?
            }
            "session.input" => {
                let store = sessions.lock().map_err(|_| anyhow::anyhow!("session store poisoned"))?;
                let id = request.params.get("id").and_then(|v| v.as_str()).unwrap_or_default();
                let max_bytes = request.params.get("max_bytes").and_then(|v| v.as_u64()).unwrap_or(65536) as usize;
                let data = store.sessions.get(id).and_then(|session| session.input.as_ref()).map(|input| {
                    let mut input = input.lock().unwrap();
                    let take = input.len().min(max_bytes);
                    String::from_utf8_lossy(&input.drain(..take).collect::<Vec<_>>()).into_owned()
                }).unwrap_or_default();
                serde_json::to_string(&response_ok(request.id, serde_json::json!({"data": data})))?
            }
            "session.create" => {
                let mut store = sessions
                    .lock()
                    .map_err(|_| anyhow::anyhow!("session store poisoned"))?;
                let id = uuid::Uuid::new_v4().to_string();
                let cwd = request
                    .params
                    .get("cwd")
                    .and_then(|v| v.as_str())
                    .map(str::to_owned)
                    .unwrap_or(std::env::current_dir()?.display().to_string());
                let shell = request
                    .params
                    .get("shell")
                    .and_then(|v| v.as_str())
                    .unwrap_or("default");
                let requested_command = request
                    .params
                    .get("command")
                    .and_then(|v| v.as_str())
                    .map(str::trim)
                    .filter(|command| !command.is_empty());
                let requested_argv = request
                    .params
                    .get("argv")
                    .and_then(|v| v.as_array())
                    .map(|argv| {
                        argv.iter()
                            .filter_map(|item| item.as_str().map(std::ffi::OsString::from))
                            .collect::<Vec<_>>()
                    })
                    .filter(|argv| !argv.is_empty());
                let has_argv = requested_argv.is_some();
                let size = PtySize {
                    rows: request
                        .params
                        .get("rows")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(30) as u16,
                    cols: request
                        .params
                        .get("cols")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(120) as u16,
                    ..PtySize::default()
                };
                let pty = native_pty_system()
                    .openpty(size)
                    .context("open session pty")?;
                let mut command = if let Some(argv) = requested_argv {
                    CommandBuilder::from_argv(argv)
                } else if let Some(shell_path) = request
                    .params
                    .get("shell_path")
                    .and_then(|v| v.as_str())
                    .filter(|path| !path.is_empty())
                {
                    CommandBuilder::new(shell_path)
                } else if cfg!(windows) {
                    CommandBuilder::new("cmd.exe")
                } else {
                    CommandBuilder::new("sh")
                };
                if !has_argv {
                    if let Some(command_line) = requested_command {
                        if cfg!(windows) {
                            command.arg("/K");
                            command.arg(command_line);
                        } else {
                            command.arg("-c");
                            command.arg(format!("{command_line}; exec \"${{SHELL:-sh}}\""));
                        }
                    }
                }
                if !cwd.is_empty() {
                    command.cwd(&cwd);
                }
                let child = pty
                    .slave
                    .spawn_command(command)
                    .context("spawn session shell")?;
                let pid = child.process_id();
                let killer = Some(child.clone_killer());
                let writer: Arc<Mutex<Box<dyn Write + Send>>> = Arc::new(Mutex::new(
                    pty.master.take_writer().context("open session writer")?,
                ));
                let terminal = Arc::new(Mutex::new(wezterm_term::Terminal::new(
                    wezterm_term::TerminalSize {
                        rows: size.rows as usize,
                        cols: size.cols as usize,
                        ..Default::default()
                    },
                    Arc::new(CoreTermConfig),
                    "unterm-core",
                    PRODUCT_VERSION,
                    Box::new(SharedWriter(writer.clone())),
                )));
                let mut reader = pty
                    .master
                    .try_clone_reader()
                    .context("open session reader")?;
                let output = Arc::new(Mutex::new(Vec::new()));
                let output_reader = output.clone();
                let terminal_reader = terminal.clone();
                let sessions_for_reader = sessions.clone();
                let session_id_for_reader = id.clone();
                std::thread::spawn(move || {
                    let mut sink = [0u8; 4096];
                    loop {
                        match reader.read(&mut sink) {
                            Ok(0) | Err(_) => break,
                            Ok(count) => {
                                if let Ok(mut term) = terminal_reader.lock() {
                                    term.advance_bytes(&sink[..count]);
                                }
                                if let Ok(mut buffer) = output_reader.lock() {
                                    buffer.extend_from_slice(&sink[..count]);
                                    if buffer.len() > 65536 {
                                        let trim = buffer.len() - 65536;
                                        buffer.drain(..trim);
                                    }
                                }
                            }
                        }
                    }
                    if let Ok(mut all) = sessions_for_reader.lock() {
                        if let Some(session) = all.sessions.get_mut(&session_id_for_reader) {
                            session.info.status = "exited".into();
                        }
                    }
                });
                let session = SessionInfo {
                    id: id.clone(),
                    shell: shell.into(),
                    cwd,
                    status: "running".into(),
                    pid,
                };
                store.sessions.insert(
                    id,
                    RuntimeSession {
                        info: session.clone(),
                        master: Some(pty.master),
                        writer,
                        killer,
                        output,
                        input: None,
                        terminal,
                    },
                );
                serde_json::to_string(&response_ok(request.id, session))?
            }
            "session.close" => {
                let mut store = sessions
                    .lock()
                    .map_err(|_| anyhow::anyhow!("session store poisoned"))?;
                let id = request
                    .params
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let removed = store
                    .sessions
                    .remove(id)
                    .map(|mut session| {
                        session
                            .killer
                            .as_mut()
                            .map(|killer| killer.kill().is_ok())
                            .unwrap_or(true)
                    })
                    .unwrap_or(false);
                serde_json::to_string(&response_ok(
                    request.id,
                    serde_json::json!({"status": if removed { "closed" } else { "not_found" }}),
                ))?
            }
            "session.write" => {
                let mut store = sessions
                    .lock()
                    .map_err(|_| anyhow::anyhow!("session store poisoned"))?;
                let id = request
                    .params
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let data = request
                    .params
                    .get("data")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let written = if let Some(session) = store.sessions.get_mut(id) {
                    if let Some(input) = &session.input {
                        input.lock().map_err(|_| anyhow::anyhow!("input poisoned"))?.extend_from_slice(data.as_bytes());
                    } else {
                        let mut writer = session.writer.lock().map_err(|_| anyhow::anyhow!("writer poisoned"))?;
                        writer.write_all(data.as_bytes())?;
                        writer.flush()?;
                    }
                    true
                } else {
                    false
                };
                serde_json::to_string(&response_ok(
                    request.id,
                    serde_json::json!({"written": written}),
                ))?
            }
            "session.read" => {
                let store = sessions
                    .lock()
                    .map_err(|_| anyhow::anyhow!("session store poisoned"))?;
                let id = request
                    .params
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let max_bytes = request
                    .params
                    .get("max_bytes")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(65536) as usize;
                let consume = request
                    .params
                    .get("consume")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let session_status = store
                    .sessions
                    .get(id)
                    .map(|session| session.info.status.clone());
                let text = store.sessions.get(id).map(|session| {
                    session
                        .output
                        .lock()
                        .map(|mut buffer| {
                            if consume {
                                let take = buffer.len().min(max_bytes);
                                let bytes = buffer.drain(..take).collect::<Vec<_>>();
                                String::from_utf8_lossy(&bytes).into_owned()
                            } else {
                                let start = buffer.len().saturating_sub(max_bytes);
                                String::from_utf8_lossy(&buffer[start..]).into_owned()
                            }
                        })
                        .unwrap_or_default()
                });
                serde_json::to_string(&response_ok(
                    request.id,
                    serde_json::json!({"found": text.is_some(), "status": session_status.unwrap_or_else(|| "missing".into()), "text": text.unwrap_or_default()}),
                ))?
            }
            "session.screen" => {
                let store = sessions
                    .lock()
                    .map_err(|_| anyhow::anyhow!("session store poisoned"))?;
                let id = request
                    .params
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let result = if let Some(session) = store.sessions.get(id) {
                    let term = session
                        .terminal
                        .lock()
                        .map_err(|_| anyhow::anyhow!("terminal poisoned"))?;
                    let screen = term.screen();
                    let meta_only = request
                        .params
                        .get("meta_only")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    let requested_start = request.params.get("start").and_then(|v| v.as_i64());
                    let requested_end = request.params.get("end").and_then(|v| v.as_i64());
                    let (first_row, physical_range) = match (requested_start, requested_end) {
                        (Some(start), Some(end)) if end > start => {
                            let stable = start as wezterm_term::StableRowIndex
                                ..end as wezterm_term::StableRowIndex;
                            let range = screen.stable_range(&stable);
                            (screen.phys_to_stable_row_index(range.start), range)
                        }
                        _ => (screen.visible_row_to_stable_row(0), 0..screen.physical_rows),
                    };
                    let lines = if meta_only {
                        Vec::new()
                    } else {
                        screen
                            .lines_in_phys_range(physical_range)
                            .into_iter()
                            .map(|line| line.as_str().trim_end().to_owned())
                            .collect::<Vec<_>>()
                    };
                    let cursor = term.cursor_pos();
                    serde_json::json!({
                        "found": true,
                        "lines": lines,
                        "first_row": first_row,
                        "cols": screen.physical_cols,
                        "rows": screen.physical_rows,
                        "scrollback_rows": screen.scrollback_rows(),
                        "physical_top": screen.visible_row_to_stable_row(0),
                        "scrollback_top": screen.phys_to_stable_row_index(0),
                        "cursor": serde_json::to_value(cursor)?,
                        "seqno": term.current_seqno(),
                        "title": term.get_title(),
                        "cwd": term.get_current_dir().map(|url| url.to_string()),
                        "alt_screen": term.is_alt_screen_active(),
                        "mouse_grabbed": term.is_mouse_grabbed(),
                    })
                } else {
                    serde_json::json!({"found": false, "lines": []})
                };
                serde_json::to_string(&response_ok(request.id, result))?
            }
            "session.lines" => {
                let store = sessions
                    .lock()
                    .map_err(|_| anyhow::anyhow!("session store poisoned"))?;
                let id = request
                    .params
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let result = if let Some(session) = store.sessions.get(id) {
                    let term = session
                        .terminal
                        .lock()
                        .map_err(|_| anyhow::anyhow!("terminal poisoned"))?;
                    let screen = term.screen();
                    let requested_start = request.params.get("start").and_then(|v| v.as_i64());
                    let requested_end = request.params.get("end").and_then(|v| v.as_i64());
                    let (first_row, physical_range) = match (requested_start, requested_end) {
                        (Some(start), Some(end)) if end > start => {
                            let stable = start as wezterm_term::StableRowIndex
                                ..end as wezterm_term::StableRowIndex;
                            let range = screen.stable_range(&stable);
                            (screen.phys_to_stable_row_index(range.start), range)
                        }
                        _ => (screen.visible_row_to_stable_row(0), 0..screen.physical_rows),
                    };
                    let lines = screen.lines_in_phys_range(physical_range);
                    serde_json::json!({
                        "found": true,
                        "first_row": first_row,
                        "seqno": term.current_seqno(),
                        "lines": serde_json::to_value(&lines)?,
                    })
                } else {
                    serde_json::json!({"found": false, "lines": []})
                };
                serde_json::to_string(&response_ok(request.id, result))?
            }
            "session.changed" => {
                let store = sessions
                    .lock()
                    .map_err(|_| anyhow::anyhow!("session store poisoned"))?;
                let id = request
                    .params
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let result = if let Some(session) = store.sessions.get(id) {
                    let term = session
                        .terminal
                        .lock()
                        .map_err(|_| anyhow::anyhow!("terminal poisoned"))?;
                    let screen = term.screen();
                    let start = request
                        .params
                        .get("start")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0) as wezterm_term::StableRowIndex;
                    let end = request
                        .params
                        .get("end")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(screen.physical_rows as i64)
                        as wezterm_term::StableRowIndex;
                    let since = request
                        .params
                        .get("seqno")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0) as usize;
                    let rows = screen.get_changed_stable_rows(start..end, since);
                    serde_json::json!({
                        "found": true,
                        "rows": rows,
                        "seqno": term.current_seqno(),
                    })
                } else {
                    serde_json::json!({"found": false, "rows": []})
                };
                serde_json::to_string(&response_ok(request.id, result))?
            }
            "session.zones" => {
                let store = sessions
                    .lock()
                    .map_err(|_| anyhow::anyhow!("session store poisoned"))?;
                let id = request
                    .params
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let result = if let Some(session) = store.sessions.get(id) {
                    let mut term = session
                        .terminal
                        .lock()
                        .map_err(|_| anyhow::anyhow!("terminal poisoned"))?;
                    let zones = term.get_semantic_zones().unwrap_or_default();
                    serde_json::json!({"found": true, "zones": serde_json::to_value(&zones)?})
                } else {
                    serde_json::json!({"found": false, "zones": []})
                };
                serde_json::to_string(&response_ok(request.id, result))?
            }
            "session.serial.create" => create_serial_session(&request, sessions)?,
            "session.resize" => {
                let store = sessions
                    .lock()
                    .map_err(|_| anyhow::anyhow!("session store poisoned"))?;
                let id = request
                    .params
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let cols = request
                    .params
                    .get("cols")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(120) as u16;
                let rows = request
                    .params
                    .get("rows")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(30) as u16;
                let resized = if let Some(session) = store.sessions.get(id) {
                    session
                        .master
                        .as_ref()
                        .map(|master| master.resize(PtySize {
                            rows,
                            cols,
                            ..PtySize::default()
                        }))
                        .unwrap_or(Ok(()))
                        .is_ok()
                } else {
                    false
                };
                serde_json::to_string(&response_ok(
                    request.id,
                    serde_json::json!({"resized": resized}),
                ))?
            }
            method => serde_json::to_string(&response_error::<()>(
                request.id,
                "method_not_found",
                method,
            ))?,
        };
        writeln!(stream, "{response}")?;
        stream.flush()?;
        if should_stop {
            running.store(false, Ordering::Release);
            break;
        }
    }
    Ok(())
}

fn create_serial_session(request: &Request, sessions: &Arc<Mutex<SessionStore>>) -> Result<String> {
    let mut store = sessions
        .lock()
        .map_err(|_| anyhow::anyhow!("session store poisoned"))?;
    let id = uuid::Uuid::new_v4().to_string();
    let port = request
        .params
        .get("port")
        .and_then(|v| v.as_str())
        .filter(|port| !port.is_empty())
        .ok_or_else(|| anyhow::anyhow!("serial session requires port"))?;
    let mut serial = portable_pty::serial::SerialTty::new(&port);
    if let Some(baud) = request.params.get("baud").and_then(|v| v.as_u64()) {
        serial.set_baud_rate(baud as u32);
    }
    let size = PtySize {
        rows: request
            .params
            .get("rows")
            .and_then(|v| v.as_u64())
            .unwrap_or(30) as u16,
        cols: request
            .params
            .get("cols")
            .and_then(|v| v.as_u64())
            .unwrap_or(120) as u16,
        ..PtySize::default()
    };
    let pair = serial.openpty(size).context("open serial session")?;
    let child = pair
        .slave
        .spawn_command(CommandBuilder::new_default_prog())
        .context("attach serial session")?;
    let killer = Some(child.clone_killer());
    let writer: Arc<Mutex<Box<dyn Write + Send>>> = Arc::new(Mutex::new(
        pair.master.take_writer().context("open serial writer")?,
    ));
    let terminal = Arc::new(Mutex::new(wezterm_term::Terminal::new(
        wezterm_term::TerminalSize {
            rows: size.rows as usize,
            cols: size.cols as usize,
            ..Default::default()
        },
        Arc::new(CoreTermConfig),
        "unterm-core",
        PRODUCT_VERSION,
        Box::new(SharedWriter(writer.clone())),
    )));
    let mut reader = pair
        .master
        .try_clone_reader()
        .context("open serial reader")?;
    let output = Arc::new(Mutex::new(Vec::new()));
    let output_reader = output.clone();
    let terminal_reader = terminal.clone();
    let sessions_for_reader = sessions.clone();
    let session_id_for_reader = id.clone();
    std::thread::spawn(move || {
        let mut sink = [0u8; 4096];
        loop {
            match reader.read(&mut sink) {
                Ok(0) | Err(_) => break,
                Ok(count) => {
                    if let Ok(mut term) = terminal_reader.lock() {
                        term.advance_bytes(&sink[..count]);
                    }
                    if let Ok(mut buffer) = output_reader.lock() {
                        buffer.extend_from_slice(&sink[..count]);
                        if buffer.len() > 65536 {
                            let trim = buffer.len() - 65536;
                            buffer.drain(..trim);
                        }
                    }
                }
            }
        }
        if let Ok(mut all) = sessions_for_reader.lock() {
            if let Some(session) = all.sessions.get_mut(&session_id_for_reader) {
                session.info.status = "exited".into();
            }
        }
    });
    let session = SessionInfo {
        id: id.clone(),
        shell: "serial".into(),
        cwd: port.to_string(),
        status: "running".into(),
        pid: None,
    };
    store.sessions.insert(
        id,
        RuntimeSession {
            info: session.clone(),
            master: Some(pair.master),
            writer,
            killer,
            output,
            input: None,
            terminal,
        },
    );
    Ok(serde_json::to_string(&response_ok(
        request.id.clone(),
        session,
    ))?)
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

    /// Query the Core's identity and fail loudly on protocol skew.
    /// Version-mismatched client/core pairs must not talk past the
    /// handshake: silently degraded sessions are worse than a clear
    /// startup error.
    pub fn handshake(&mut self) -> Result<CoreInfo> {
        let info: Response<CoreInfo> = self.request("core.info")?;
        let info = info
            .result
            .ok_or_else(|| anyhow::anyhow!("unterm-core returned no info"))?;
        if info.protocol_version != PROTOCOL_VERSION {
            anyhow::bail!(
                "unterm-core protocol mismatch: core pid {} (version {}, commit {}) speaks protocol {} but this client requires {}",
                info.pid,
                info.product_version,
                info.build_commit,
                info.protocol_version,
                PROTOCOL_VERSION,
            );
        }
        Ok(info)
    }

    pub fn request_with_params<T: for<'de> Deserialize<'de>>(
        &mut self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<Response<T>> {
        let request = serde_json::json!({"id": uuid::Uuid::new_v4().to_string(), "method": method, "token": self.token, "params": params});
        writeln!(self.stream, "{request}")?;
        self.stream.flush()?;
        let mut line = String::new();
        BufReader::new(self.stream.try_clone()?).read_line(&mut line)?;
        Ok(serde_json::from_str(&line).context("decode unterm-core response")?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn protocol_versions_are_present() {
        let info = CoreInfo::current();
        assert_eq!(info.product, "unterm-core");
        assert!(!info.product_version.is_empty());
        assert!(!info.build_commit.is_empty());
        assert!(info.started_at > 0);
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
    fn drain_rejects_new_sessions_but_keeps_existing() {
        let server = CoreServer::bind(("127.0.0.1", 0), "drain-token").unwrap();
        let endpoint = server.endpoint().unwrap();
        let worker = std::thread::spawn(move || server.run().unwrap());
        std::thread::sleep(Duration::from_millis(20));
        let mut client = CoreClient::connect(endpoint, "drain-token").unwrap();
        let info = client.handshake().unwrap();
        assert_eq!(info.protocol_version, PROTOCOL_VERSION);

        let created: Response<SessionInfo> = client
            .request_with_params(
                "session.external.create",
                serde_json::json!({"shell":"pre-drain","cols":80,"rows":24}),
            )
            .unwrap();
        let id = created.result.unwrap().id;

        let drained: Response<serde_json::Value> = client.request("core.drain").unwrap();
        assert_eq!(
            drained.result.unwrap()["status"].as_str(),
            Some("draining")
        );
        let health: Response<serde_json::Value> = client.request("core.health").unwrap();
        assert_eq!(health.result.unwrap()["status"].as_str(), Some("draining"));

        let rejected: Response<SessionInfo> = client
            .request_with_params(
                "session.external.create",
                serde_json::json!({"shell":"post-drain","cols":80,"rows":24}),
            )
            .unwrap();
        assert!(!rejected.ok);
        assert_eq!(rejected.error.unwrap().code, "draining");

        // The pre-drain session keeps working.
        let _: Response<serde_json::Value> = client
            .request_with_params(
                "session.feed",
                serde_json::json!({"id": &id, "data": "still-alive"}),
            )
            .unwrap();
        let screen: Response<serde_json::Value> = client
            .request_with_params("session.screen", serde_json::json!({"id": &id}))
            .unwrap();
        assert!(screen.result.unwrap()["found"].as_bool().unwrap());

        let _: Response<serde_json::Value> = client.request("core.shutdown").unwrap();
        worker.join().unwrap();
    }

    #[test]
    fn authenticated_client_can_query_and_shutdown() {
        let server = CoreServer::bind(("127.0.0.1", 0), "test-token").unwrap();
        let endpoint = server.endpoint().unwrap();
        let worker = std::thread::spawn(move || server.run().unwrap());
        std::thread::sleep(Duration::from_millis(20));
        let mut client = CoreClient::connect(endpoint, "test-token").unwrap();
        let info: Response<CoreInfo> = client.request("core.info").unwrap();
        assert!(info.ok);
        let created: Response<SessionInfo> = client
            .request_with_params(
                "session.create",
                serde_json::json!({
                    "shell":"test-shell",
                    "argv": if cfg!(windows) { vec!["cmd.exe"] } else { vec!["sh"] },
                    "cwd":"C:\\"
                }),
            )
            .unwrap();
        assert!(created.ok);
        assert_eq!(created.result.as_ref().unwrap().shell, "test-shell");
        let session_id = created.result.as_ref().unwrap().id.clone();
        let written: Response<serde_json::Value> = client
            .request_with_params(
                "session.write",
                serde_json::json!({"id": &session_id, "data": "echo core-ready\r\n"}),
            )
            .unwrap();
        assert!(written.result.unwrap()["written"].as_bool().unwrap());
        std::thread::sleep(Duration::from_millis(50));
        let output: Response<serde_json::Value> = client
            .request_with_params("session.read", serde_json::json!({"id": &session_id}))
            .unwrap();
        assert!(output.result.unwrap()["found"].as_bool().unwrap());
        let consumed: Response<serde_json::Value> = client
            .request_with_params(
                "session.read",
                serde_json::json!({"id": &session_id, "consume": true}),
            )
            .unwrap();
        assert_eq!(consumed.result.unwrap()["status"].as_str(), Some("running"));
        let screen: Response<serde_json::Value> = client
            .request_with_params("session.screen", serde_json::json!({"id": &session_id}))
            .unwrap();
        assert!(screen.result.unwrap()["found"].as_bool().unwrap());
        let sessions: Response<Vec<SessionInfo>> = client.request("session.list").unwrap();
        assert_eq!(sessions.result.unwrap().len(), 1);
        let _: Response<serde_json::Value> = client.request("core.shutdown").unwrap();
        worker.join().unwrap();
    }

    #[test]
    fn session_lines_carry_attributes_and_render_metadata() {
        let server = CoreServer::bind(("127.0.0.1", 0), "render-token").unwrap();
        let endpoint = server.endpoint().unwrap();
        let worker = std::thread::spawn(move || server.run().unwrap());
        std::thread::sleep(Duration::from_millis(20));
        let mut client = CoreClient::connect(endpoint, "render-token").unwrap();
        let created: Response<SessionInfo> = client
            .request_with_params(
                "session.external.create",
                serde_json::json!({"shell":"render","cols":80,"rows":24}),
            )
            .unwrap();
        let id = created.result.unwrap().id;

        let _: Response<serde_json::Value> = client
            .request_with_params(
                "session.feed",
                serde_json::json!({"id": &id, "data": "\u{1b}]0;my-title\u{7}\u{1b}[31mred\u{1b}[0m plain"}),
            )
            .unwrap();

        // Metadata: title, seqno and alt-screen state come back from the
        // Core-held terminal.
        let screen: Response<serde_json::Value> = client
            .request_with_params(
                "session.screen",
                serde_json::json!({"id": &id, "meta_only": true}),
            )
            .unwrap();
        let meta = screen.result.unwrap();
        assert_eq!(meta["title"].as_str(), Some("my-title"));
        assert!(meta["seqno"].as_u64().unwrap() > 0);
        assert_eq!(meta["alt_screen"].as_bool(), Some(false));
        assert!(meta["lines"].as_array().unwrap().is_empty());
        assert!(meta["cursor"]["shape"].is_string() || meta["cursor"]["shape"].is_object());
        let seqno = meta["seqno"].as_u64().unwrap();

        // Rich lines: the serialized Line deserializes on the client side
        // and keeps the red foreground attribute.
        let lines: Response<serde_json::Value> = client
            .request_with_params(
                "session.lines",
                serde_json::json!({"id": &id, "start": 0, "end": 1}),
            )
            .unwrap();
        let result = lines.result.unwrap();
        assert_eq!(result["found"].as_bool(), Some(true));
        let parsed: Vec<wezterm_term::Line> =
            serde_json::from_value(result["lines"].clone()).unwrap();
        assert_eq!(parsed.len(), 1);
        let cells = parsed[0].visible_cells().collect::<Vec<_>>();
        assert_eq!(cells[0].str(), "r");
        assert_ne!(
            cells[0].attrs().foreground(),
            wezterm_term::CellAttributes::default().foreground()
        );

        // Damage tracking: everything is dirty since seqno 0, nothing is
        // dirty once the client has caught up to the current seqno.
        let changed: Response<serde_json::Value> = client
            .request_with_params(
                "session.changed",
                serde_json::json!({"id": &id, "start": 0, "end": 24, "seqno": 0}),
            )
            .unwrap();
        let changed = changed.result.unwrap();
        assert!(!changed["rows"].as_array().unwrap().is_empty());
        let caught_up: Response<serde_json::Value> = client
            .request_with_params(
                "session.changed",
                serde_json::json!({"id": &id, "start": 0, "end": 24, "seqno": seqno}),
            )
            .unwrap();
        assert!(caught_up.result.unwrap()["rows"]
            .as_array()
            .unwrap()
            .is_empty());

        // Alt screen flips the reported state.
        let _: Response<serde_json::Value> = client
            .request_with_params(
                "session.feed",
                serde_json::json!({"id": &id, "data": "\u{1b}[?1049h"}),
            )
            .unwrap();
        let screen: Response<serde_json::Value> = client
            .request_with_params(
                "session.screen",
                serde_json::json!({"id": &id, "meta_only": true}),
            )
            .unwrap();
        assert_eq!(screen.result.unwrap()["alt_screen"].as_bool(), Some(true));

        let zones: Response<serde_json::Value> = client
            .request_with_params("session.zones", serde_json::json!({"id": &id}))
            .unwrap();
        assert_eq!(zones.result.unwrap()["found"].as_bool(), Some(true));

        let _: Response<serde_json::Value> = client.request("core.shutdown").unwrap();
        worker.join().unwrap();
    }

    #[test]
    fn external_session_bridges_input_and_screen() {
        let server = CoreServer::bind(("127.0.0.1", 0), "external-token").unwrap();
        let endpoint = server.endpoint().unwrap();
        let worker = std::thread::spawn(move || server.run().unwrap());
        std::thread::sleep(Duration::from_millis(20));
        let mut client = CoreClient::connect(endpoint, "external-token").unwrap();
        let created: Response<SessionInfo> = client.request_with_params(
            "session.external.create",
            serde_json::json!({"shell":"tmux-control","cols":80,"rows":24}),
        ).unwrap();
        let id = created.result.unwrap().id;
        let _: Response<serde_json::Value> = client.request_with_params(
            "session.write", serde_json::json!({"id": &id, "data":"keys"}),
        ).unwrap();
        let input: Response<serde_json::Value> = client.request_with_params(
            "session.input", serde_json::json!({"id": &id}),
        ).unwrap();
        assert_eq!(input.result.unwrap()["data"].as_str(), Some("keys"));
        let _: Response<serde_json::Value> = client.request_with_params(
            "session.feed", serde_json::json!({"id": &id, "data":"core"}),
        ).unwrap();
        let screen: Response<serde_json::Value> = client.request_with_params(
            "session.screen", serde_json::json!({"id": &id}),
        ).unwrap();
        assert!(screen.result.unwrap()["found"].as_bool().unwrap());
        let _: Response<serde_json::Value> = client.request("core.shutdown").unwrap();
        worker.join().unwrap();
    }
}
