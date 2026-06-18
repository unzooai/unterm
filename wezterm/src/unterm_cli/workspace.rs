//! `unterm-cli workspace ...` — save and restore pane workspaces.

use super::client::McpClient;
use super::output::{print_json, print_kv};
use anyhow::Result;
use clap::{Parser, Subcommand};
use serde_json::json;

#[derive(Debug, Parser, Clone)]
pub struct WorkspaceCommand {
    #[command(subcommand)]
    pub sub: WorkspaceSubCommand,
}

#[derive(Debug, Subcommand, Clone)]
pub enum WorkspaceSubCommand {
    /// List saved workspaces.
    List,
    /// Save the current window's panes as a named workspace.
    Save {
        /// Workspace name.
        name: String,
    },
    /// Restore a named workspace by opening its saved cwd entries as new tabs.
    Restore {
        /// Workspace name.
        name: String,
        /// Show what would be restored without opening tabs.
        #[arg(long)]
        dry_run: bool,
    },
}

pub fn run(cmd: WorkspaceCommand, json_out: bool) -> Result<()> {
    let mut client = McpClient::connect()?;
    match cmd.sub {
        WorkspaceSubCommand::List => {
            let result = client.call("workspace.list", json!({}))?;
            if json_out {
                print_json(&result);
            } else {
                let workspaces = result
                    .get("workspaces")
                    .and_then(|v| v.as_array())
                    .cloned()
                    .unwrap_or_default();
                if workspaces.is_empty() {
                    println!("No saved workspaces.");
                } else {
                    println!("{:<24}", "NAME");
                    for workspace in &workspaces {
                        println!(
                            "{:<24}",
                            workspace.get("name").and_then(|v| v.as_str()).unwrap_or("")
                        );
                    }
                }
            }
        }
        WorkspaceSubCommand::Save { name } => {
            let result = client.call("workspace.save", json!({ "name": name }))?;
            if json_out {
                print_json(&result);
            } else {
                print_kv(
                    "Workspace",
                    result.get("name").and_then(|v| v.as_str()).unwrap_or(""),
                );
                print_kv(
                    "Sessions",
                    &result
                        .get("sessions")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0)
                        .to_string(),
                );
            }
        }
        WorkspaceSubCommand::Restore { name, dry_run } => {
            let result = client.call(
                "workspace.restore",
                json!({
                    "name": name,
                    "dry_run": dry_run,
                }),
            )?;
            if json_out {
                print_json(&result);
            } else {
                print_kv(
                    "Workspace",
                    result.get("name").and_then(|v| v.as_str()).unwrap_or(""),
                );
                print_kv(
                    if dry_run { "Planned" } else { "Created" },
                    &result
                        .get(if dry_run { "planned" } else { "created" })
                        .and_then(|v| v.as_array())
                        .map(|v| v.len())
                        .unwrap_or(0)
                        .to_string(),
                );
                let failed = result
                    .get("failed")
                    .and_then(|v| v.as_array())
                    .map(|v| v.len())
                    .unwrap_or(0);
                if failed > 0 {
                    print_kv("Failed", &failed.to_string());
                }
            }
        }
    }
    Ok(())
}
