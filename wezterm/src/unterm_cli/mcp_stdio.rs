//! `unterm-cli mcp-stdio` — a Model Context Protocol *stdio* server that
//! bridges an MCP client (Claude Code, Codex CLI, Gemini CLI, …) to the
//! running Unterm instance's TCP JSON-RPC control server.
//!
//! Why this exists: Unterm's own control server speaks a custom line-
//! delimited JSON-RPC over a TCP socket with a bearer token — that's what
//! `unterm-cli` and the Web Settings UI talk to. AI agents, however, speak
//! the MCP *spec* (newline-delimited JSON-RPC over stdio, with the
//! `initialize` / `tools/list` / `tools/call` handshake). They can't dial
//! the TCP port directly. This bridge is the adapter: an agent spawns
//! `unterm-cli mcp-stdio` as a stdio MCP server, and we proxy every
//! `tools/call` to the TCP server, exposing the full Unterm surface
//! (screen.*, session.*, exec.*, capture.*, upload.*, …) as MCP tools.
//!
//! Agents are auto-wired to this command by the agent launcher (see
//! `unterm-agents`), which writes the right per-agent config
//! (`.mcp.json`, `~/.codex/config.toml`, etc.) pointing here.
//!
//! Protocol notes:
//!   * stdout carries ONLY newline-delimited JSON-RPC. All diagnostics go
//!     to stderr — a stray stdout write corrupts the MCP stream.
//!   * Requests (with `id`) get a response; notifications (no `id`) don't.
//!   * Tool inventory + param schemas come from the TCP server's
//!     `meta.surface`, so the MCP tool list never drifts from dispatch.

use super::client::McpClient;
use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use std::io::{BufRead, Write};

/// MCP protocol version we advertise in the initialize response. This is a
/// widely-supported revision; clients negotiate down if needed.
const PROTOCOL_VERSION: &str = "2024-11-05";

/// Friendly per-call error when the GUI isn't up. Returned with
/// `isError: true` so the agent can read it and tell the user.
const GUI_NOT_RUNNING: &str = "Unterm GUI is not running — open Unterm.app (or run \
`unterm start`), then retry. The tool surface is still listed so you know what \
will be available once it's up.";

pub fn run() -> Result<()> {
    // Connect to the running Unterm instance up front — but if the GUI isn't
    // running, come up anyway and keep speaking MCP: serve `initialize` and
    // `tools/list` from the static surface tables baked into this binary,
    // return a clean per-call error on `tools/call`, and lazily reconnect
    // when the GUI appears. Exiting here used to break (a) registry health
    // checks that introspect the server headlessly and (b) agents that start
    // before the terminal does.
    let mut client: Option<McpClient> = match McpClient::connect() {
        Ok(c) => Some(c),
        Err(e) => {
            eprintln!(
                "unterm mcp-stdio: Unterm control server not reachable ({e}); \
                 serving static tool list, will reconnect on demand"
            );
            None
        }
    };

    // Tool inventory: prefer the live server's meta.surface (never drifts
    // from dispatch); fall back to the compiled-in tables.
    let surface = client
        .as_mut()
        .and_then(|c| c.call("meta.surface", json!({})).ok())
        .unwrap_or_else(|| {
            json!({
                "mcp_methods": serde_json::to_value(unterm_agents::mcp_meta::MCP_METHODS)
                    .unwrap_or_else(|_| json!([]))
            })
        });
    let tools = build_tool_list(&surface);

    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();

    let mut line = String::new();
    loop {
        line.clear();
        let n = stdin.lock().read_line(&mut line)?;
        if n == 0 {
            break; // EOF — client closed stdin
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let req: Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(e) => {
                // Parse error — reply only if we can't even find an id.
                write_msg(
                    &mut stdout,
                    &json!({
                        "jsonrpc": "2.0",
                        "id": Value::Null,
                        "error": { "code": -32700, "message": format!("parse error: {e}") },
                    }),
                )?;
                continue;
            }
        };

        let method = req.get("method").and_then(|v| v.as_str()).unwrap_or("");
        let id = req.get("id").cloned();
        let is_notification = id.is_none();

        match method {
            "initialize" => {
                respond(
                    &mut stdout,
                    id,
                    json!({
                        "protocolVersion": PROTOCOL_VERSION,
                        "capabilities": { "tools": { "listChanged": false } },
                        "serverInfo": { "name": "unterm", "version": env!("CARGO_PKG_VERSION") },
                        // Injected into the connecting agent's context by most
                        // MCP clients — this is where we teach it what Unterm
                        // is and which tools drive it. Shared with the text
                        // `setup-ai` writes into context files, so the two
                        // discovery channels stay in sync.
                        "instructions": super::setup_ai::MCP_INSTRUCTIONS,
                    }),
                )?;
            }
            "notifications/initialized" | "initialized" => {
                // Notification — no response.
            }
            "ping" => {
                respond(&mut stdout, id, json!({}))?;
            }
            "tools/list" => {
                respond(&mut stdout, id, json!({ "tools": tools }))?;
            }
            "tools/call" => {
                let params = req.get("params").cloned().unwrap_or(Value::Null);
                let name = params
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let arguments = params
                    .get("arguments")
                    .cloned()
                    .unwrap_or_else(|| json!({}));
                // Lazy reconnect: the GUI may have started after us.
                if client.is_none() {
                    client = McpClient::connect().ok();
                }
                let Some(live) = client.as_mut() else {
                    respond(
                        &mut stdout,
                        id,
                        json!({
                            "content": [ { "type": "text", "text": GUI_NOT_RUNNING } ],
                            "isError": true,
                        }),
                    )?;
                    continue;
                };
                match live.call(&name, arguments) {
                    Ok(result) => {
                        let text = serde_json::to_string_pretty(&result)
                            .unwrap_or_else(|_| result.to_string());
                        respond(
                            &mut stdout,
                            id,
                            json!({
                                "content": [ { "type": "text", "text": text } ],
                                "isError": false,
                            }),
                        )?;
                    }
                    Err(e) => {
                        // MCP convention: tool execution failures are returned
                        // as a successful response with isError=true, so the
                        // model can read + react to the error text.
                        respond(
                            &mut stdout,
                            id,
                            json!({
                                "content": [ { "type": "text", "text": format!("{e}") } ],
                                "isError": true,
                            }),
                        )?;
                    }
                }
            }
            _ => {
                if !is_notification {
                    respond_err(
                        &mut stdout,
                        id,
                        -32601,
                        &format!("method not found: {method}"),
                    )?;
                }
            }
        }
    }

    Ok(())
}

/// Convert `meta.surface`'s mcp_methods array into MCP tool descriptors with
/// a JSON-Schema inputSchema derived from each method's param list.
fn build_tool_list(surface: &Value) -> Vec<Value> {
    let methods = surface
        .get("mcp_methods")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    methods
        .iter()
        .filter_map(|m| {
            let name = m.get("name").and_then(|v| v.as_str())?;
            // meta.surface is itself proxyable but pointless to expose as a
            // tool to the agent — it'd just be describing the toolset the
            // agent already received. Skip it.
            if name == "meta.surface" {
                return None;
            }
            let summary = m.get("summary").and_then(|v| v.as_str()).unwrap_or("");
            let mut properties = serde_json::Map::new();
            let mut required: Vec<Value> = Vec::new();
            if let Some(params) = m.get("params").and_then(|v| v.as_array()) {
                for p in params {
                    let pname = match p.get("name").and_then(|v| v.as_str()) {
                        Some(n) => n,
                        None => continue,
                    };
                    let kind = p.get("kind").and_then(|v| v.as_str()).unwrap_or("string");
                    let psummary = p.get("summary").and_then(|v| v.as_str()).unwrap_or("");
                    properties.insert(
                        pname.to_string(),
                        json!({ "type": json_schema_type(kind), "description": psummary }),
                    );
                    if p.get("required").and_then(|v| v.as_bool()).unwrap_or(false) {
                        required.push(Value::String(pname.to_string()));
                    }
                }
            }
            Some(json!({
                "name": name,
                "description": summary,
                "inputSchema": {
                    "type": "object",
                    "properties": Value::Object(properties),
                    "required": required,
                },
            }))
        })
        .collect()
}

/// Map our param `kind` strings to JSON-Schema `type` values. Unions like
/// `string|int` degrade to `string` (the agent can pass either; the TCP
/// server coerces). Unknown kinds default to `string`.
fn json_schema_type(kind: &str) -> &'static str {
    match kind {
        "int" => "integer",
        "bool" => "boolean",
        "number" | "float" => "number",
        _ => "string",
    }
}

fn respond(out: &mut impl Write, id: Option<Value>, result: Value) -> Result<()> {
    write_msg(
        out,
        &json!({ "jsonrpc": "2.0", "id": id.unwrap_or(Value::Null), "result": result }),
    )
}

fn respond_err(
    out: &mut impl Write,
    id: Option<Value>,
    code: i64,
    message: &str,
) -> Result<()> {
    write_msg(
        out,
        &json!({
            "jsonrpc": "2.0",
            "id": id.unwrap_or(Value::Null),
            "error": { "code": code, "message": message },
        }),
    )
}

fn write_msg(out: &mut impl Write, msg: &Value) -> Result<()> {
    let mut s = serde_json::to_string(msg).map_err(|e| anyhow!("serialize MCP message: {e}"))?;
    s.push('\n');
    out.write_all(s.as_bytes())?;
    out.flush()?;
    Ok(())
}
