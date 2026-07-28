//! `unterm-cli server ...` — MCP server health and capability probes.

use super::client::McpClient;
use super::output::{print_json, print_kv};
use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use serde_json::{json, Value};

#[derive(Debug, Parser, Clone)]
pub struct ServerCommand {
    #[command(subcommand)]
    pub sub: ServerSubCommand,
}

#[derive(Debug, Subcommand, Clone)]
pub enum ServerSubCommand {
    /// Print server version/protocol metadata.
    Info,
    /// Print liveness/readiness details.
    Health,
    /// Print the MCP namespace -> method capability map.
    Capabilities,
    /// Run the built-in MCP self-test suite.
    Selftest {
        /// Optional pane/session id to include pane-specific checks.
        #[arg(long)]
        session_id: Option<String>,
    },
}

pub fn run(cmd: ServerCommand, json_out: bool) -> Result<()> {
    let mut client = McpClient::connect()?;
    match cmd.sub {
        ServerSubCommand::Info => {
            let result = client.call("server.info", json!({}))?;
            if json_out {
                print_json(&result);
            } else {
                print_kv(
                    "Name",
                    result.get("name").and_then(Value::as_str).unwrap_or(""),
                );
                print_kv(
                    "Version",
                    result.get("version").and_then(Value::as_str).unwrap_or(""),
                );
                print_kv(
                    "Engine",
                    result.get("engine").and_then(Value::as_str).unwrap_or(""),
                );
                print_kv(
                    "Protocol",
                    result.get("protocol").and_then(Value::as_str).unwrap_or(""),
                );
            }
        }
        ServerSubCommand::Health => {
            let result = client.call("server.health", json!({}))?;
            if json_out {
                print_json(&result);
            } else {
                print_kv(
                    "Status",
                    result.get("status").and_then(Value::as_str).unwrap_or(""),
                );
                if let Some(pane_count) = result.pointer("/mux/pane_count").and_then(Value::as_u64)
                {
                    print_kv("Panes", &pane_count.to_string());
                }
                if let Some(term) = result.pointer("/terminal/term").and_then(Value::as_str) {
                    print_kv("Term", term);
                }
                if let Some(color_scheme) = result
                    .pointer("/terminal/color_scheme")
                    .and_then(Value::as_str)
                {
                    print_kv("Color scheme", color_scheme);
                }
            }
        }
        ServerSubCommand::Capabilities => {
            let result = client.call("server.capabilities", json!({}))?;
            if json_out {
                print_json(&result);
            } else {
                let object = result
                    .as_object()
                    .ok_or_else(|| anyhow!("server.capabilities did not return an object"))?;
                for (namespace, methods) in object {
                    let count = methods.as_array().map(|items| items.len()).unwrap_or(0);
                    print_kv(namespace, &count.to_string());
                }
            }
        }
        ServerSubCommand::Selftest { session_id } => {
            let mut params = json!({});
            if let Some(session_id) = session_id {
                params["session_id"] = json!(session_id);
            }
            let result = client.call("selftest.run", params)?;
            if json_out {
                print_json(&result);
            } else {
                println!(
                    "{}",
                    result.get("ok").and_then(Value::as_bool).unwrap_or(false)
                );
                let checks = result
                    .get("checks")
                    .and_then(Value::as_array)
                    .ok_or_else(|| anyhow!("selftest.run did not return `checks`: {}", result))?;
                println!("{:<30} OK", "CHECK");
                for check in checks {
                    let name = check.get("name").and_then(Value::as_str).unwrap_or("");
                    let ok = check.get("ok").and_then(Value::as_bool).unwrap_or(false);
                    println!("{:<30} {}", name, ok);
                }
            }
        }
    }
    Ok(())
}
