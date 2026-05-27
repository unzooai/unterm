//! `unterm-cli scrollback` — dump the full scrollback (history + viewport)
//! of a pane as text. AI-friendly companion to `screenshot`: when you want
//! to give an LLM the entire terminal output, text is strictly better than
//! a rendered "long screenshot" (no OCR, no font fidelity loss).

use super::client::McpClient;
use super::output::print_json;
use anyhow::{anyhow, Result};
use clap::Args;
use serde_json::{json, Value};
use std::path::PathBuf;

#[derive(Debug, Args, Clone)]
pub struct ScrollbackCommand {
    /// Target pane (UUID or pane index). If omitted, uses the active pane
    /// of the connected Unterm instance — same default as the other CLI
    /// commands.
    #[arg(long = "pane-id")]
    pub pane_id: Option<String>,

    /// Preserve ANSI color/style escapes in the output. Off by default
    /// because most LLM hand-off use cases want plain text.
    #[arg(long = "escapes")]
    pub escapes: bool,

    /// Optional output file. If omitted, writes to stdout.
    #[arg(short = 'o', long = "output")]
    pub output: Option<PathBuf>,

    /// Absolute starting StableRowIndex (negatives reach back into the
    /// scrollback). Default: scrollback_top (start of history).
    #[arg(long = "start-line", allow_hyphen_values = true)]
    pub start_line: Option<i64>,

    /// Absolute ending StableRowIndex, exclusive. Default: bottom of
    /// viewport.
    #[arg(long = "end-line", allow_hyphen_values = true)]
    pub end_line: Option<i64>,
}

pub fn run(cmd: ScrollbackCommand, json_out: bool) -> Result<()> {
    let mut params = json!({ "escapes": cmd.escapes });
    if let Some(id) = cmd.pane_id.as_deref() {
        params["pane_id"] = Value::String(id.to_string());
    }
    if let Some(n) = cmd.start_line {
        params["start_line"] = Value::Number(n.into());
    }
    if let Some(n) = cmd.end_line {
        params["end_line"] = Value::Number(n.into());
    }

    let mut client = McpClient::connect()?;
    let result = client.call("screen.scrollback_text", params)?;

    if json_out {
        print_json(&result);
        return Ok(());
    }

    let text = result
        .get("text")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("screen.scrollback_text did not return `text`: {}", result))?;

    if let Some(dest) = cmd.output.as_ref() {
        if let Some(parent) = dest.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).ok();
            }
        }
        std::fs::write(dest, text)?;
        println!("{}", dest.display());
    } else {
        print!("{}", text);
        if !text.ends_with('\n') {
            println!();
        }
    }
    Ok(())
}
