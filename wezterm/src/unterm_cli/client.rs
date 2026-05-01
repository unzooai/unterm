//! Minimal JSON-RPC 2.0 client over TCP for the Unterm MCP server.
//!
//! See `wezterm-gui/src/mcp/server.rs` for the server side. The wire format
//! is line-delimited JSON-RPC 2.0; the first message must be `auth.login`
//! with the UUID stored at `~/.unterm/server.json` (preferred) or the
//! legacy `~/.unterm/auth_token` (fallback for older Unterm builds).

use anyhow::{anyhow, Context, Result};
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::time::Duration;

const MCP_HOST: &str = "127.0.0.1";
const LEGACY_MCP_PORT: u16 = 19876;

const NOT_RUNNING_HINT: &str =
    "unterm GUI is not running — open Unterm.app to start the MCP server, or run 'unterm start' first";

pub struct McpClient {
    reader: BufReader<TcpStream>,
    writer: TcpStream,
    next_id: u64,
}

impl McpClient {
    /// Read the auth token + MCP port from `~/.unterm/server.json` (with
    /// fallback to the legacy `~/.unterm/auth_token` + 19876), dial the
    /// MCP server, and complete the `auth.login` handshake.
    pub fn connect() -> Result<Self> {
        let info = ServerEndpoint::resolve()?;

        let stream = TcpStream::connect_timeout(
            &format!("{}:{}", MCP_HOST, info.port)
                .parse::<std::net::SocketAddr>()
                .expect("static addr"),
            Duration::from_secs(2),
        )
        .map_err(|_| anyhow!("{}", NOT_RUNNING_HINT))?;

        // Generous read timeout for slow ops (recording stop, screenshot, etc.).
        stream.set_read_timeout(Some(Duration::from_secs(30))).ok();
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

        let resp = client
            .call("auth.login", json!({ "token": info.token }))
            .context("MCP auth.login")?;
        if resp.get("status").and_then(|v| v.as_str()) != Some("ok") {
            return Err(anyhow!("MCP auth.login rejected: {}", resp));
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
        let n = self
            .reader
            .read_line(&mut buf)
            .map_err(|e| anyhow!("MCP read failed ({}); {}", e, NOT_RUNNING_HINT))?;
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
}

impl ServerEndpoint {
    pub fn resolve() -> Result<Self> {
        let dir = unterm_dir()?;

        // Prefer server.json
        let server_json = dir.join("server.json");
        if let Ok(raw) = std::fs::read_to_string(&server_json) {
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
                let http_port = info
                    .get("http_port")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u16;
                if !token.is_empty() && port != 0 {
                    return Ok(Self {
                        token,
                        port,
                        http_port,
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
        })
    }
}

fn unterm_dir() -> Result<PathBuf> {
    Ok(dirs_next::home_dir()
        .ok_or_else(|| anyhow!("could not resolve home directory"))?
        .join(".unterm"))
}
