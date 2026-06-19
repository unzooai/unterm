//! `unterm-cli session ...` — operate on a single live pane (record / export).

use super::client::McpClient;
use super::i18n;
use super::output::{print_json, print_kv};
use anyhow::{anyhow, Result};
use clap::Parser;
use serde_json::{json, Value};
use std::io::Read;
use std::path::PathBuf;

#[derive(Debug, Parser, Clone)]
pub struct SessionCommand {
    #[command(subcommand)]
    pub sub: SessionSubCommand,
}

#[derive(Debug, Parser, Clone)]
pub enum SessionSubCommand {
    /// List live panes (sessions).
    List,
    /// Spawn a new tab.
    Create {
        /// Working directory for the new tab.
        #[arg(long)]
        cwd: Option<PathBuf>,
        /// Identity profile to apply to this tab's environment.
        #[arg(long)]
        profile: Option<String>,
        /// Shell command to run. Use `--` before commands with flags.
        #[arg(
            value_name = "COMMAND",
            trailing_var_arg = true,
            allow_hyphen_values = true
        )]
        command: Vec<String>,
    },
    /// Split an existing pane and spawn a shell in the new split.
    Split {
        /// Source pane id (defaults to the first live pane).
        #[arg(long)]
        id: Option<u64>,
        /// Split direction: right, left, down, or up.
        #[arg(long, default_value = "right")]
        direction: String,
        /// Size of the new pane as a percentage.
        #[arg(long, default_value_t = 50)]
        size_percent: u8,
        /// Working directory for the new split.
        #[arg(long)]
        cwd: Option<PathBuf>,
    },
    /// Focus a pane and its containing tab.
    Focus {
        /// Target pane id (defaults to the first live pane).
        #[arg(long)]
        id: Option<u64>,
    },
    /// Resize a pane's PTY.
    Resize {
        /// Target pane id (defaults to the first live pane).
        #[arg(long)]
        id: Option<u64>,
        #[arg(long)]
        cols: u64,
        #[arg(long)]
        rows: u64,
    },
    /// Close a pane. Requires an explicit pane id.
    Destroy {
        /// Target pane id.
        #[arg(long)]
        id: u64,
    },
    /// Manage block recording for a pane.
    Record(RecordCommand),
    /// Export a pane's block log as Markdown.
    Export {
        /// Target pane id (defaults to the first live pane).
        #[arg(long)]
        id: Option<u64>,
        /// Optional output file. If omitted, the Unterm-side path is printed.
        #[arg(short = 'o', long = "output")]
        output: Option<PathBuf>,
    },
    /// Write text into a pane via MCP `session.input`.
    Input {
        /// Target pane id (defaults to the first live pane).
        #[arg(long)]
        id: Option<u64>,
        /// Read additional input from stdin.
        #[arg(long)]
        stdin: bool,
        /// Append carriage return, matching a real Enter keypress.
        #[arg(long)]
        enter: bool,
        /// Text to write. Use `--` before text that starts with a dash.
        #[arg(
            value_name = "TEXT",
            trailing_var_arg = true,
            allow_hyphen_values = true
        )]
        text: Vec<String>,
    },
    /// Read the visible pane viewport as plain text.
    Text {
        /// Target pane id (defaults to the first live pane).
        #[arg(long)]
        id: Option<u64>,
    },
    /// Print the pane's current working directory.
    Cwd {
        /// Target pane id (defaults to the first live pane).
        #[arg(long)]
        id: Option<u64>,
    },
    /// Print whether the pane appears idle or running.
    Status {
        /// Target pane id (defaults to the first live pane).
        #[arg(long)]
        id: Option<u64>,
    },
    /// Scan the visible viewport for common error patterns.
    Errors {
        /// Target pane id (defaults to the first live pane).
        #[arg(long)]
        id: Option<u64>,
    },
}

#[derive(Debug, Parser, Clone)]
pub struct RecordCommand {
    #[command(subcommand)]
    pub sub: RecordSubCommand,
}

#[derive(Debug, Parser, Clone)]
pub enum RecordSubCommand {
    /// Start recording on the target pane.
    Start {
        /// Target pane id (defaults to the first live pane).
        #[arg(long)]
        id: Option<u64>,
    },
    /// Stop recording on the target pane.
    Stop {
        #[arg(long)]
        id: Option<u64>,
    },
    /// Show recording status for the target pane.
    Status {
        #[arg(long)]
        id: Option<u64>,
    },
}

pub fn run(cmd: SessionCommand, json_out: bool) -> Result<()> {
    let mut client = McpClient::connect()?;
    match cmd.sub {
        SessionSubCommand::List => {
            let result = client.call("session.list", json!({}))?;
            if json_out {
                print_json(&result);
            } else {
                let sessions = result
                    .get("sessions")
                    .and_then(|v| v.as_array())
                    .cloned()
                    .unwrap_or_default();
                if sessions.is_empty() {
                    println!("{}", i18n::t("cli.session.empty"));
                } else {
                    println!(
                        "{:<5} {:<6} {:<6} {:<10} {}",
                        i18n::t("cli.session.head.id"),
                        i18n::t("cli.session.head.cols"),
                        i18n::t("cli.session.head.rows"),
                        i18n::t("cli.session.head.shell"),
                        i18n::t("cli.session.head.title")
                    );
                    for s in &sessions {
                        let id = s.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
                        let cols = s.get("cols").and_then(|v| v.as_u64()).unwrap_or(0);
                        let rows = s.get("rows").and_then(|v| v.as_u64()).unwrap_or(0);
                        let shell = s
                            .get("shell")
                            .and_then(|v| v.get("shell_type"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("?");
                        let title = s.get("title").and_then(|v| v.as_str()).unwrap_or("");
                        println!("{:<5} {:<6} {:<6} {:<10} {}", id, cols, rows, shell, title);
                    }
                }
            }
        }
        SessionSubCommand::Create {
            cwd,
            profile,
            command,
        } => {
            let mut params = json!({});
            if let Some(cwd) = cwd.as_ref() {
                params["cwd"] = json!(cwd.display().to_string());
            }
            if let Some(profile) = profile.as_ref() {
                params["profile"] = json!(profile);
            }
            let command = command.join(" ");
            if !command.trim().is_empty() {
                params["command"] = json!(command);
            }
            let result = client.call("session.create", params)?;
            if json_out {
                print_json(&result);
            } else {
                if let Some(id) = result.get("id").and_then(|v| v.as_u64()) {
                    print_kv("Pane", &id.to_string());
                }
                if let Some(title) = result.get("title").and_then(|v| v.as_str()) {
                    print_kv("Title", title);
                }
                if let Some(profile) = result.get("profile").and_then(|v| v.as_str()) {
                    print_kv("Profile", profile);
                }
            }
        }
        SessionSubCommand::Split {
            id,
            direction,
            size_percent,
            cwd,
        } => {
            let id = resolve_pane_id(&mut client, id)?;
            let mut params = json!({
                "id": id,
                "direction": direction,
                "size_percent": size_percent,
            });
            if let Some(cwd) = cwd.as_ref() {
                params["cwd"] = json!(cwd.display().to_string());
            }
            let result = client.call("session.split", params)?;
            if json_out {
                print_json(&result);
            } else {
                if let Some(id) = result.get("id").and_then(Value::as_u64) {
                    print_kv("Pane", &id.to_string());
                }
                if let Some(title) = result.get("title").and_then(Value::as_str) {
                    print_kv("Title", title);
                }
                if let Some(direction) = result.get("direction").and_then(Value::as_str) {
                    print_kv("Direction", direction);
                }
            }
        }
        SessionSubCommand::Focus { id } => {
            let id = resolve_pane_id(&mut client, id)?;
            let result = client.call("session.focus", json!({ "id": id }))?;
            if json_out {
                print_json(&result);
            } else {
                println!(
                    "{}",
                    result.get("ok").and_then(Value::as_bool).unwrap_or(false)
                );
            }
        }
        SessionSubCommand::Resize { id, cols, rows } => {
            let id = resolve_pane_id(&mut client, id)?;
            let result = client.call(
                "session.resize",
                json!({ "id": id, "cols": cols, "rows": rows }),
            )?;
            if json_out {
                print_json(&result);
            } else {
                println!(
                    "{}",
                    result.get("status").and_then(Value::as_str).unwrap_or("ok")
                );
            }
        }
        SessionSubCommand::Destroy { id } => {
            let result = client.call("session.destroy", json!({ "id": id }))?;
            if json_out {
                print_json(&result);
            } else {
                println!(
                    "{}",
                    result
                        .get("destroyed")
                        .and_then(Value::as_bool)
                        .unwrap_or(false)
                );
            }
        }
        SessionSubCommand::Record(rec) => match rec.sub {
            RecordSubCommand::Start { id } => {
                let id = resolve_pane_id(&mut client, id)?;
                let result = client.call("session.recording_start", json!({ "id": id }))?;
                if json_out {
                    print_json(&result);
                } else {
                    if let Some(sid) = result.get("session_id").and_then(|v| v.as_str()) {
                        print_kv(&i18n::t("cli.session.label.session_id"), sid);
                    }
                    if let Some(p) = result.get("log_path").and_then(|v| v.as_str()) {
                        print_kv(&i18n::t("cli.session.label.log_path"), p);
                    }
                    if let Some(p) = result.get("md_path_when_done").and_then(|v| v.as_str()) {
                        print_kv(&i18n::t("cli.session.label.md_when_done"), p);
                    }
                }
            }
            RecordSubCommand::Stop { id } => {
                let id = resolve_pane_id(&mut client, id)?;
                let result = client.call("session.recording_stop", json!({ "id": id }))?;
                if json_out {
                    print_json(&result);
                } else {
                    if let Some(sid) = result.get("session_id").and_then(|v| v.as_str()) {
                        print_kv(&i18n::t("cli.session.label.session_id"), sid);
                    }
                    if let Some(c) = result.get("block_count").and_then(|v| v.as_u64()) {
                        print_kv(&i18n::t("cli.session.label.block_count"), &c.to_string());
                    }
                    if let Some(p) = result.get("md_path").and_then(|v| v.as_str()) {
                        print_kv(&i18n::t("cli.session.label.markdown"), p);
                    }
                    if let Some(reason) = result.get("exit_reason").and_then(|v| v.as_str()) {
                        print_kv(&i18n::t("cli.session.label.exit_reason"), reason);
                    }
                }
            }
            RecordSubCommand::Status { id } => {
                let id = resolve_pane_id(&mut client, id)?;
                let result = client.call("session.recording_status", json!({ "id": id }))?;
                if json_out {
                    print_json(&result);
                } else {
                    let active = result
                        .get("enabled")
                        .or_else(|| result.get("active"))
                        .or_else(|| result.get("recording"))
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    let yes_no = if active {
                        i18n::t("cli.session.recording.yes")
                    } else {
                        i18n::t("cli.session.recording.no")
                    };
                    print_kv(&i18n::t("cli.session.label.recording"), &yes_no);
                    if let Some(sid) = result.get("session_id").and_then(|v| v.as_str()) {
                        print_kv(&i18n::t("cli.session.label.session_id"), sid);
                    }
                    if let Some(c) = result.get("block_count").and_then(|v| v.as_u64()) {
                        print_kv(&i18n::t("cli.session.label.block_count"), &c.to_string());
                    }
                }
            }
        },
        SessionSubCommand::Export { id, output } => {
            let id = resolve_pane_id(&mut client, id)?;
            let mut params = json!({ "id": id });
            if let Some(out) = output.as_ref() {
                // If the caller supplied a path, ask MCP to write directly there.
                params["path"] = json!(out.display().to_string());
            }
            let result = client.call("session.export_markdown", params)?;
            let mcp_path = result
                .get("path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("session.export_markdown did not return a path"))?;

            // If caller asked for an explicit destination and MCP wrote elsewhere,
            // copy the file across so `-o FILE` always lands at FILE.
            if let Some(dest) = output.as_ref() {
                let dest_path = dest.canonicalize().unwrap_or_else(|_| dest.clone());
                let src_path = std::path::Path::new(mcp_path);
                let src_canon = src_path
                    .canonicalize()
                    .unwrap_or_else(|_| src_path.to_path_buf());
                if src_canon != dest_path {
                    if let Some(parent) = dest.parent() {
                        if !parent.as_os_str().is_empty() {
                            std::fs::create_dir_all(parent).ok();
                        }
                    }
                    std::fs::copy(src_path, dest)?;
                }
            }

            if json_out {
                print_json(&result);
            } else if output.is_some() {
                println!("{}", output.unwrap().display());
            } else {
                println!("{}", mcp_path);
            }
        }
        SessionSubCommand::Input {
            id,
            stdin,
            enter,
            text,
        } => {
            let id = resolve_pane_id(&mut client, id)?;
            let mut input = text.join(" ");
            if stdin {
                let mut buf = String::new();
                std::io::stdin().read_to_string(&mut buf)?;
                if !input.is_empty() && !buf.is_empty() {
                    input.push('\n');
                }
                input.push_str(&buf);
            }
            if enter {
                input.push('\r');
            }
            if input.is_empty() {
                return Err(anyhow!("session input needs TEXT or --stdin"));
            }
            let result = client.call("session.input", json!({ "id": id, "input": input }))?;
            if json_out {
                print_json(&result);
            } else {
                println!(
                    "{}",
                    result.get("status").and_then(Value::as_str).unwrap_or("ok")
                );
            }
        }
        SessionSubCommand::Text { id } => {
            let id = resolve_pane_id(&mut client, id)?;
            let result = client.call("screen.text", json!({ "id": id }))?;
            if json_out {
                print_json(&result);
            } else {
                let lines = result
                    .get("lines")
                    .and_then(Value::as_array)
                    .ok_or_else(|| anyhow!("screen.text did not return `lines`: {}", result))?;
                for line in lines {
                    if let Some(text) = line.as_str() {
                        println!("{text}");
                    }
                }
            }
        }
        SessionSubCommand::Cwd { id } => {
            let id = resolve_pane_id(&mut client, id)?;
            let result = client.call("session.cwd", json!({ "id": id }))?;
            if json_out {
                print_json(&result);
            } else {
                println!(
                    "{}",
                    result.get("cwd").and_then(Value::as_str).unwrap_or("")
                );
            }
        }
        SessionSubCommand::Status { id } => {
            let id = resolve_pane_id(&mut client, id)?;
            let result = client.call("exec.status", json!({ "id": id }))?;
            if json_out {
                print_json(&result);
            } else {
                print_kv(
                    "Status",
                    result.get("status").and_then(Value::as_str).unwrap_or("?"),
                );
                if let Some(fg) = result.get("foreground_process").and_then(Value::as_str) {
                    print_kv("Foreground", fg);
                }
            }
        }
        SessionSubCommand::Errors { id } => {
            let id = resolve_pane_id(&mut client, id)?;
            let result = client.call("screen.detect_errors", json!({ "id": id }))?;
            if json_out {
                print_json(&result);
            } else {
                let errors = result
                    .get("errors")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                if errors.is_empty() {
                    println!("No visible errors.");
                } else {
                    println!("{:<8} {:<18} TEXT", "ROW", "PATTERN");
                    for err in errors {
                        let row = err.get("row").and_then(Value::as_i64).unwrap_or(0);
                        let pattern = err.get("pattern").and_then(Value::as_str).unwrap_or("");
                        let text = err.get("text").and_then(Value::as_str).unwrap_or("");
                        println!("{:<8} {:<18} {}", row, pattern, text);
                    }
                }
            }
        }
    }
    Ok(())
}

/// Pick the user-supplied id, or fall back to the first live pane.
fn resolve_pane_id(client: &mut McpClient, id: Option<u64>) -> Result<u64> {
    if let Some(id) = id {
        return Ok(id);
    }
    let result = client.call("session.list", json!({}))?;
    let sessions = result
        .get("sessions")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let first = sessions
        .first()
        .ok_or_else(|| anyhow!("{}", i18n::t("cli.session.no_panes")))?;
    first
        .get("id")
        .and_then(Value::as_u64)
        .ok_or_else(|| anyhow!("first pane is missing an integer id"))
}
