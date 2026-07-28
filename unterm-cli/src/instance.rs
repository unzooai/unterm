//! `unterm-cli instance ...` — inspect and label live Unterm GUI instances.

use super::client::McpClient;
use super::output::{print_json, print_kv};
use anyhow::Result;
use clap::{Parser, Subcommand};
use serde_json::json;

#[derive(Debug, Parser, Clone)]
pub struct InstanceCommand {
    #[command(subcommand)]
    pub sub: InstanceSubCommand,
}

#[derive(Debug, Subcommand, Clone)]
pub enum InstanceSubCommand {
    /// List live Unterm windows/instances on this machine.
    List,
    /// Show metadata for the target instance.
    Info,
    /// Pin or clear the target instance's display title.
    SetTitle {
        /// Title to show in instance lists. Omit with --clear.
        title: Option<String>,
        /// Clear the custom title and resume automatic titles.
        #[arg(long)]
        clear: bool,
    },
    /// Bring the target instance to the foreground when supported.
    Focus,
}

pub fn run(cmd: InstanceCommand, json_out: bool) -> Result<()> {
    let mut client = McpClient::connect()?;
    match cmd.sub {
        InstanceSubCommand::List => {
            let result = client.call("instance.list", json!({}))?;
            if json_out {
                print_json(&result);
            } else {
                let instances = result
                    .get("instances")
                    .and_then(|v| v.as_array())
                    .cloned()
                    .unwrap_or_default();
                if instances.is_empty() {
                    println!("No live Unterm instances.");
                } else {
                    println!(
                        "{:<9} {:<8} {:<7} {:<7} {:<24} {}",
                        "ID", "PID", "MCP", "HTTP", "CWD", "TITLE"
                    );
                    for instance in &instances {
                        println!(
                            "{:<9} {:<8} {:<7} {:<7} {:<24} {}",
                            instance.get("id").and_then(|v| v.as_str()).unwrap_or(""),
                            instance.get("pid").and_then(|v| v.as_u64()).unwrap_or(0),
                            instance
                                .get("mcp_port")
                                .and_then(|v| v.as_u64())
                                .unwrap_or(0),
                            instance
                                .get("http_port")
                                .and_then(|v| v.as_u64())
                                .unwrap_or(0),
                            instance.get("cwd").and_then(|v| v.as_str()).unwrap_or(""),
                            instance.get("title").and_then(|v| v.as_str()).unwrap_or("")
                        );
                    }
                }
            }
        }
        InstanceSubCommand::Info => {
            let result = client.call("instance.info", json!({}))?;
            if json_out {
                print_json(&result);
            } else {
                print_instance_info(&result);
            }
        }
        InstanceSubCommand::SetTitle { title, clear } => {
            let params = if clear {
                json!({ "title": null })
            } else {
                json!({ "title": title.unwrap_or_default() })
            };
            let result = client.call("instance.set_title", params)?;
            if json_out {
                print_json(&result);
            } else {
                let title = result
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or("(automatic)");
                print_kv("Title", title);
            }
        }
        InstanceSubCommand::Focus => {
            let result = client.call("instance.focus", json!({}))?;
            if json_out {
                print_json(&result);
            } else {
                print_kv(
                    "Focused",
                    if result.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
                        "true"
                    } else {
                        "false"
                    },
                );
                if let Some(note) = result.get("note").and_then(|v| v.as_str()) {
                    print_kv("Note", note);
                }
            }
        }
    }
    Ok(())
}

fn print_instance_info(result: &serde_json::Value) {
    for key in [
        "id",
        "pid",
        "mcp_port",
        "http_port",
        "started_at",
        "title",
        "cwd",
        "version",
        "platform",
    ] {
        let value = result.get(key).map(format_value).unwrap_or_default();
        print_kv(key, &value);
    }
}

fn format_value(value: &serde_json::Value) -> String {
    value
        .as_str()
        .map(str::to_string)
        .unwrap_or_else(|| value.to_string())
}
