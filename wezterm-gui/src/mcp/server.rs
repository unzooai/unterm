//! TCP server for MCP JSON-RPC protocol.
//!
//! Listens on 127.0.0.1:19876, authenticates clients via UUID token,
//! and dispatches requests to the handler module.

use super::handler::McpHandler;
use anyhow::Result;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::thread;

const MCP_PORT: u16 = 19876;
const MCP_BIND: &str = "127.0.0.1";

/// Start the MCP server in a background thread.
/// Returns the auth token for client connections.
pub fn start_mcp_server() -> String {
    let auth_token = uuid::Uuid::new_v4().to_string();

    // Save token to ~/.unterm/auth_token
    if let Err(e) = save_auth_token(&auth_token) {
        log::warn!("Failed to save MCP auth token: {}", e);
    }

    let token = auth_token.clone();
    thread::Builder::new()
        .name("mcp-server".into())
        .spawn(move || {
            if let Err(e) = run_server(&token) {
                log::error!("MCP server error: {}", e);
            }
        })
        .expect("Failed to spawn MCP server thread");

    log::info!("MCP server starting on {}:{}", MCP_BIND, MCP_PORT);
    auth_token
}

fn save_auth_token(token: &str) -> Result<()> {
    let dir = dirs_next::home_dir().unwrap_or_default().join(".unterm");
    fs::create_dir_all(&dir)?;
    fs::write(dir.join("auth_token"), token)?;
    Ok(())
}

fn run_server(auth_token: &str) -> Result<()> {
    let listener = TcpListener::bind(format!("{}:{}", MCP_BIND, MCP_PORT))?;
    log::info!("MCP server listening on {}:{}", MCP_BIND, MCP_PORT);

    let handler = Arc::new(McpHandler::new());

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let handler = Arc::clone(&handler);
                let token = auth_token.to_string();
                thread::Builder::new()
                    .name("mcp-client".into())
                    .spawn(move || {
                        if let Err(e) = handle_client(stream, &token, &handler) {
                            log::debug!("MCP client disconnected: {}", e);
                        }
                    })
                    .ok();
            }
            Err(e) => {
                log::warn!("MCP accept error: {}", e);
            }
        }
    }
    Ok(())
}

fn handle_client(stream: TcpStream, auth_token: &str, handler: &McpHandler) -> Result<()> {
    stream.set_nodelay(true)?;
    let peer = stream.peer_addr()?;
    log::info!("MCP client connected: {}", peer);

    let reader = BufReader::new(stream.try_clone()?);
    let mut writer = stream;
    let mut authenticated = false;

    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        let request: serde_json::Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(e) => {
                let error_resp = make_error_response(
                    serde_json::Value::Null,
                    -32700,
                    &format!("Parse error: {}", e),
                );
                write_response(&mut writer, &error_resp)?;
                continue;
            }
        };

        let id = request
            .get("id")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        let method = request.get("method").and_then(|m| m.as_str()).unwrap_or("");
        let params = request
            .get("params")
            .cloned()
            .unwrap_or(serde_json::json!({}));

        // Auth check
        if !authenticated {
            if method == "auth.login" {
                let client_token = params.get("token").and_then(|t| t.as_str()).unwrap_or("");
                if client_token == auth_token {
                    authenticated = true;
                    let resp = make_success_response(id, serde_json::json!({"status": "ok"}));
                    write_response(&mut writer, &resp)?;
                } else {
                    let resp = make_error_response(id, -32001, "Invalid auth token");
                    write_response(&mut writer, &resp)?;
                }
                continue;
            } else {
                let resp =
                    make_error_response(id, -32002, "Not authenticated. Call auth.login first");
                write_response(&mut writer, &resp)?;
                continue;
            }
        }

        // Dispatch to handler
        let result = handler.handle(method, &params);
        let resp = match result {
            Ok(value) => make_success_response(id, value),
            Err(e) => make_error_response(id, -32603, &e.to_string()),
        };
        write_response(&mut writer, &resp)?;
    }

    log::info!("MCP client disconnected: {}", peer);
    Ok(())
}

fn write_response(writer: &mut impl Write, resp: &serde_json::Value) -> Result<()> {
    let mut data = serde_json::to_string(resp)?;
    data.push('\n');
    writer.write_all(data.as_bytes())?;
    writer.flush()?;
    Ok(())
}

fn make_success_response(id: serde_json::Value, result: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result,
    })
}

fn make_error_response(id: serde_json::Value, code: i32, message: &str) -> serde_json::Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": code,
            "message": message,
        },
    })
}
