//! `unterm-cli proxy ...` subcommands.

use super::client::McpClient;
use super::i18n;
use super::output::{print_json, print_kv};
use anyhow::Result;
use clap::Parser;
use serde_json::json;

#[derive(Debug, Parser, Clone)]
pub struct ProxyCommand {
    #[command(subcommand)]
    pub sub: ProxySubCommand,
}

#[derive(Debug, Parser, Clone)]
pub enum ProxySubCommand {
    /// Show current proxy status.
    Status,
    /// List configured proxy nodes.
    Nodes,
    /// Switch to the named proxy node.
    Switch {
        /// Proxy node name (must match a `proxy.nodes` entry).
        name: String,
    },
    /// Disable the proxy.
    Disable,
    /// Print proxy environment variables (HTTP_PROXY etc.).
    Env,
    /// Endpoint-level auto-rotation: fail over to the fastest live node in a
    /// pool when the active one dies. With no flags, prints current settings.
    Rotation {
        /// Turn auto-rotation on.
        #[arg(long, conflicts_with = "off")]
        on: bool,
        /// Turn auto-rotation off.
        #[arg(long)]
        off: bool,
        /// Node names to rotate among (repeat or comma-separate). Sets the pool.
        #[arg(long, value_delimiter = ',')]
        pool: Vec<String>,
        /// Health-check interval in seconds (min 5).
        #[arg(long)]
        interval: Option<u64>,
    },
}

pub fn run(cmd: ProxyCommand, json_out: bool) -> Result<()> {
    let mut client = McpClient::connect()?;
    match cmd.sub {
        ProxySubCommand::Status => {
            let result = client.call("proxy.status", json!({}))?;
            if json_out {
                print_json(&result);
            } else {
                let enabled = result
                    .get("enabled")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let mode = result.get("mode").and_then(|v| v.as_str()).unwrap_or("off");
                let none = i18n::t("cli.proxy.value.none");
                let http = result
                    .get("http_proxy")
                    .and_then(|v| v.as_str())
                    .map(str::to_string)
                    .unwrap_or_else(|| none.clone());
                let socks = result
                    .get("socks_proxy")
                    .and_then(|v| v.as_str())
                    .map(str::to_string)
                    .unwrap_or_else(|| none.clone());
                let current = result
                    .get("current_node")
                    .and_then(|v| v.as_str())
                    .map(str::to_string)
                    .unwrap_or_else(|| none.clone());
                let count = result
                    .get("node_count")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                let no_proxy = result
                    .get("no_proxy")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let on_off = if enabled {
                    i18n::t("cli.proxy.on")
                } else {
                    i18n::t("cli.proxy.off")
                };
                println!("{}: {}", i18n::t("cli.proxy.label.proxy"), on_off);
                println!("{}:  {}", i18n::t("cli.proxy.label.mode"), mode);
                println!("{}:  {}", i18n::t("cli.proxy.label.http"), http);
                println!("{}: {}", i18n::t("cli.proxy.label.socks"), socks);
                println!("{}: {}", i18n::t("cli.proxy.label.current_node"), current);
                println!("{}: {}", i18n::t("cli.proxy.label.node_count"), count);
                if !no_proxy.is_empty() {
                    println!("{}: {}", i18n::t("cli.proxy.label.no_proxy"), no_proxy);
                }
            }
        }
        ProxySubCommand::Nodes => {
            let result = client.call("proxy.nodes", json!({}))?;
            if json_out {
                print_json(&result);
            } else {
                let current = result
                    .get("current_node")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let nodes = result
                    .get("nodes")
                    .and_then(|v| v.as_array())
                    .cloned()
                    .unwrap_or_default();
                if nodes.is_empty() {
                    println!("{}", i18n::t("cli.proxy.no_nodes"));
                } else {
                    println!(
                        "{:<3} {:<24} {}",
                        "",
                        i18n::t("cli.proxy.head.name"),
                        i18n::t("cli.proxy.head.url")
                    );
                    for node in &nodes {
                        let name = node.get("name").and_then(|v| v.as_str()).unwrap_or("");
                        let url = node.get("url").and_then(|v| v.as_str()).unwrap_or("");
                        let marker = if name == current { "*" } else { " " };
                        println!("{:<3} {:<24} {}", marker, name, url);
                    }
                }
            }
        }
        ProxySubCommand::Switch { name } => {
            // Server expects {node_name: ...}, even though the CLI surface uses {name}.
            let result = client.call("proxy.switch", json!({ "node_name": name }))?;
            if json_out {
                print_json(&result);
            } else {
                print_kv(&i18n::t("cli.proxy.switched"), "true");
                if let Some(node) = result.get("current_node").and_then(|v| v.as_str()) {
                    print_kv(&i18n::t("cli.proxy.label.current_node"), node);
                }
                if let Some(http) = result.get("http_proxy").and_then(|v| v.as_str()) {
                    print_kv(&i18n::t("cli.proxy.label.http"), http);
                }
            }
        }
        ProxySubCommand::Disable => {
            let result = client.call("proxy.disable", json!({}))?;
            if json_out {
                print_json(&result);
            } else {
                println!("{}", i18n::t("cli.proxy.disabled"));
            }
        }
        ProxySubCommand::Env => {
            let result = client.call("proxy.env", json!({}))?;
            if json_out {
                print_json(&result);
            } else {
                let enabled = result
                    .get("enabled")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                if !enabled {
                    println!("{}", i18n::t("cli.proxy.disabled_comment"));
                    return Ok(());
                }
                if let Some(env) = result.get("env").and_then(|v| v.as_object()) {
                    for (k, v) in env {
                        if let Some(s) = v.as_str() {
                            println!("export {}={}", k, shell_quote(s));
                        }
                    }
                }
            }
        }
        ProxySubCommand::Rotation {
            on,
            off,
            pool,
            interval,
        } => {
            // Build the params map from only the flags the user passed, so a
            // bare `proxy rotation` is a pure read.
            let mut params = serde_json::Map::new();
            if on {
                params.insert("enabled".into(), json!(true));
            } else if off {
                params.insert("enabled".into(), json!(false));
            }
            if !pool.is_empty() {
                params.insert("pool".into(), json!(pool));
            }
            if let Some(iv) = interval {
                params.insert("interval_secs".into(), json!(iv));
            }
            let result = client.call("proxy.rotation", serde_json::Value::Object(params))?;
            if json_out {
                print_json(&result);
            } else {
                let enabled = result
                    .get("enabled")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                print_kv("rotation", if enabled { "on" } else { "off" });
                if let Some(p) = result.get("pool").and_then(|v| v.as_array()) {
                    let names: Vec<String> = p
                        .iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect();
                    print_kv("pool", &names.join(", "));
                }
                if let Some(iv) = result.get("interval_secs").and_then(|v| v.as_u64()) {
                    print_kv("interval_secs", &iv.to_string());
                }
                if let Some(cur) = result.get("current_node").and_then(|v| v.as_str()) {
                    print_kv("current_node", cur);
                }
            }
        }
    }
    Ok(())
}

/// Single-quote a value for safe shell `export` output.
fn shell_quote(s: &str) -> String {
    if s.chars()
        .all(|c| c.is_ascii_alphanumeric() || ":/.,-_=".contains(c))
    {
        s.to_string()
    } else {
        format!("'{}'", s.replace('\'', "'\\''"))
    }
}
