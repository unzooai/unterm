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
    CreateSessionRequest, CursorSnapshot, InputEngine, LaunchPolicySnapshot,
    PaneModesSnapshot, RenderFrameSnapshot, ScreenEngine, ScreenLine, ScreenSearchMatch,
    ScreenSnapshot, ScrollbackTextRequest, ScrollbackTextSnapshot, SearchMode,
    SessionActivitySnapshot, SessionEngine, SessionSnapshot, ShellSnapshot,
    SplitDirection, SplitSessionRequest, StyledScreenSnapshot, StyledScrollbackSnapshot,
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

/// One session-affecting change, pushed to `core.events` subscribers.
///
/// Edge notifications, not a replayable log: a subscriber that connects
/// late bootstraps from `session.list` and only hears what happens next.
/// The durable, cursor-addressable event store is M2's work, not this.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum CoreEvent {
    SessionCreated { pane_id: usize },
    SessionClosed { pane_id: usize },
    SessionDead { pane_id: usize, reason: Option<String> },
    ScreenUpdated { pane_id: usize, revision: u64 },
    Draining,
}

/// Fan-out point between the engine watcher and `core.events`
/// connections. Dead subscribers are dropped on the next publish.
struct EventHub {
    subscribers: std::sync::Mutex<Vec<std::sync::mpsc::Sender<String>>>,
}

impl EventHub {
    fn new() -> Self {
        Self {
            subscribers: std::sync::Mutex::new(Vec::new()),
        }
    }

    fn subscribe(&self) -> std::sync::mpsc::Receiver<String> {
        let (sender, receiver) = std::sync::mpsc::channel();
        self.subscribers
            .lock()
            .expect("event hub lock poisoned")
            .push(sender);
        receiver
    }

    fn subscriber_count(&self) -> usize {
        self.subscribers
            .lock()
            .expect("event hub lock poisoned")
            .len()
    }

    fn publish(&self, event: &CoreEvent) {
        let Ok(line) = serde_json::to_string(event) else {
            return;
        };
        self.subscribers
            .lock()
            .expect("event hub lock poisoned")
            .retain(|sender| sender.send(line.clone()).is_ok());
    }
}

/// Poll the engine for changes and publish them as events.
///
/// The engine has no wakeup hook yet, so this is where the polling
/// lives -- once, in the Core, instead of in every client's frame
/// loop. Adding a real hook later changes this function, not the wire
/// protocol. Idle cost stays low: with no subscribers the watcher only
/// keeps its baseline fresh, at a slower cadence.
fn watch_engine_events(hub: Arc<EventHub>, running: Arc<AtomicBool>, draining: Arc<AtomicBool>) {
    let engine = unterm_engine::next_core();
    let mut known: std::collections::HashMap<usize, (bool, u64)> = std::collections::HashMap::new();
    let mut announced_draining = false;
    while running.load(Ordering::Acquire) {
        let has_subscribers = hub.subscriber_count() > 0;

        if draining.load(Ordering::Acquire) && !announced_draining {
            announced_draining = true;
            hub.publish(&CoreEvent::Draining);
        }

        // Publishing to an empty hub is a no-op, so events are never
        // gated on subscriber presence: gating would race a subscriber
        // arriving between the check and a session appearing, and lose
        // that session's created event for good.
        if let Ok(sessions) = engine.list_sessions() {
            let mut seen = std::collections::HashSet::new();
            for session in &sessions {
                seen.insert(session.id);
                let revision = engine.screen_revision(session.id).unwrap_or(0);
                match known.get(&session.id) {
                    None => {
                        known.insert(session.id, (session.is_dead, revision));
                        hub.publish(&CoreEvent::SessionCreated {
                            pane_id: session.id,
                        });
                    }
                    Some(&(was_dead, last_revision)) => {
                        if session.is_dead && !was_dead {
                            hub.publish(&CoreEvent::SessionDead {
                                pane_id: session.id,
                                reason: session.dead_reason.clone(),
                            });
                        }
                        if revision != last_revision {
                            hub.publish(&CoreEvent::ScreenUpdated {
                                pane_id: session.id,
                                revision,
                            });
                        }
                        known.insert(session.id, (session.is_dead, revision));
                    }
                }
            }
            let closed: Vec<usize> = known
                .keys()
                .copied()
                .filter(|id| !seen.contains(id))
                .collect();
            for pane_id in closed {
                known.remove(&pane_id);
                hub.publish(&CoreEvent::SessionClosed { pane_id });
            }
        }

        let interval = if has_subscribers { 25 } else { 250 };
        std::thread::sleep(Duration::from_millis(interval));
    }
}

/// Authenticated local server shared by GUI, CLI and automation clients.
pub struct CoreServer {
    listener: TcpListener,
    token: String,
    started_at: String,
    running: Arc<AtomicBool>,
    draining: Arc<AtomicBool>,
    events: Arc<EventHub>,
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
            events: Arc::new(EventHub::new()),
        })
    }

    pub fn endpoint(&self) -> Result<std::net::SocketAddr> {
        Ok(self.listener.local_addr()?)
    }

    pub fn run(&self) -> Result<()> {
        let watcher = {
            let hub = self.events.clone();
            let running = self.running.clone();
            let draining = self.draining.clone();
            std::thread::Builder::new()
                .name("core-event-watcher".into())
                .spawn(move || watch_engine_events(hub, running, draining))
                .context("spawn core event watcher")?
        };
        while self.running.load(Ordering::Acquire) {
            match self.listener.accept() {
                Ok((stream, _)) => {
                    stream
                        .set_nonblocking(false)
                        .context("set core client blocking")?;
                    // Request/response frames are latency-bound, not
                    // throughput-bound; Nagle only adds stalls here.
                    stream
                        .set_nodelay(true)
                        .context("set core client nodelay")?;
                    let token = self.token.clone();
                    let started_at = self.started_at.clone();
                    let running = self.running.clone();
                    let draining = self.draining.clone();
                    let events = self.events.clone();
                    std::thread::spawn(move || {
                        if let Err(err) =
                            handle_stream(stream, &token, &started_at, &running, &draining, &events)
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
        let _ = watcher.join();
        Ok(())
    }
}

fn handle_stream(
    mut stream: TcpStream,
    token: &str,
    started_at: &str,
    running: &AtomicBool,
    draining: &AtomicBool,
    events: &Arc<EventHub>,
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
        if request.method == "core.events" {
            // This connection becomes a one-way event feed: acknowledge,
            // then push until the subscriber hangs up or the core stops.
            let receiver = events.subscribe();
            writeln!(
                stream,
                "{}",
                serde_json::to_string(&response_ok(
                    request.id,
                    serde_json::json!({"subscribed": true}),
                ))?
            )?;
            stream.flush()?;
            loop {
                match receiver.recv_timeout(Duration::from_millis(500)) {
                    Ok(line) => {
                        if writeln!(stream, "{line}").is_err() || stream.flush().is_err() {
                            return Ok(());
                        }
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                        if !running.load(Ordering::Acquire) {
                            return Ok(());
                        }
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => return Ok(()),
                }
            }
        }
        let should_stop = request.method == "core.shutdown";
        let creates_session =
            matches!(request.method.as_str(), "session.create" | "session.split");
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
            let (cols, rows) = parse_dimensions(&request.params);
            let session = engine.create_session(CreateSessionRequest {
                cols,
                rows,
                command_dir: parse_cwd(&request.params),
                command: parse_argv(&request.params),
                env: parse_env(&request.params),
                launch_policy: parse_launch_policy(&request.params),
            })?;
            serde_json::to_string(&response_ok(id, session))?
        }
        "session.split" => {
            let source_pane_id = required_pane_id(request)?;
            let direction = match request
                .params
                .get("direction")
                .and_then(|v| v.as_str())
                .unwrap_or("right")
            {
                "left" => SplitDirection::Left,
                "down" => SplitDirection::Down,
                "up" => SplitDirection::Up,
                _ => SplitDirection::Right,
            };
            let size_percent = request
                .params
                .get("size_percent")
                .and_then(|v| v.as_u64())
                .unwrap_or(50) as u8;
            let session = engine.split_session(SplitSessionRequest {
                source_pane_id,
                direction,
                size_percent,
                command_dir: parse_cwd(&request.params),
                command: parse_argv(&request.params),
                env: parse_env(&request.params),
                launch_policy: parse_launch_policy(&request.params),
            })?;
            serde_json::to_string(&response_ok(id, session))?
        }
        "session.get" => {
            let pane_id = required_pane_id(request)?;
            serde_json::to_string(&response_ok(id, engine.get_session(pane_id)?))?
        }
        "session.focus" => {
            engine.focus_session(required_pane_id(request)?)?;
            serde_json::to_string(&response_ok(id, serde_json::json!({"focused": true})))?
        }
        "session.shell" => {
            let pane_id = required_pane_id(request)?;
            serde_json::to_string(&response_ok(id, engine.shell(pane_id)?))?
        }
        "session.activity" => {
            let pane_id = required_pane_id(request)?;
            serde_json::to_string(&response_ok(id, engine.activity(pane_id)?))?
        }
        "session.modes" => {
            let pane_id = required_pane_id(request)?;
            serde_json::to_string(&response_ok(id, engine.pane_modes(pane_id)?))?
        }
        "session.styled_screen" => {
            let pane_id = required_pane_id(request)?;
            serde_json::to_string(&response_ok(id, engine.read_styled_screen(pane_id)?))?
        }
        "session.visible_text" => {
            let pane_id = required_pane_id(request)?;
            serde_json::to_string(&response_ok(id, engine.read_visible_text(pane_id)?))?
        }
        "session.lines" => {
            let pane_id = required_pane_id(request)?;
            let start = request
                .params
                .get("start")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            let count = request
                .params
                .get("count")
                .and_then(|v| v.as_u64())
                .unwrap_or(100) as usize;
            serde_json::to_string(&response_ok(id, engine.read_lines(pane_id, start, count)?))?
        }
        "session.scrollback" => {
            let pane_id = required_pane_id(request)?;
            let limit = request
                .params
                .get("limit")
                .and_then(|v| v.as_u64())
                .unwrap_or(1000) as usize;
            serde_json::to_string(&response_ok(id, engine.read_scrollback(pane_id, limit)?))?
        }
        "session.scrollback_text" => {
            let pane_id = required_pane_id(request)?;
            serde_json::to_string(&response_ok(
                id,
                engine.read_scrollback_text(pane_id, parse_scrollback_request(&request.params))?,
            ))?
        }
        "session.styled_scrollback" => {
            let pane_id = required_pane_id(request)?;
            serde_json::to_string(&response_ok(
                id,
                engine.read_styled_scrollback(pane_id, parse_scrollback_request(&request.params))?,
            ))?
        }
        "session.search" => {
            let pane_id = required_pane_id(request)?;
            let pattern = request
                .params
                .get("pattern")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("pattern is required"))?;
            let mode = match request.params.get("mode").and_then(|v| v.as_str()) {
                Some("case_sensitive") => SearchMode::CaseSensitive,
                _ => SearchMode::CaseInsensitive,
            };
            let max_results = request
                .params
                .get("max_results")
                .and_then(|v| v.as_u64())
                .unwrap_or(100) as usize;
            serde_json::to_string(&response_ok(
                id,
                engine.search(pane_id, pattern, mode, max_results)?,
            ))?
        }
        "session.cursor" => {
            let pane_id = required_pane_id(request)?;
            serde_json::to_string(&response_ok(id, engine.cursor(pane_id)?))?
        }
        "session.erase_scrollback" => {
            let pane_id = required_pane_id(request)?;
            let include_viewport = request
                .params
                .get("include_viewport")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            engine.erase_scrollback(pane_id, include_viewport)?;
            serde_json::to_string(&response_ok(id, serde_json::json!({"erased": true})))?
        }
        "session.paste" => {
            let pane_id = required_pane_id(request)?;
            let data = request
                .params
                .get("data")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            engine.paste_input(pane_id, data)?;
            serde_json::to_string(&response_ok(id, serde_json::json!({"pasted": true})))?
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

fn parse_dimensions(params: &serde_json::Value) -> (usize, usize) {
    let cols = params.get("cols").and_then(|v| v.as_u64()).unwrap_or(120) as usize;
    let rows = params.get("rows").and_then(|v| v.as_u64()).unwrap_or(30) as usize;
    (cols, rows)
}

fn parse_cwd(params: &serde_json::Value) -> Option<String> {
    params
        .get("cwd")
        .and_then(|v| v.as_str())
        .filter(|dir| !dir.is_empty())
        .map(str::to_owned)
}

fn parse_argv(params: &serde_json::Value) -> Option<CommandBuilder> {
    params
        .get("argv")
        .and_then(|v| v.as_array())
        .map(|argv| {
            argv.iter()
                .filter_map(|item| item.as_str().map(std::ffi::OsString::from))
                .collect::<Vec<_>>()
        })
        .filter(|argv| !argv.is_empty())
        .map(CommandBuilder::from_argv)
}

fn parse_env(params: &serde_json::Value) -> Vec<(String, String)> {
    params
        .get("env")
        .cloned()
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default()
}

fn parse_launch_policy(params: &serde_json::Value) -> LaunchPolicySnapshot {
    params
        .get("launch_policy")
        .cloned()
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default()
}

fn parse_scrollback_request(params: &serde_json::Value) -> ScrollbackTextRequest {
    ScrollbackTextRequest {
        start_line: params.get("start_line").and_then(|v| v.as_i64()),
        end_line: params.get("end_line").and_then(|v| v.as_i64()),
        tail_lines: params.get("tail_lines").and_then(|v| v.as_i64()),
        escapes: params
            .get("escapes")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
    }
}

pub struct CoreClient {
    stream: TcpStream,
    token: String,
}

impl CoreClient {
    pub fn connect<A: ToSocketAddrs>(address: A, token: impl Into<String>) -> Result<Self> {
        let stream = TcpStream::connect(address).context("connect unterm-core")?;
        stream.set_nodelay(true).context("set core nodelay")?;
        Ok(Self {
            stream,
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

/// Blocking subscription to the Core's event feed.
///
/// Owns its own connection: events push on it continuously, so it
/// cannot share the request/response connection a `CoreEngineClient`
/// multiplexes.
pub struct CoreEventStream {
    reader: BufReader<TcpStream>,
}

impl CoreEventStream {
    pub fn connect<A: ToSocketAddrs>(address: A, token: impl Into<String>) -> Result<Self> {
        let stream = TcpStream::connect(address).context("connect unterm-core events")?;
        stream
            .set_nodelay(true)
            .context("set core events nodelay")?;
        let mut writer = stream.try_clone()?;
        let request = serde_json::json!({
            "id": uuid::Uuid::new_v4().to_string(),
            "method": "core.events",
            "token": token.into(),
            "params": serde_json::Value::Null,
        });
        writeln!(writer, "{request}")?;
        writer.flush()?;
        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        reader.read_line(&mut line)?;
        let ack: Response<serde_json::Value> =
            serde_json::from_str(&line).context("decode core.events ack")?;
        if !ack.ok {
            let code = ack.error.map(|error| error.code).unwrap_or_default();
            anyhow::bail!("core.events subscription refused ({code})");
        }
        Ok(Self { reader })
    }

    /// Block until the next event arrives. `Ok(None)` means the feed
    /// closed: the core shut down or the connection dropped.
    pub fn next_event(&mut self) -> Result<Option<CoreEvent>> {
        let mut line = String::new();
        if self.reader.read_line(&mut line)? == 0 {
            return Ok(None);
        }
        Ok(Some(
            serde_json::from_str(line.trim()).context("decode core event")?,
        ))
    }

    pub fn set_read_timeout(&self, timeout: Option<Duration>) -> Result<()> {
        self.reader
            .get_ref()
            .set_read_timeout(timeout)
            .context("set core event read timeout")
    }
}

/// The Core-backed engine a GUI or CLI holds in place of a local
/// `NextCoreEngine`. Implements the same engine traits, but every call
/// crosses the authenticated IPC boundary, so sessions live in the Core
/// process and survive this process exiting.
///
/// One TCP connection serves all callers; the mutex makes each
/// request/response pair atomic, so concurrent threads can never
/// interleave one request's frame with another's reply.
pub struct CoreEngineClient {
    inner: std::sync::Mutex<CoreClient>,
}

impl CoreEngineClient {
    /// Connect and complete the version handshake. Version skew is a
    /// hard error here for the same reason it is in `ensure_running`.
    pub fn connect<A: ToSocketAddrs>(address: A, token: impl Into<String>) -> Result<Self> {
        let mut client = CoreClient::connect(address, token)?;
        client.handshake()?;
        Ok(Self {
            inner: std::sync::Mutex::new(client),
        })
    }

    fn call<T: for<'de> Deserialize<'de>>(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<T> {
        let mut client = self
            .inner
            .lock()
            .map_err(|_| anyhow::anyhow!("unterm-core client lock poisoned"))?;
        let response: Response<T> = client.request_with_params(method, params)?;
        if let Some(error) = response.error {
            anyhow::bail!("unterm-core {method} failed ({}): {}", error.code, error.message);
        }
        response
            .result
            .ok_or_else(|| anyhow::anyhow!("unterm-core {method} returned no result"))
    }

    fn call_unit(&self, method: &str, params: serde_json::Value) -> Result<()> {
        let _: serde_json::Value = self.call(method, params)?;
        Ok(())
    }

    /// Mirror of `NextCoreEngine::pane_modes`, which the GUI needs per
    /// frame to decide who owns a mouse click.
    pub fn pane_modes(&self, pane_id: usize) -> Result<PaneModesSnapshot> {
        self.call("session.modes", serde_json::json!({"pane_id": pane_id}))
    }
}

fn command_argv(command: &Option<CommandBuilder>) -> Option<Vec<String>> {
    command.as_ref().map(|command| {
        command
            .get_argv()
            .iter()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect()
    })
}

impl SessionEngine for CoreEngineClient {
    fn list_sessions(&self) -> Result<Vec<SessionSnapshot>> {
        self.call("session.list", serde_json::Value::Null)
    }

    fn get_session(&self, pane_id: usize) -> Result<SessionSnapshot> {
        self.call("session.get", serde_json::json!({"pane_id": pane_id}))
    }

    fn create_session(&self, request: CreateSessionRequest) -> Result<SessionSnapshot> {
        let cwd = request.command_dir.clone().or_else(|| {
            request
                .command
                .as_ref()
                .and_then(|command| command.get_cwd())
                .map(|dir| dir.to_string_lossy().into_owned())
        });
        self.call(
            "session.create",
            serde_json::json!({
                "cols": request.cols,
                "rows": request.rows,
                "cwd": cwd,
                "argv": command_argv(&request.command),
                "env": request.env,
                "launch_policy": request.launch_policy,
            }),
        )
    }

    fn split_session(&self, request: SplitSessionRequest) -> Result<SessionSnapshot> {
        let direction = match request.direction {
            SplitDirection::Right => "right",
            SplitDirection::Left => "left",
            SplitDirection::Down => "down",
            SplitDirection::Up => "up",
        };
        self.call(
            "session.split",
            serde_json::json!({
                "pane_id": request.source_pane_id,
                "direction": direction,
                "size_percent": request.size_percent,
                "cwd": request.command_dir,
                "argv": command_argv(&request.command),
                "env": request.env,
                "launch_policy": request.launch_policy,
            }),
        )
    }

    fn focus_session(&self, pane_id: usize) -> Result<()> {
        self.call_unit("session.focus", serde_json::json!({"pane_id": pane_id}))
    }

    fn shell(&self, pane_id: usize) -> Result<ShellSnapshot> {
        self.call("session.shell", serde_json::json!({"pane_id": pane_id}))
    }

    fn activity(&self, pane_id: usize) -> Result<SessionActivitySnapshot> {
        self.call("session.activity", serde_json::json!({"pane_id": pane_id}))
    }

    fn resize_session(&self, pane_id: usize, cols: usize, rows: usize) -> Result<()> {
        self.call_unit(
            "session.resize",
            serde_json::json!({"pane_id": pane_id, "cols": cols, "rows": rows}),
        )
    }

    fn destroy_session(&self, pane_id: usize) -> Result<()> {
        self.call_unit("session.close", serde_json::json!({"pane_id": pane_id}))
    }
}

impl ScreenEngine for CoreEngineClient {
    fn read_screen(&self, pane_id: usize) -> Result<ScreenSnapshot> {
        self.call("session.screen", serde_json::json!({"pane_id": pane_id}))
    }

    fn erase_scrollback(&self, pane_id: usize, include_viewport: bool) -> Result<()> {
        self.call_unit(
            "session.erase_scrollback",
            serde_json::json!({"pane_id": pane_id, "include_viewport": include_viewport}),
        )
    }

    fn read_styled_screen(&self, pane_id: usize) -> Result<StyledScreenSnapshot> {
        self.call("session.styled_screen", serde_json::json!({"pane_id": pane_id}))
    }

    fn read_render_frame(
        &self,
        pane_id: usize,
        since_revision: Option<u64>,
    ) -> Result<RenderFrameSnapshot> {
        self.call(
            "session.frame",
            serde_json::json!({"pane_id": pane_id, "since_revision": since_revision}),
        )
    }

    fn read_visible_text(&self, pane_id: usize) -> Result<String> {
        self.call("session.visible_text", serde_json::json!({"pane_id": pane_id}))
    }

    fn read_lines(&self, pane_id: usize, start: i64, count: usize) -> Result<Vec<ScreenLine>> {
        self.call(
            "session.lines",
            serde_json::json!({"pane_id": pane_id, "start": start, "count": count}),
        )
    }

    fn read_scrollback(&self, pane_id: usize, limit: usize) -> Result<Vec<String>> {
        self.call(
            "session.scrollback",
            serde_json::json!({"pane_id": pane_id, "limit": limit}),
        )
    }

    fn read_scrollback_text(
        &self,
        pane_id: usize,
        request: ScrollbackTextRequest,
    ) -> Result<ScrollbackTextSnapshot> {
        self.call(
            "session.scrollback_text",
            scrollback_request_params(pane_id, &request),
        )
    }

    fn read_styled_scrollback(
        &self,
        pane_id: usize,
        request: ScrollbackTextRequest,
    ) -> Result<StyledScrollbackSnapshot> {
        self.call(
            "session.styled_scrollback",
            scrollback_request_params(pane_id, &request),
        )
    }

    fn search(
        &self,
        pane_id: usize,
        pattern: &str,
        mode: SearchMode,
        max_results: usize,
    ) -> Result<Vec<ScreenSearchMatch>> {
        let mode = match mode {
            SearchMode::CaseSensitive => "case_sensitive",
            SearchMode::CaseInsensitive => "case_insensitive",
        };
        self.call(
            "session.search",
            serde_json::json!({
                "pane_id": pane_id,
                "pattern": pattern,
                "mode": mode,
                "max_results": max_results,
            }),
        )
    }

    fn cursor(&self, pane_id: usize) -> Result<CursorSnapshot> {
        self.call("session.cursor", serde_json::json!({"pane_id": pane_id}))
    }
}

impl InputEngine for CoreEngineClient {
    fn write_input(&self, pane_id: usize, input: &str) -> Result<()> {
        self.call_unit(
            "session.write",
            serde_json::json!({"pane_id": pane_id, "data": input}),
        )
    }

    fn paste_input(&self, pane_id: usize, text: &str) -> Result<()> {
        self.call_unit(
            "session.paste",
            serde_json::json!({"pane_id": pane_id, "data": text}),
        )
    }
}

fn scrollback_request_params(
    pane_id: usize,
    request: &ScrollbackTextRequest,
) -> serde_json::Value {
    serde_json::json!({
        "pane_id": pane_id,
        "start_line": request.start_line,
        "end_line": request.end_line,
        "tail_lines": request.tail_lines,
        "escapes": request.escapes,
    })
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

    fn shell_argv() -> Vec<std::ffi::OsString> {
        if cfg!(windows) {
            vec!["cmd.exe".into()]
        } else {
            vec!["sh".into()]
        }
    }

    fn styled_screen_text(snapshot: &StyledScreenSnapshot) -> String {
        snapshot
            .lines
            .iter()
            .map(|line| line.cells.iter().map(|cell| cell.ch).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn facade_drives_sessions_over_ipc() {
        let (endpoint, worker) = start_server("facade-token");
        let facade = CoreEngineClient::connect(endpoint, "facade-token").unwrap();

        let session = facade
            .create_session(CreateSessionRequest {
                cols: 80,
                rows: 24,
                command_dir: None,
                command: Some(CommandBuilder::from_argv(shell_argv())),
                env: Vec::new(),
                launch_policy: Default::default(),
            })
            .unwrap();
        let pane_id = session.id;

        facade
            .write_input(pane_id, "echo facade-ready\r\n")
            .unwrap();

        let mut styled = None;
        for _ in 0..50 {
            let snapshot = facade.read_styled_screen(pane_id).unwrap();
            if styled_screen_text(&snapshot).contains("facade-ready") {
                styled = Some(snapshot);
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        let styled = styled.expect("styled screen never showed the echoed text");
        assert!(styled.revision > 0);

        // The incremental path a renderer relies on: an up-to-date
        // revision must come back empty instead of resending the frame.
        let unchanged = facade
            .read_render_frame(pane_id, Some(styled.revision))
            .unwrap();
        assert!(!unchanged.full);
        assert!(unchanged.lines.is_empty());

        assert!(facade
            .read_visible_text(pane_id)
            .unwrap()
            .contains("facade-ready"));
        let matches = facade
            .search(pane_id, "facade-ready", SearchMode::CaseInsensitive, 10)
            .unwrap();
        assert!(!matches.is_empty());

        let cursor = facade.cursor(pane_id).unwrap();
        assert!(!cursor.shape.is_empty());
        let modes = facade.pane_modes(pane_id).unwrap();
        assert!(!modes.alt_screen_active);
        let shell = facade.shell(pane_id).unwrap();
        assert!(!shell.process_name.is_empty());
        facade.activity(pane_id).unwrap();

        let listed = facade.list_sessions().unwrap();
        assert!(listed.iter().any(|s| s.id == pane_id));
        assert_eq!(facade.get_session(pane_id).unwrap().id, pane_id);

        facade.resize_session(pane_id, 100, 30).unwrap();
        assert_eq!(facade.get_session(pane_id).unwrap().cols, 100);

        facade.destroy_session(pane_id).unwrap();

        let mut owner = CoreClient::connect(endpoint, "facade-token").unwrap();
        let _: Response<serde_json::Value> = owner.request("core.shutdown").unwrap();
        worker.join().unwrap();
    }

    #[test]
    fn facade_split_creates_adjacent_session_and_drain_blocks_it() {
        let (endpoint, worker) = start_server("split-token");
        let facade = CoreEngineClient::connect(endpoint, "split-token").unwrap();

        let source = facade
            .create_session(CreateSessionRequest {
                cols: 120,
                rows: 40,
                command_dir: None,
                command: Some(CommandBuilder::from_argv(shell_argv())),
                env: Vec::new(),
                launch_policy: Default::default(),
            })
            .unwrap();

        let split = facade
            .split_session(SplitSessionRequest {
                source_pane_id: source.id,
                direction: SplitDirection::Right,
                size_percent: 50,
                command_dir: None,
                command: Some(CommandBuilder::from_argv(shell_argv())),
                env: Vec::new(),
                launch_policy: Default::default(),
            })
            .unwrap();
        assert_ne!(split.id, source.id);
        assert_eq!(split.split_from, Some(source.id));

        let mut owner = CoreClient::connect(endpoint, "split-token").unwrap();
        let _: Response<serde_json::Value> = owner.request("core.drain").unwrap();

        let blocked = facade.split_session(SplitSessionRequest {
            source_pane_id: source.id,
            direction: SplitDirection::Down,
            size_percent: 50,
            command_dir: None,
            command: Some(CommandBuilder::from_argv(shell_argv())),
            env: Vec::new(),
            launch_policy: Default::default(),
        });
        let err = blocked.expect_err("split must be refused while draining");
        assert!(err.to_string().contains("draining"), "got: {err:#}");

        facade.destroy_session(split.id).unwrap();
        facade.destroy_session(source.id).unwrap();
        let _: Response<serde_json::Value> = owner.request("core.shutdown").unwrap();
        worker.join().unwrap();
    }

    /// Read events until one satisfies the predicate. Other tests share
    /// the process-global engine, so unrelated sessions' events are
    /// expected here and must be skipped, not treated as failures.
    fn wait_for_event(
        events: &mut CoreEventStream,
        what: &str,
        mut predicate: impl FnMut(&CoreEvent) -> bool,
    ) -> CoreEvent {
        for _ in 0..500 {
            match events.next_event() {
                Ok(Some(event)) if predicate(&event) => return event,
                Ok(Some(_)) => continue,
                Ok(None) => panic!("event feed closed while waiting for {what}"),
                Err(err) => panic!("event feed failed while waiting for {what}: {err:#}"),
            }
        }
        panic!("gave up waiting for {what}");
    }

    #[test]
    fn events_stream_reports_session_lifecycle() {
        let (endpoint, worker) = start_server("events-token");
        let mut events = CoreEventStream::connect(endpoint, "events-token").unwrap();
        events
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();

        let facade = CoreEngineClient::connect(endpoint, "events-token").unwrap();
        let session = facade
            .create_session(CreateSessionRequest {
                cols: 80,
                rows: 24,
                command_dir: None,
                command: Some(CommandBuilder::from_argv(shell_argv())),
                env: Vec::new(),
                launch_policy: Default::default(),
            })
            .unwrap();
        let pane_id = session.id;

        wait_for_event(&mut events, "session_created", |event| {
            matches!(event, CoreEvent::SessionCreated { pane_id: id } if *id == pane_id)
        });

        facade
            .write_input(pane_id, "echo events-ready\r\n")
            .unwrap();
        let updated = wait_for_event(&mut events, "screen_updated", |event| {
            matches!(event, CoreEvent::ScreenUpdated { pane_id: id, .. } if *id == pane_id)
        });
        match updated {
            CoreEvent::ScreenUpdated { revision, .. } => assert!(revision > 0),
            _ => unreachable!(),
        }

        facade.destroy_session(pane_id).unwrap();
        wait_for_event(&mut events, "session_closed", |event| {
            matches!(event, CoreEvent::SessionClosed { pane_id: id } if *id == pane_id)
        });

        let mut owner = CoreClient::connect(endpoint, "events-token").unwrap();
        let _: Response<serde_json::Value> = owner.request("core.drain").unwrap();
        wait_for_event(&mut events, "draining", |event| {
            matches!(event, CoreEvent::Draining)
        });

        let _: Response<serde_json::Value> = owner.request("core.shutdown").unwrap();
        worker.join().unwrap();
    }

    /// Not a correctness test: measures what a GUI frame would pay to
    /// read a styled screen across the IPC boundary instead of from
    /// process-local memory. Run explicitly with
    /// `cargo test -p unterm-core --release -- --ignored --nocapture bench_styled`.
    #[test]
    #[ignore]
    fn bench_styled_screen_ipc_round_trip() {
        let (endpoint, worker) = start_server("bench-token");
        let facade = CoreEngineClient::connect(endpoint, "bench-token").unwrap();
        let session = facade
            .create_session(CreateSessionRequest {
                cols: 120,
                rows: 40,
                command_dir: None,
                command: Some(CommandBuilder::from_argv(shell_argv())),
                env: Vec::new(),
                launch_policy: Default::default(),
            })
            .unwrap();
        let pane_id = session.id;

        // Put real content on the screen so the payload is honest.
        for chunk in 0..5 {
            facade
                .write_input(pane_id, &format!("echo bench-fill-{chunk} {}\r\n", "x".repeat(80)))
                .unwrap();
        }
        std::thread::sleep(Duration::from_millis(500));

        let mut percentiles = |label: &str, mut samples: Vec<Duration>| {
            samples.sort();
            let p50 = samples[samples.len() / 2];
            let p95 = samples[samples.len() * 95 / 100];
            let max = samples[samples.len() - 1];
            println!("{label}: p50 {p50:?}, p95 {p95:?}, max {max:?}");
        };

        const ROUNDS: usize = 500;

        let mut styled = Vec::with_capacity(ROUNDS);
        for _ in 0..ROUNDS {
            let start = std::time::Instant::now();
            let snapshot = facade.read_styled_screen(pane_id).unwrap();
            styled.push(start.elapsed());
            assert_eq!(snapshot.cols, 120);
        }
        percentiles("read_styled_screen (full, 120x40)", styled);

        let revision = facade.read_styled_screen(pane_id).unwrap().revision;
        let mut unchanged = Vec::with_capacity(ROUNDS);
        for _ in 0..ROUNDS {
            let start = std::time::Instant::now();
            let frame = facade.read_render_frame(pane_id, Some(revision)).unwrap();
            unchanged.push(start.elapsed());
            assert!(!frame.full);
        }
        percentiles("read_render_frame (unchanged)", unchanged);

        // The in-process baseline the IPC numbers are judged against.
        let local = unterm_engine::next_core();
        let mut baseline = Vec::with_capacity(ROUNDS);
        for _ in 0..ROUNDS {
            let start = std::time::Instant::now();
            let snapshot = local.read_styled_screen(pane_id).unwrap();
            baseline.push(start.elapsed());
            assert_eq!(snapshot.cols, 120);
        }
        percentiles("read_styled_screen (in-process baseline)", baseline);

        facade.destroy_session(pane_id).unwrap();
        let mut owner = CoreClient::connect(endpoint, "bench-token").unwrap();
        let _: Response<serde_json::Value> = owner.request("core.shutdown").unwrap();
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
