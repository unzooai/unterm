//! Hand a directory to the window that is already open.
//!
//! The Explorer context menu's "Open in Unterm tab": this binary is the
//! GUI-subsystem one, so it can run from a right-click without flashing a
//! console -- but the window the user means is usually not this process, it
//! is the one they already have. Forward the directory to the registered
//! instance as `session.create` and get out of the way; only when nobody is
//! there does the caller fall through to opening a window itself.
//!
//! A deliberately minimal MCP client: connect, `auth.login`, one create, one
//! focus. `unterm-cli` carries the full-featured one, but a binary cannot
//! depend on another binary, and pulling a shared client crate into
//! existence for four calls would be scaffolding for its own sake.

use anyhow::{anyhow, Context, Result};
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::time::Duration;

pub fn open_tab_in_live_window(cwd: &std::path::Path) -> Result<()> {
    let info = unterm_services::server_info::read();
    if info.mcp_port == 0 || info.auth_token.is_empty() {
        return Err(anyhow!("no instance is registered"));
    }
    if !unterm_services::server_info::pid_alive(info.pid) {
        return Err(anyhow!("the registered instance is no longer running"));
    }

    let stream = TcpStream::connect(("127.0.0.1", info.mcp_port))
        .context("connect to the running instance")?;
    stream.set_read_timeout(Some(Duration::from_secs(5))).ok();
    stream.set_write_timeout(Some(Duration::from_secs(5))).ok();
    let mut reader = BufReader::new(stream.try_clone().context("clone stream")?);
    let mut writer = stream;
    let mut next_id = 1u64;
    let mut call = |method: &str, params: serde_json::Value| -> Result<serde_json::Value> {
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": next_id,
            "method": method,
            "params": params,
        });
        next_id += 1;
        let mut line = serde_json::to_string(&request)?;
        line.push('\n');
        writer.write_all(line.as_bytes())?;
        writer.flush().ok();
        let mut buf = String::new();
        reader.read_line(&mut buf)?;
        let reply: serde_json::Value = serde_json::from_str(&buf)
            .with_context(|| format!("parse {method} reply"))?;
        if let Some(error) = reply.get("error") {
            return Err(anyhow!("{method} failed: {error}"));
        }
        Ok(reply.get("result").cloned().unwrap_or(serde_json::Value::Null))
    };

    let handshake = unterm_protocol::BuildHandshake::current(
        unterm_protocol::ProcessRole::Gui,
        std::process::id(),
        chrono::Utc::now().to_rfc3339(),
    );
    let login = call(
        "auth.login",
        serde_json::json!({ "token": info.auth_token, "client": handshake }),
    )?;
    if login.get("status").and_then(|status| status.as_str()) != Some("ok") {
        return Err(anyhow!("auth.login rejected: {login}"));
    }

    call(
        "session.create",
        serde_json::json!({ "cwd": cwd.to_string_lossy() }),
    )?;
    // Raising the window is a courtesy, not a condition: the tab exists
    // either way, and a platform that refuses to yield foreground focus
    // should not turn a successful open into a reported failure.
    let _ = call("instance.focus", serde_json::json!({}));
    Ok(())
}
