//! Minimal JSON-RPC 2.0 client over TCP for the Unterm MCP server.
//!
//! See `wezterm-gui/src/mcp/server.rs` for the server side. The wire format
//! is line-delimited JSON-RPC 2.0; the first message must be `auth.login`
//! with the UUID stored at `~/.unterm/server.json` (preferred) or the
//! legacy `~/.unterm/auth_token` (fallback for older Unterm builds).

use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use serde_json::{json, Value};
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::Duration;
use unterm_protocol::{BuildHandshake, ProcessRole};

const MCP_HOST: &str = "127.0.0.1";
const LEGACY_MCP_PORT: u16 = 19876;

const NOT_RUNNING_HINT: &str =
    "unterm GUI is not running — open Unterm.app to start the MCP server, or run 'unterm start' first";
static TARGET_INSTANCE: OnceLock<String> = OnceLock::new();

pub struct McpClient {
    reader: BufReader<TcpStream>,
    writer: TcpStream,
    next_id: u64,
}

pub fn set_target_instance(id: Option<&str>) {
    if let Some(id) = id.map(str::trim).filter(|id| !id.is_empty()) {
        let _ = TARGET_INSTANCE.set(id.to_string());
    }
}

/// Windows reports a socket read timeout as `TimedOut`, but some stacks
/// surface it as `WouldBlock`; both mean the same thing here.
fn is_timeout(err: &std::io::Error) -> bool {
    matches!(
        err.kind(),
        std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
    )
}

impl McpClient {
    /// Read the auth token + MCP port from `~/.unterm/server.json` (with
    /// fallback to the legacy `~/.unterm/auth_token` + 19876), dial the
    /// MCP server, and complete the `auth.login` handshake.
    pub fn connect() -> Result<Self> {
        Self::connect_as(ProcessRole::Cli)
    }

    /// Connect while identifying the caller's process role. The MCP bridge
    /// uses this path so the server can distinguish it from an interactive
    /// CLI during drain and future supervisor accounting.
    pub fn connect_as(process_role: ProcessRole) -> Result<Self> {
        let info = ServerEndpoint::resolve()?;
        validate_peer_identity(info.identity.as_ref())?;

        let stream = TcpStream::connect_timeout(
            &format!("{}:{}", MCP_HOST, info.port)
                .parse::<std::net::SocketAddr>()
                .expect("static addr"),
            Duration::from_secs(2),
        )
        .map_err(|_| anyhow!("{}", NOT_RUNNING_HINT))?;

        // Generous read timeout for slow ops (recording stop, screenshot,
        // etc.) -- and it must outlast the server's confirmation wait, not
        // merely equal it. A PTY write parks the server thread on a
        // confirmation banner for `mcp_confirmation_timeout_ms`; when both
        // sides ran the same 30s the client gave up in the same instant the
        // server decided, so the real verdict ("the user refused", "the
        // question timed out") was never delivered and the CLI invented a
        // connection failure instead. The margin makes the server's answer
        // always win the race.
        let confirm_wait =
            Duration::from_millis(unterm_services::settings::current().mcp_confirmation_timeout_ms);
        stream
            .set_read_timeout(Some(
                confirm_wait
                    .saturating_add(Duration::from_secs(15))
                    .max(Duration::from_secs(30)),
            ))
            .ok();
        stream.set_write_timeout(Some(Duration::from_secs(10))).ok();
        stream.set_nodelay(true).ok();

        let writer = stream
            .try_clone()
            .context("cloning MCP TCP stream for writer")?;
        let reader = BufReader::new(stream);

        let mut client = McpClient {
            reader,
            writer,
            next_id: 1,
        };

        let caller = BuildHandshake::current(
            process_role,
            std::process::id(),
            chrono::Utc::now().to_rfc3339(),
        );
        let resp = client
            .call(
                "auth.login",
                json!({ "token": info.token, "client": caller }),
            )
            .context("MCP auth.login")?;
        if resp.get("status").and_then(|v| v.as_str()) != Some("ok") {
            return Err(anyhow!("MCP auth.login rejected: {}", resp));
        }
        // Validate the process actually reached, not only the registry record
        // used to discover it. A replacement can occur between file read and
        // TCP connect. Pre-M0 servers have no `build` object and remain usable
        // when their legacy registry version already matched.
        if let Ok(live) = client.call("server.info", json!({})) {
            if let Some(build) = live.get("build") {
                if let Ok(identity) = serde_json::from_value::<BuildHandshake>(build.clone()) {
                    validate_peer_identity(Some(&identity))?;
                }
            }
        }
        Ok(client)
    }

    /// Send a JSON-RPC request and return the `result` field on success.
    pub fn call(&mut self, method: &str, params: Value) -> Result<Value> {
        let id = self.next_id;
        self.next_id += 1;
        let req = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });
        let mut line = serde_json::to_string(&req)?;
        line.push('\n');
        self.writer
            .write_all(line.as_bytes())
            .map_err(|e| anyhow!("MCP write failed ({}); {}", e, NOT_RUNNING_HINT))?;
        self.writer.flush().ok();

        let mut buf = String::new();
        let n = self.reader.read_line(&mut buf).map_err(|e| {
            // A read timeout means the server took the request and never
            // answered -- the opposite situation from "nothing is
            // listening", and pointing at the wrong one sends people to
            // restart an app that is running fine. The connect above
            // already proved something is there.
            if is_timeout(&e) {
                anyhow!(
                    "MCP {} timed out waiting for a reply. Unterm is running but did not answer; \
                     if this was a command for a pane, check the Unterm window for a confirmation \
                     asking you to approve it.",
                    method
                )
            } else {
                anyhow!("MCP read failed ({}); {}", e, NOT_RUNNING_HINT)
            }
        })?;
        if n == 0 {
            return Err(anyhow!(
                "MCP server closed the connection unexpectedly; {}",
                NOT_RUNNING_HINT
            ));
        }

        let resp: Value = serde_json::from_str(buf.trim())
            .with_context(|| format!("parsing MCP response for {}: {:?}", method, buf))?;

        if let Some(err) = resp.get("error") {
            let code = err.get("code").and_then(|v| v.as_i64()).unwrap_or(-1);
            let message = err
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            return Err(anyhow!("MCP {} failed [{}]: {}", method, code, message));
        }
        Ok(resp.get("result").cloned().unwrap_or(Value::Null))
    }
}

/// Where to find the MCP server. Pulled from `~/.unterm/server.json` if
/// present, with fallback to the legacy `~/.unterm/auth_token` + 19876.
pub struct ServerEndpoint {
    pub token: String,
    pub port: u16,
    pub http_port: u16,
    pub identity: Option<BuildHandshake>,
}

#[derive(Debug, Clone, Deserialize)]
struct InstanceRecord {
    pub mcp_port: u16,
    #[serde(default)]
    pub http_port: u16,
    pub auth_token: String,
    pub pid: u32,
    #[serde(default)]
    pub started_at: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub product_version: String,
    #[serde(default)]
    pub build_commit: String,
    #[serde(default)]
    pub protocol_version: String,
    #[serde(default)]
    pub data_schema_version: u32,
    #[serde(default)]
    pub process_role: ProcessRole,
}

impl InstanceRecord {
    fn build_handshake(&self) -> Option<BuildHandshake> {
        let product_version = if self.product_version.is_empty() {
            self.version.clone()
        } else {
            self.product_version.clone()
        };
        if product_version.is_empty() {
            return None;
        }
        Some(BuildHandshake {
            product_version,
            build_commit: if self.build_commit.is_empty() {
                "unknown".into()
            } else {
                self.build_commit.clone()
            },
            protocol_version: if self.protocol_version.is_empty() {
                "legacy".into()
            } else {
                self.protocol_version.clone()
            },
            data_schema_version: self.data_schema_version,
            process_role: self.process_role,
            pid: self.pid,
            started_at: self.started_at.clone(),
        })
    }
}

fn validate_peer_identity(identity: Option<&BuildHandshake>) -> Result<()> {
    let Some(identity) = identity else {
        return Ok(());
    };
    let compatibility = identity.compatibility();
    if compatibility.is_usable() {
        return Ok(());
    }
    let code = compatibility
        .error_code()
        .unwrap_or("protocol_incompatible");
    Err(anyhow!(
        "{code}: running Unterm {} (protocol {}, schema {}) is incompatible with bridge {} (protocol {}, schema {}); drain this bridge and let the MCP client restart it from the installed unterm-cli",
        identity.product_version,
        identity.protocol_version,
        identity.data_schema_version,
        unterm_protocol::PRODUCT_VERSION,
        unterm_protocol::PROTOCOL_VERSION,
        unterm_protocol::DATA_SCHEMA_VERSION,
    ))
}

impl ServerEndpoint {
    pub fn resolve() -> Result<Self> {
        let dir = unterm_dir()?;

        if let Some(instance_id) = requested_instance_id() {
            let path = dir.join("instances").join(format!("{instance_id}.json"));
            let Some(info) = read_live_record(&path)? else {
                return Err(anyhow!(
                    "Unterm instance '{}' was not found or is not running. Use MCP `instance.list`, or inspect {}.",
                    instance_id,
                    dir.join("instances").display()
                ));
            };
            let identity = info.build_handshake();
            return Ok(Self {
                token: info.auth_token,
                port: info.mcp_port,
                http_port: info.http_port,
                identity,
            });
        }

        if let Some(info) = resolve_live_instance(&dir)? {
            let identity = info.build_handshake();
            return Ok(Self {
                token: info.auth_token,
                port: info.mcp_port,
                http_port: info.http_port,
                identity,
            });
        }

        // No live GUI instance — but sessions never lived in the GUI
        // anyway. The Core process serves the same MCP surface and
        // outlives every window; fall through to it before declaring
        // the product absent.
        if let Some(endpoint) = resolve_core_endpoint() {
            return Ok(endpoint);
        }

        // Prefer server.json as the legacy fallback for older builds.
        let server_json = dir.join("server.json");
        if let Ok(raw) = fs::read_to_string(&server_json) {
            if let Ok(info) = serde_json::from_str::<Value>(&raw) {
                let token = info
                    .get("auth_token")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let port = info
                    .get("mcp_port")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(LEGACY_MCP_PORT as u64) as u16;
                let http_port = info.get("http_port").and_then(|v| v.as_u64()).unwrap_or(0) as u16;
                if !token.is_empty() && port != 0 {
                    return Ok(Self {
                        token,
                        port,
                        http_port,
                        identity: identity_from_value(&info),
                    });
                }
            }
        }

        // Fallback to legacy auth_token
        let token_path = dir.join("auth_token");
        if !token_path.exists() {
            return Err(anyhow!("{}", NOT_RUNNING_HINT));
        }
        let token = std::fs::read_to_string(&token_path)
            .with_context(|| format!("reading {}", token_path.display()))?
            .trim()
            .to_string();
        if token.is_empty() {
            return Err(anyhow!("{}", NOT_RUNNING_HINT));
        }
        Ok(Self {
            token,
            port: LEGACY_MCP_PORT,
            http_port: 0,
            identity: None,
        })
    }
}

/// The Core process's discovery record: `core.json` under
/// `%LOCALAPPDATA%\Unterm` (or `UNTERM_STATE_DIR`), written by
/// `unterm-core`. Its `mcp_port` is the agent surface that keeps
/// working with no GUI alive; the token doubles for MCP auth.
fn resolve_core_endpoint() -> Option<ServerEndpoint> {
    let dir = match std::env::var_os("UNTERM_STATE_DIR") {
        Some(path) if !path.is_empty() => PathBuf::from(path),
        Some(_) => return None,
        None => dirs_next::data_local_dir()?.join("Unterm"),
    };
    let raw = fs::read_to_string(dir.join("core.json")).ok()?;
    let info: Value = serde_json::from_str(&raw).ok()?;
    let pid = info.get("pid")?.as_u64()? as u32;
    let port = info.get("mcp_port")?.as_u64()? as u16;
    let token = info.get("token")?.as_str()?.to_string();
    if pid == 0 || !pid_alive(pid) || port == 0 || token.is_empty() {
        return None;
    }
    Some(ServerEndpoint {
        token,
        port,
        http_port: 0,
        identity: identity_from_value(&info),
    })
}

fn identity_from_value(value: &Value) -> Option<BuildHandshake> {
    let version = value
        .get("product_version")
        .or_else(|| value.get("version"))?
        .as_str()?
        .to_string();
    Some(BuildHandshake {
        product_version: version,
        build_commit: value
            .get("build_commit")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string(),
        protocol_version: value
            .get("protocol_version")
            .and_then(Value::as_str)
            .unwrap_or("legacy")
            .to_string(),
        data_schema_version: value
            .get("data_schema_version")
            .and_then(Value::as_u64)
            .unwrap_or(0) as u32,
        process_role: ProcessRole::Gui,
        pid: value.get("pid").and_then(Value::as_u64).unwrap_or(0) as u32,
        started_at: value
            .get("started_at")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
    })
}

pub fn http_post_json(path: &str, body: Value) -> Result<Value> {
    let info = ServerEndpoint::resolve()?;
    if info.http_port == 0 {
        return Err(anyhow!("Unterm HTTP settings server is not available"));
    }

    let addr = format!("{}:{}", MCP_HOST, info.http_port)
        .parse::<std::net::SocketAddr>()
        .expect("static addr");
    let mut stream = TcpStream::connect_timeout(&addr, Duration::from_secs(2)).map_err(|e| {
        anyhow!(
            "HTTP settings server unavailable ({}); {}",
            e,
            NOT_RUNNING_HINT
        )
    })?;
    stream.set_read_timeout(Some(Duration::from_secs(10))).ok();
    stream.set_write_timeout(Some(Duration::from_secs(10))).ok();
    stream.set_nodelay(true).ok();

    let body = serde_json::to_vec(&body)?;
    let request = format!(
        "POST {} HTTP/1.1\r\nHost: {}:{}\r\nAuthorization: Bearer {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        path,
        MCP_HOST,
        info.http_port,
        info.token,
        body.len()
    );
    stream.write_all(request.as_bytes())?;
    stream.write_all(&body)?;
    stream.flush().ok();

    let mut response = Vec::new();
    stream.read_to_end(&mut response)?;
    let split = response
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .ok_or_else(|| anyhow!("malformed HTTP response from Unterm settings server"))?;
    let header = String::from_utf8_lossy(&response[..split]);
    let body = &response[split + 4..];
    let status = header
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|code| code.parse::<u16>().ok())
        .ok_or_else(|| anyhow!("malformed HTTP status from Unterm settings server"))?;

    let value = if body.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice::<Value>(body).unwrap_or(Value::Null)
    };
    if !(200..300).contains(&status) {
        let message = value
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("HTTP settings request failed");
        return Err(anyhow!("{}: {}", status, message));
    }
    Ok(value)
}

/// Resolve the instance id whose GUI pid matches — used by `agent
/// signal` to route hook events to the instance that owns the calling
/// pane. Hooks inherit `WEZTERM_UNIX_SOCKET=…/gui-sock-<pid>` from the
/// pane's shell, and pid is the one instance-unique key in that env.
pub fn instance_for_pid(pid: u32) -> Option<String> {
    let dir = unterm_dir().ok()?.join("instances");
    for entry in fs::read_dir(&dir).ok()?.flatten() {
        let path = entry.path();
        if path.extension().map(|e| e != "json").unwrap_or(true) {
            continue;
        }
        let Ok(raw) = fs::read_to_string(&path) else {
            continue;
        };
        let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) else {
            continue;
        };
        if v.get("pid").and_then(|p| p.as_u64()) == Some(pid as u64) {
            if let Some(id) = v.get("id").and_then(|i| i.as_str()) {
                return Some(id.to_string());
            }
        }
    }
    None
}

fn requested_instance_id() -> Option<String> {
    TARGET_INSTANCE
        .get()
        .cloned()
        .or_else(|| std::env::var("UNTERM_INSTANCE").ok())
        .map(|id| id.trim().to_string())
        .filter(|id| !id.is_empty())
}

fn resolve_live_instance(dir: &PathBuf) -> Result<Option<InstanceRecord>> {
    if let Some(info) = read_live_record(&dir.join("active.json"))? {
        return Ok(Some(info));
    }

    let instances_dir = dir.join("instances");
    let mut live = Vec::new();
    let entries = match fs::read_dir(&instances_dir) {
        Ok(entries) => entries,
        Err(_) => return Ok(None),
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().map(|ext| ext != "json").unwrap_or(true) {
            continue;
        }
        if let Some(info) = read_live_record(&path)? {
            live.push(info);
        }
    }

    live.sort_by(|a, b| b.started_at.cmp(&a.started_at));
    Ok(live.into_iter().next())
}

fn read_live_record(path: &PathBuf) -> Result<Option<InstanceRecord>> {
    let raw = match fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(_) => return Ok(None),
    };
    let info: InstanceRecord = match serde_json::from_str(&raw) {
        Ok(info) => info,
        Err(_) => return Ok(None),
    };
    if info.pid == 0 || !pid_alive(info.pid) || info.mcp_port == 0 || info.auth_token.is_empty() {
        return Ok(None);
    }
    Ok(Some(info))
}

#[cfg(unix)]
fn pid_alive(pid: u32) -> bool {
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
fn pid_alive(pid: u32) -> bool {
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
        ok && code == 259
    }
}

fn unterm_dir() -> Result<PathBuf> {
    unterm_protocol::state_dir().ok_or_else(|| anyhow!("could not resolve home directory"))
}

#[cfg(test)]
mod compatibility_tests {
    use super::*;

    #[test]
    fn same_version_legacy_instance_is_allowed() {
        let record: InstanceRecord = serde_json::from_value(json!({
            "mcp_port": 19876,
            "http_port": 19877,
            "auth_token": "secret",
            "pid": 42,
            "started_at": "now",
            "version": unterm_protocol::PRODUCT_VERSION,
        }))
        .unwrap();
        let identity = record.build_handshake().unwrap();
        assert_eq!(
            identity.compatibility(),
            unterm_protocol::Compatibility::Legacy
        );
        validate_peer_identity(Some(&identity)).unwrap();
    }

    #[test]
    fn stale_product_version_has_a_machine_readable_error() {
        let mut identity = BuildHandshake::current(ProcessRole::Gui, 42, "now");
        identity.product_version = "0.57.4".into();
        let error = validate_peer_identity(Some(&identity))
            .unwrap_err()
            .to_string();
        assert!(error.starts_with("product_version_mismatch:"), "{error}");
        assert!(error.contains("drain this bridge"), "{error}");
    }

    #[test]
    fn newer_schema_is_rejected_before_connecting() {
        let mut identity = BuildHandshake::current(ProcessRole::Gui, 42, "now");
        identity.data_schema_version = unterm_protocol::DATA_SCHEMA_VERSION + 1;
        let error = validate_peer_identity(Some(&identity))
            .unwrap_err()
            .to_string();
        assert!(error.starts_with("data_schema_incompatible:"), "{error}");
    }
}
