//! Multi-instance discovery for Unterm.
//!
//! Each running Unterm process owns one **instance** with a NATO-phonetic
//! name (alpha, bravo, charlie, …). On launch the instance writes its
//! metadata (ports, auth token, pid, started_at, version, platform) to
//! `~/.unterm/instances/<name>.json`. AI agents that drive multiple
//! Unterm windows enumerate that directory to discover all live
//! instances and pick which one to talk to.
//!
//! For backward compat with single-instance agents, we also maintain
//! `~/.unterm/server.json` and `~/.unterm/active.json` which mirror the
//! "active" instance (the most recently launched one whose ancestor is
//! still alive). Per the design lock-in on 2026-05-02, active.json is
//! updated only when the previous active dies — not on every focus
//! event — to keep disk IO minimal.
//!
//! Two servers (MCP JSON-RPC, HTTP web settings) cooperate within one
//! process: the MCP server starts first and seeds the instance file
//! with `mcp_port + auth_token`; the HTTP server then updates
//! `http_port` in place. Within-process writes are serialized via
//! `file_lock()`. Across-process races (two instances claiming the
//! same NATO name simultaneously) are handled with O_EXCL atomic
//! creation — see `claim_instance_name`.

use anyhow::{Context, Result};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write as _;
use std::net::TcpListener;
use std::path::{Path, PathBuf};

pub const MCP_PREFERRED_PORT: u16 = 19876;
pub const HTTP_PREFERRED_PORT: u16 = 19877;
pub const PORT_RETRY_LIMIT: u16 = 5;
pub const SERVER_BIND: &str = "127.0.0.1";

/// NATO phonetic alphabet — 26 single-word names. Choice locked
/// 2026-05-02: easier to pronounce than Crockford Base32 IDs and
/// AI agents handle them right. When all 26 are simultaneously taken
/// we append a digit (alpha2, bravo2, …); see `claim_instance_name`.
pub const NATO_NAMES: &[&str] = &[
    "alpha", "bravo", "charlie", "delta", "echo", "foxtrot", "golf", "hotel", "india", "juliet",
    "kilo", "lima", "mike", "november", "oscar", "papa", "quebec", "romeo", "sierra", "tango",
    "uniform", "victor", "whiskey", "xray", "yankee", "zulu",
];

/// On-disk metadata for one Unterm instance. Lives at
/// `~/.unterm/instances/<id>.json`. Both port fields can be 0 briefly
/// during startup before both servers have bound.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct InstanceInfo {
    pub id: String,
    pub mcp_port: u16,
    pub http_port: u16,
    pub auth_token: String,
    pub pid: u32,
    pub started_at: String,
    /// User-overridable display label. None = use auto-derived
    /// `Unterm — <id> — <project>` title; Some(str) = preserve user override.
    #[serde(default)]
    pub title: Option<String>,
    /// Last-seen cwd of the active pane. Refreshed periodically
    /// by the foreground update loop. Best-effort; agents can also
    /// query it live via `session.list`.
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub platform: String,
}

/// Compat alias: legacy server.json schema. The current process always
/// writes the *full* InstanceInfo into server.json (extra fields are
/// ignored by older deserializers), so older agents that only read
/// {mcp_port, http_port, auth_token, pid, started_at} keep working.
pub type ServerInfo = InstanceInfo;

fn unterm_dir() -> PathBuf {
    dirs_next::home_dir().unwrap_or_default().join(".unterm")
}

fn instances_dir() -> PathBuf {
    unterm_dir().join("instances")
}

fn instance_file(id: &str) -> PathBuf {
    instances_dir().join(format!("{}.json", id))
}

fn server_info_path() -> PathBuf {
    unterm_dir().join("server.json")
}

fn active_pointer_path() -> PathBuf {
    unterm_dir().join("active.json")
}

fn auth_token_path() -> PathBuf {
    unterm_dir().join("auth_token")
}

/// Coarse mutex serializing instance/active/server file writes within
/// this process. Cross-process atomicity handled by the O_EXCL claim
/// in `claim_instance_name` plus tmp+rename writes elsewhere.
fn file_lock() -> &'static Mutex<()> {
    static LOCK: std::sync::OnceLock<Mutex<()>> = std::sync::OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

/// This process's instance ID, set once at startup by `write_initial`.
fn current_id() -> &'static Mutex<Option<String>> {
    static ID: std::sync::OnceLock<Mutex<Option<String>>> = std::sync::OnceLock::new();
    ID.get_or_init(|| Mutex::new(None))
}

pub fn current_instance_id() -> Option<String> {
    current_id().lock().clone()
}

/// Try to bind to `preferred`, then `preferred+1 .. preferred+PORT_RETRY_LIMIT`.
/// Falls back to OS-assigned port (`port=0`) on persistent failure.
/// Returns the listener and the actually-bound port.
pub fn bind_with_fallback(preferred: u16) -> Result<(TcpListener, u16)> {
    for offset in 0..=PORT_RETRY_LIMIT {
        let port = preferred.saturating_add(offset);
        match TcpListener::bind((SERVER_BIND, port)) {
            Ok(listener) => {
                let port = listener.local_addr().map(|a| a.port()).unwrap_or(port);
                return Ok((listener, port));
            }
            Err(e) => {
                log::debug!("{}:{} bind failed ({}); trying next", SERVER_BIND, port, e);
            }
        }
    }
    let listener =
        TcpListener::bind((SERVER_BIND, 0u16)).context("OS-assigned port also failed")?;
    let port = listener.local_addr()?.port();
    Ok((listener, port))
}

/// Cross-platform "is this PID alive?" check. Used to clean up stale
/// instance files left behind by crashed processes. Best-effort: if we
/// can't tell, assume alive (preferring false-positives over deleting
/// a healthy peer's file).
pub fn pid_alive(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }
    #[cfg(unix)]
    {
        // kill(pid, 0) returns 0 if process exists and we can signal.
        // ESRCH = no such process. EPERM = exists but we can't signal,
        // which still means it's running. Only ESRCH = dead.
        //
        // Read errno portably via std::io::Error::last_os_error so this
        // works on both macOS (libc::__error) and Linux (__errno_location)
        // without #[cfg(target_os)] forks.
        let rc = unsafe { libc::kill(pid as libc::pid_t, 0) };
        if rc == 0 {
            return true;
        }
        !matches!(
            std::io::Error::last_os_error().raw_os_error(),
            Some(e) if e == libc::ESRCH
        )
    }
    #[cfg(windows)]
    unsafe {
        use winapi::shared::minwindef::FALSE;
        use winapi::um::handleapi::CloseHandle;
        use winapi::um::processthreadsapi::{GetExitCodeProcess, OpenProcess};
        use winapi::um::winnt::PROCESS_QUERY_LIMITED_INFORMATION;
        let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, FALSE, pid);
        if h.is_null() {
            return false;
        }
        let mut code: u32 = 0;
        let ok = GetExitCodeProcess(h, &mut code) != 0;
        CloseHandle(h);
        // STILL_ACTIVE (259) means the process hasn't exited.
        ok && code == 259
    }
}

/// Scan `instances/`, parse each `*.json`, drop entries whose PID is
/// no longer alive (and delete those files), return the survivors.
fn live_instances_locked() -> Vec<InstanceInfo> {
    let dir = instances_dir();
    let read = match fs::read_dir(&dir) {
        Ok(r) => r,
        Err(_) => return vec![], // dir doesn't exist yet
    };
    let mut alive = Vec::new();
    for entry in read.flatten() {
        let path = entry.path();
        if path.extension().map(|e| e != "json").unwrap_or(true) {
            continue;
        }
        let Ok(content) = fs::read_to_string(&path) else {
            continue;
        };
        let Ok(info): std::result::Result<InstanceInfo, _> = serde_json::from_str(&content) else {
            // Corrupt file: leave it alone (could be a partial write
            // by a peer; deleting would be racy).
            continue;
        };
        if info.id.is_empty() {
            continue;
        }
        if pid_alive(info.pid) {
            alive.push(info);
        } else {
            // Crashed/quit: remove stale file. Best-effort.
            let _ = fs::remove_file(&path);
        }
    }
    alive
}

/// Public: list all live instances. Used by the MCP `instance.list`
/// method and any agent that wants to enumerate.
pub fn list_live_instances() -> Vec<InstanceInfo> {
    let _g = file_lock().lock();
    live_instances_locked()
}

/// Pick the lowest-NATO name not currently taken by a live instance,
/// then try to atomically claim it via O_EXCL create. If two instances
/// race for the same name, the second one's create_new fails and we
/// retry with the next name. Falls back to NATO+digit (alpha2, bravo2…)
/// if all 26 base names are simultaneously taken.
fn claim_instance_name() -> Result<String> {
    let dir = instances_dir();
    fs::create_dir_all(&dir).ok();

    let alive = live_instances_locked();
    let taken: std::collections::HashSet<String> =
        alive.iter().map(|i| i.id.clone()).collect();

    // First pass: NATO base names.
    for name in NATO_NAMES {
        if taken.contains(*name) {
            continue;
        }
        if try_o_excl_create(&instance_file(name)).is_ok() {
            return Ok(name.to_string());
        }
    }
    // Second pass: NATO+digit. Cap at 99 to bound the loop —
    // if you're really running 2,574 Untermsself something else is wrong.
    for n in 2..=99 {
        for name in NATO_NAMES {
            let candidate = format!("{}{}", name, n);
            if taken.contains(&candidate) {
                continue;
            }
            if try_o_excl_create(&instance_file(&candidate)).is_ok() {
                return Ok(candidate);
            }
        }
    }
    anyhow::bail!("no free instance name available (capped at NATO×99)")
}

fn try_o_excl_create(path: &Path) -> std::io::Result<()> {
    let mut f = OpenOptions::new()
        .write(true)
        .create_new(true) // O_EXCL: fail if already exists
        .open(path)?;
    // Touch the file with an empty {} so concurrent peers see it as
    // taken. Real metadata gets written by `write_initial` immediately
    // after this returns.
    f.write_all(b"{}")?;
    Ok(())
}

/// Read the active instance pointer (`~/.unterm/active.json`).
/// If the pointer is missing, points to a dead instance, or fails to
/// parse, returns the most recently started live instance, or default.
pub fn read() -> InstanceInfo {
    let _g = file_lock().lock();
    // Prefer active.json if it points to a live instance.
    if let Ok(content) = fs::read_to_string(active_pointer_path()) {
        if let Ok(info) = serde_json::from_str::<InstanceInfo>(&content) {
            if pid_alive(info.pid) {
                return info;
            }
        }
    }
    // Fall back to scanning instances/, picking the most recent live one.
    let mut alive = live_instances_locked();
    alive.sort_by(|a, b| b.started_at.cmp(&a.started_at));
    if let Some(info) = alive.into_iter().next() {
        return info;
    }
    // Truly nothing alive: legacy server.json (might be from a previous run).
    fs::read_to_string(server_info_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

/// Read this current process's instance file (or default if not yet written).
pub fn read_current() -> InstanceInfo {
    let id = match current_instance_id() {
        Some(id) => id,
        None => return InstanceInfo::default(),
    };
    let _g = file_lock().lock();
    fs::read_to_string(instance_file(&id))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

/// Initial write at MCP server startup: claim a NATO name, generate
/// a token, stamp pid + started_at + mcp_port + version + platform.
/// Also seeds active.json (if there's no live active currently) and
/// keeps server.json + auth_token in sync for legacy clients.
pub fn write_initial(mcp_port: u16) -> Result<InstanceInfo> {
    let _g = file_lock().lock();
    fs::create_dir_all(unterm_dir())?;
    fs::create_dir_all(instances_dir())?;

    let id = claim_instance_name().context("could not claim NATO instance name")?;
    *current_id().lock() = Some(id.clone());

    let info = InstanceInfo {
        id: id.clone(),
        mcp_port,
        http_port: 0,
        auth_token: uuid::Uuid::new_v4().to_string(),
        pid: std::process::id(),
        started_at: chrono::Local::now().to_rfc3339(),
        title: None,
        cwd: None,
        version: env!("CARGO_PKG_VERSION").to_string(),
        platform: std::env::consts::OS.to_string(),
    };
    write_atomic(&instance_file(&id), &info)?;

    claim_compat_files_if_needed(&info)?;
    Ok(info)
}

/// Update this instance's file to record the HTTP server's port.
/// Called after the HTTP server successfully binds. Also updates
/// active.json + server.json if we're the active instance.
pub fn set_http_port(port: u16) -> Result<InstanceInfo> {
    let id = match current_instance_id() {
        Some(id) => id,
        None => return Ok(InstanceInfo::default()),
    };
    let _g = file_lock().lock();
    let mut info: InstanceInfo = fs::read_to_string(instance_file(&id))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    info.http_port = port;
    write_atomic(&instance_file(&id), &info)?;
    claim_compat_files_if_needed(&info)?;
    Ok(info)
}

/// Update this instance's `cwd` field. Called periodically by the
/// foreground update loop. Cheap (one file write); skipped if the
/// value hasn't changed since last write.
pub fn set_cwd(cwd: Option<String>) -> Result<()> {
    let id = match current_instance_id() {
        Some(id) => id,
        None => return Ok(()),
    };
    let _g = file_lock().lock();
    let mut info: InstanceInfo = match fs::read_to_string(instance_file(&id)) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
        Err(_) => return Ok(()),
    };
    if info.cwd == cwd {
        claim_compat_files_if_needed(&info)?;
        return Ok(()); // no change
    }
    info.cwd = cwd;
    write_atomic(&instance_file(&id), &info)?;
    claim_compat_files_if_needed(&info)?;
    Ok(())
}

/// Update this instance's user-overridable title. None clears the
/// override (auto-title resumes); Some(str) pins a custom title.
pub fn set_title(title: Option<String>) -> Result<()> {
    let id = match current_instance_id() {
        Some(id) => id,
        None => return Ok(()),
    };
    let _g = file_lock().lock();
    let mut info: InstanceInfo = match fs::read_to_string(instance_file(&id)) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
        Err(_) => return Ok(()),
    };
    info.title = title;
    write_atomic(&instance_file(&id), &info)?;
    Ok(())
}

/// Best-effort cleanup on graceful exit: delete this instance's file
/// and, if we were the active pointer, drop active.json so the next
/// launching peer can claim it. Called from drop / signal / atexit.
///
/// Dead-code-allow is intentional: callers that want explicit cleanup
/// invoke this; on hard crashes, the next-launching instance handles
/// stale files via its PID-liveness scan in `live_instances_locked()`.
/// Both paths are correct, so wiring shutdown into every code-path
/// isn't load-bearing.
#[allow(dead_code)]
pub fn shutdown() {
    let Some(id) = current_instance_id() else {
        return;
    };
    let _g = file_lock().lock();
    let _ = fs::remove_file(instance_file(&id));
    // Was I the active? If so, hand off — clear the pointer and pick
    // the next live instance, if any. Single-instance agents will
    // re-resolve on next read.
    if let Ok(content) = fs::read_to_string(active_pointer_path()) {
        if let Ok(active) = serde_json::from_str::<InstanceInfo>(&content) {
            if active.id == id {
                let _ = fs::remove_file(active_pointer_path());
                let _ = fs::remove_file(server_info_path());
                let mut alive = live_instances_locked();
                alive.sort_by(|a, b| b.started_at.cmp(&a.started_at));
                if let Some(next) = alive.into_iter().next() {
                    let _ = write_atomic(&active_pointer_path(), &next);
                    let _ = write_atomic(&server_info_path(), &next);
                    let _ = write_legacy_token(&next.auth_token);
                }
            }
        }
    }
}

fn write_atomic<T: Serialize>(path: &Path, info: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let body = serde_json::to_string_pretty(info)?;
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, body)?;
    fs::rename(&tmp, path)?;
    Ok(())
}

fn write_legacy_token(token: &str) -> Result<()> {
    let path = auth_token_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, token)?;
    Ok(())
}

fn claim_compat_files_if_needed(info: &InstanceInfo) -> Result<()> {
    let should_claim_active = match fs::read_to_string(active_pointer_path()) {
        Ok(content) => match serde_json::from_str::<InstanceInfo>(&content) {
            Ok(prev) => prev.id == info.id || !pid_alive(prev.pid),
            Err(_) => true,
        },
        Err(_) => true,
    };

    if should_claim_active {
        write_atomic(&active_pointer_path(), info)?;
        // Mirror to legacy server.json for old CLI / agent clients.
        write_atomic(&server_info_path(), info)?;
        write_legacy_token(&info.auth_token)?;
    }

    Ok(())
}
