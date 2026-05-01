//! Shared server-discovery glue: cooperatively-written `~/.unterm/server.json`
//! and a small port-binding helper.
//!
//! Both `mcp` (line-delimited JSON-RPC) and `web_settings` (HTTP/1.1) need to
//! advertise their bound ports plus a shared auth token to local clients
//! (the `unterm` CLI, the SPA running in the browser). The MCP server starts
//! first, generates a UUID auth token and writes a partial `server.json`;
//! the HTTP-Settings server then binds and updates the same file in place
//! with its own `http_port`. The legacy `~/.unterm/auth_token` file is also
//! kept up to date so older CLI builds continue to work.

use anyhow::{Context, Result};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::fs;
use std::net::TcpListener;
use std::path::PathBuf;

pub const MCP_PREFERRED_PORT: u16 = 19876;
pub const HTTP_PREFERRED_PORT: u16 = 19877;
pub const PORT_RETRY_LIMIT: u16 = 5;
pub const SERVER_BIND: &str = "127.0.0.1";

/// On-disk shape of `~/.unterm/server.json`. Both `mcp_port` and `http_port`
/// are required after both servers come up; before then they may be 0.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ServerInfo {
    pub mcp_port: u16,
    pub http_port: u16,
    pub auth_token: String,
    pub pid: u32,
    pub started_at: String,
}

fn server_info_path() -> PathBuf {
    dirs_next::home_dir()
        .unwrap_or_default()
        .join(".unterm")
        .join("server.json")
}

fn auth_token_path() -> PathBuf {
    dirs_next::home_dir()
        .unwrap_or_default()
        .join(".unterm")
        .join("auth_token")
}

/// Coarse mutex serializing `server.json` reads/writes within this process.
/// The MCP server thread and the HTTP server thread both update the file at
/// startup, so we want their writes to be a strict happens-before chain.
fn file_lock() -> &'static Mutex<()> {
    static LOCK: std::sync::OnceLock<Mutex<()>> = std::sync::OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

/// Try to bind to `preferred`, then `preferred+1 .. preferred+PORT_RETRY_LIMIT`.
/// Falls back to OS-assigned port (`port=0`) on persistent failure.
/// Returns the listener and the actually-bound port.
pub fn bind_with_fallback(preferred: u16) -> Result<(TcpListener, u16)> {
    for offset in 0..=PORT_RETRY_LIMIT {
        let port = preferred.saturating_add(offset);
        match TcpListener::bind((SERVER_BIND, port)) {
            Ok(listener) => {
                let port = listener
                    .local_addr()
                    .map(|a| a.port())
                    .unwrap_or(port);
                return Ok((listener, port));
            }
            Err(e) => {
                log::debug!(
                    "{}:{} bind failed ({}); trying next",
                    SERVER_BIND,
                    port,
                    e
                );
            }
        }
    }
    let listener = TcpListener::bind((SERVER_BIND, 0u16))
        .context("OS-assigned port also failed")?;
    let port = listener.local_addr()?.port();
    Ok((listener, port))
}

/// Read the on-disk server info; returns default if absent or unparseable.
pub fn read() -> ServerInfo {
    let _g = file_lock().lock();
    fs::read_to_string(server_info_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

/// Initial write by the MCP server: stamps token+pid+started_at and the MCP
/// port. `http_port` stays 0 until the HTTP server later calls
/// `set_http_port`. Also mirrors the token to the legacy path.
pub fn write_initial(mcp_port: u16) -> Result<ServerInfo> {
    let info = ServerInfo {
        mcp_port,
        http_port: 0,
        auth_token: uuid::Uuid::new_v4().to_string(),
        pid: std::process::id(),
        started_at: chrono::Local::now().to_rfc3339(),
    };
    write_atomic(&info)?;
    write_legacy_token(&info.auth_token)?;
    Ok(info)
}

/// Update the file in place to record the HTTP server's port. Called after
/// the HTTP server successfully binds.
pub fn set_http_port(port: u16) -> Result<ServerInfo> {
    let mut info = read();
    info.http_port = port;
    write_atomic(&info)?;
    Ok(info)
}

fn write_atomic(info: &ServerInfo) -> Result<()> {
    let _g = file_lock().lock();
    let path = server_info_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let body = serde_json::to_string_pretty(info)?;
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, body)?;
    fs::rename(&tmp, &path)?;
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
