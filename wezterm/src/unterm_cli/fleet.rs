//! `unterm-cli fleet ...` — run one task across N agents in N isolated
//! git worktrees, then inspect and clean up.
//!
//!     unterm-cli fleet launch --agents claude,claude "fix the login bug"
//!     unterm-cli fleet list
//!     unterm-cli fleet clean <id> [--force]

use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use serde_json::Value;

use super::client::McpClient;
use super::output::print_json;

#[derive(Debug, Parser, Clone)]
pub struct FleetCommand {
    #[command(subcommand)]
    pub sub: FleetSubCommand,
}

#[derive(Debug, Subcommand, Clone)]
pub enum FleetSubCommand {
    /// Launch a fleet: one task, N agents, N worktrees, one tab each.
    Launch {
        /// Comma-separated member agents, e.g. claude,claude,codex.
        #[arg(long, value_delimiter = ',')]
        agents: Vec<String>,
        /// Repo path; defaults to the active pane's cwd.
        #[arg(long)]
        cwd: Option<String>,
        /// The task prompt every member receives.
        #[arg(trailing_var_arg = true, required = true)]
        task: Vec<String>,
    },
    /// List fleets with member branches, worktrees, and review states.
    List,
    /// Remove a fleet's worktrees, branches, and panes once reviewed.
    Clean {
        id: String,
        /// Skip the all-members-reviewed check.
        #[arg(long)]
        force: bool,
    },
    /// Restart a pending member in its existing worktree, preserving changes.
    Retry {
        #[arg(long = "fleet")]
        fleet: String,
        #[arg(long)]
        member: String,
    },
}

pub fn run(cmd: FleetCommand, json_out: bool) -> Result<()> {
    let mut client = McpClient::connect()?;
    match cmd.sub {
        FleetSubCommand::Launch { agents, cwd, task } => {
            if agents.is_empty() {
                return Err(anyhow!("--agents is required, e.g. --agents claude,claude"));
            }
            let mut params = serde_json::json!({
                "task": task.join(" "),
                "agents": agents,
            });
            if let Some(cwd) = cwd {
                params["cwd"] = Value::String(cwd);
            }
            let result = client.call("fleet.launch", params)?;
            if json_out {
                print_json(&result);
            } else {
                println!(
                    "fleet {} launched — {} member(s)",
                    result.get("id").and_then(Value::as_str).unwrap_or("?"),
                    result
                        .get("members")
                        .and_then(Value::as_array)
                        .map(|a| a.len())
                        .unwrap_or(0)
                );
                for m in result
                    .get("members")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default()
                {
                    println!(
                        "  {:<9} {}  {}",
                        m.get("agent").and_then(Value::as_str).unwrap_or(""),
                        m.get("branch").and_then(Value::as_str).unwrap_or(""),
                        m.get("worktree").and_then(Value::as_str).unwrap_or(""),
                    );
                }
            }
        }
        FleetSubCommand::List => {
            let result = client.call("fleet.list", serde_json::json!({}))?;
            if json_out {
                print_json(&result);
                return Ok(());
            }
            let fleets = result
                .get("fleets")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            if fleets.is_empty() {
                println!("no fleets");
                return Ok(());
            }
            for f in &fleets {
                println!(
                    "{}  ({})  task: {}",
                    f.get("id").and_then(Value::as_str).unwrap_or(""),
                    f.get("base_branch").and_then(Value::as_str).unwrap_or(""),
                    f.get("task").and_then(Value::as_str).unwrap_or(""),
                );
                for m in f
                    .get("members")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default()
                {
                    println!(
                        "  #{} {:<9} {:<10} review={:<9} {}",
                        m.get("n").and_then(Value::as_u64).unwrap_or(0),
                        m.get("agent").and_then(Value::as_str).unwrap_or(""),
                        m.get("agent_state").and_then(Value::as_str).unwrap_or("-"),
                        m.get("review").and_then(Value::as_str).unwrap_or(""),
                        m.get("branch").and_then(Value::as_str).unwrap_or(""),
                    );
                }
            }
        }
        FleetSubCommand::Clean { id, force } => {
            let result =
                client.call("fleet.clean", serde_json::json!({ "id": id, "force": force }))?;
            if json_out {
                print_json(&result);
            } else {
                println!("fleet {id} cleaned");
            }
        }
        FleetSubCommand::Retry { fleet, member } => {
            let result = client.call(
                "fleet.retry",
                serde_json::json!({ "fleet_id": fleet, "member": member }),
            )?;
            if json_out {
                print_json(&result);
            } else {
                println!(
                    "retried member {member} of {fleet} (attempt {})",
                    result.get("attempt").and_then(Value::as_u64).unwrap_or(0)
                );
            }
        }
    }
    Ok(())
}
