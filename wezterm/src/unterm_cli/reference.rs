//! `unterm-cli reference` — print the full surface (MCP methods + CLI
//! subcommands + live keybindings) by calling the `meta.surface` MCP
//! method on the connected Unterm instance.
//!
//! Pretty-prints by default. Use `--json` for machine-readable output.
//! Use `--section mcp|cli|keys` to limit the output.

use super::client::McpClient;
use super::output::print_json;
use anyhow::{anyhow, Result};
use clap::{Args, ValueEnum};
use serde_json::Value;

#[derive(Debug, Args, Clone)]
pub struct ReferenceCommand {
    /// Restrict the output to one section. Default: all three.
    #[arg(long = "section", value_enum)]
    pub section: Option<Section>,

    /// Filter by substring match (case-insensitive). Applies to method/
    /// command names, summaries, and keybinding actions.
    #[arg(short = 'f', long = "filter")]
    pub filter: Option<String>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum Section {
    Mcp,
    Cli,
    Keys,
}

pub fn run(cmd: ReferenceCommand, json_out: bool) -> Result<()> {
    let mut client = McpClient::connect()?;
    let result = client.call("meta.surface", serde_json::json!({}))?;

    if json_out {
        let scoped = scope_payload(&result, cmd.section, cmd.filter.as_deref());
        print_json(&scoped);
        return Ok(());
    }

    let filter = cmd
        .filter
        .as_deref()
        .map(|s| s.to_ascii_lowercase());
    let filter = filter.as_deref();

    let want_mcp = matches!(cmd.section, None | Some(Section::Mcp));
    let want_cli = matches!(cmd.section, None | Some(Section::Cli));
    let want_keys = matches!(cmd.section, None | Some(Section::Keys));

    if want_mcp {
        print_mcp(&result, filter)?;
    }
    if want_cli {
        if want_mcp {
            println!();
        }
        print_cli(&result, filter)?;
    }
    if want_keys {
        if want_mcp || want_cli {
            println!();
        }
        print_keys(&result, filter)?;
    }
    Ok(())
}

fn scope_payload(result: &Value, section: Option<Section>, filter: Option<&str>) -> Value {
    let mut out = serde_json::Map::new();
    out.insert(
        "version".to_string(),
        result.get("version").cloned().unwrap_or(Value::Null),
    );
    let fl = filter.map(|s| s.to_ascii_lowercase());
    let fl = fl.as_deref();
    if matches!(section, None | Some(Section::Mcp)) {
        out.insert(
            "mcp_methods".to_string(),
            filter_array(result.get("mcp_methods"), fl, |v| {
                let name = v.get("name").and_then(|x| x.as_str()).unwrap_or("");
                let summary = v.get("summary").and_then(|x| x.as_str()).unwrap_or("");
                format!("{} {}", name, summary)
            }),
        );
    }
    if matches!(section, None | Some(Section::Cli)) {
        out.insert(
            "cli_commands".to_string(),
            filter_array(result.get("cli_commands"), fl, |v| {
                let name = v.get("name").and_then(|x| x.as_str()).unwrap_or("");
                let summary = v.get("summary").and_then(|x| x.as_str()).unwrap_or("");
                format!("{} {}", name, summary)
            }),
        );
    }
    if matches!(section, None | Some(Section::Keys)) {
        out.insert(
            "keybindings".to_string(),
            filter_array(result.get("keybindings"), fl, |v| {
                let key = v.get("key").and_then(|x| x.as_str()).unwrap_or("");
                let action = v.get("action").and_then(|x| x.as_str()).unwrap_or("");
                format!("{} {}", key, action)
            }),
        );
    }
    Value::Object(out)
}

fn filter_array<F: Fn(&Value) -> String>(
    arr: Option<&Value>,
    filter: Option<&str>,
    haystack_of: F,
) -> Value {
    let Some(Value::Array(items)) = arr else {
        return Value::Array(vec![]);
    };
    let Some(f) = filter else {
        return Value::Array(items.clone());
    };
    let f = f.to_ascii_lowercase();
    let filtered: Vec<Value> = items
        .iter()
        .filter(|v| haystack_of(v).to_ascii_lowercase().contains(&f))
        .cloned()
        .collect();
    Value::Array(filtered)
}

fn print_mcp(result: &Value, filter: Option<&str>) -> Result<()> {
    let methods = result
        .get("mcp_methods")
        .and_then(|v| v.as_array())
        .ok_or_else(|| anyhow!("meta.surface returned no mcp_methods array"))?;
    println!("MCP methods ({})", methods.len());
    println!("{}", "-".repeat(50));
    let mut shown = 0;
    for m in methods {
        let name = m.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let summary = m.get("summary").and_then(|v| v.as_str()).unwrap_or("");
        if let Some(f) = filter {
            if !format!("{} {}", name, summary)
                .to_ascii_lowercase()
                .contains(f)
            {
                continue;
            }
        }
        println!("  {:<32} {}", name, summary);
        shown += 1;
    }
    if filter.is_some() {
        println!("({} matched)", shown);
    }
    Ok(())
}

fn print_cli(result: &Value, filter: Option<&str>) -> Result<()> {
    let cmds = result
        .get("cli_commands")
        .and_then(|v| v.as_array())
        .ok_or_else(|| anyhow!("meta.surface returned no cli_commands array"))?;
    println!("CLI subcommands ({})", cmds.len());
    println!("{}", "-".repeat(50));
    let mut shown = 0;
    for c in cmds {
        let name = c.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let summary = c.get("summary").and_then(|v| v.as_str()).unwrap_or("");
        if let Some(f) = filter {
            if !format!("{} {}", name, summary)
                .to_ascii_lowercase()
                .contains(f)
            {
                continue;
            }
        }
        let subs = c
            .get("subcommands")
            .and_then(|v| v.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .unwrap_or_default();
        println!("  unterm-cli {:<18} {}", name, summary);
        if !subs.is_empty() {
            println!("    └── {}", subs);
        }
        shown += 1;
    }
    if filter.is_some() {
        println!("({} matched)", shown);
    }
    Ok(())
}

fn print_keys(result: &Value, filter: Option<&str>) -> Result<()> {
    let keys = result
        .get("keybindings")
        .and_then(|v| v.as_array())
        .ok_or_else(|| anyhow!("meta.surface returned no keybindings array"))?;
    println!("Keybindings ({})", keys.len());
    println!("{}", "-".repeat(50));
    let mut shown = 0;
    for k in keys {
        let table = k.get("table").and_then(|v| v.as_str()).unwrap_or("");
        let key = k.get("key").and_then(|v| v.as_str()).unwrap_or("");
        let mods = k.get("mods").and_then(|v| v.as_str()).unwrap_or("");
        let action = k.get("action").and_then(|v| v.as_str()).unwrap_or("");
        let pretty_keys = if mods.is_empty() || mods == "(empty)" {
            key.to_string()
        } else {
            format!("{} {}", mods, key)
        };
        if let Some(f) = filter {
            if !format!("{} {}", pretty_keys, action)
                .to_ascii_lowercase()
                .contains(f)
            {
                continue;
            }
        }
        let table_tag = if table == "default" {
            String::new()
        } else {
            format!("[{}] ", table)
        };
        println!("  {}{:<32} → {}", table_tag, pretty_keys, action);
        shown += 1;
    }
    if filter.is_some() {
        println!("({} matched)", shown);
    }
    Ok(())
}
