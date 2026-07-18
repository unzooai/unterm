//! `unterm-cli review ...` — inspect and act on agent-produced changes.
//!
//!     unterm-cli review list
//!     unterm-cli review diff --fleet <id> --member 1
//!     unterm-cli review diff --repo /path --from <sha>
//!     unterm-cli review merge --fleet <id> --member 1
//!     unterm-cli review discard --fleet <id> --member 1
//!     unterm-cli review rollback --repo /path --sha <sha> --yes
//!     unterm-cli review open

use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use serde_json::Value;

use super::client::McpClient;
use super::output::print_json;

#[derive(Debug, Parser, Clone)]
pub struct ReviewCommand {
    #[command(subcommand)]
    pub sub: ReviewSubCommand,
}

#[derive(Debug, Subcommand, Clone)]
pub enum ReviewSubCommand {
    /// Overview: fleets + auto checkpoints per repo.
    List,
    /// Line-level diff of a fleet member or a repo vs a checkpoint.
    Diff {
        #[arg(long = "fleet")]
        fleet: Option<String>,
        /// Member index (1-based) or branch name.
        #[arg(long)]
        member: Option<String>,
        #[arg(long)]
        repo: Option<String>,
        /// Checkpoint sha.
        #[arg(long)]
        from: Option<String>,
        /// Print only the file summary, not the patch.
        #[arg(long)]
        stat: bool,
    },
    /// Squash-merge a member into the base repo (leaves it staged).
    Merge {
        #[arg(long = "fleet")]
        fleet: String,
        #[arg(long)]
        member: String,
        /// Override the passed-verification gate. This action is audited.
        #[arg(long)]
        force: bool,
    },
    /// Run tests/validation for a fleet member asynchronously.
    Verify {
        #[arg(long = "fleet")]
        fleet: String,
        #[arg(long)]
        member: String,
        /// Explicit command; omitted to infer from project markers.
        #[arg(long)]
        command: Option<String>,
        #[arg(long)]
        timeout_secs: Option<u64>,
    },
    /// Mark a member's work as discarded.
    Discard {
        #[arg(long = "fleet")]
        fleet: String,
        #[arg(long)]
        member: String,
    },
    /// Restore a repo's worktree to a checkpoint. DESTRUCTIVE.
    Rollback {
        #[arg(long)]
        repo: String,
        #[arg(long)]
        sha: String,
        /// Required: confirms you want to discard everything newer.
        #[arg(long)]
        yes: bool,
    },
    /// Open the Review page in the browser.
    Open,
}

pub fn run(cmd: ReviewCommand, json_out: bool) -> Result<()> {
    match cmd.sub {
        ReviewSubCommand::List => {
            let mut client = McpClient::connect()?;
            let result = client.call("review.list", serde_json::json!({}))?;
            if json_out {
                print_json(&result);
                return Ok(());
            }
            let fleets = result
                .get("fleets")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            println!("fleets: {}", fleets.len());
            for f in &fleets {
                println!(
                    "  {}  task: {}",
                    f.get("id").and_then(Value::as_str).unwrap_or(""),
                    f.get("task").and_then(Value::as_str).unwrap_or(""),
                );
            }
            let repos = result
                .get("checkpoints")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            for r in &repos {
                let list = r
                    .get("checkpoints")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                println!(
                    "checkpoints in {} ({}):",
                    r.get("repo").and_then(Value::as_str).unwrap_or(""),
                    list.len()
                );
                for c in list.iter().take(5) {
                    println!(
                        "  {}  {}  by {}",
                        &c.get("sha").and_then(Value::as_str).unwrap_or("")[..12],
                        c.get("at").and_then(Value::as_str).unwrap_or(""),
                        c.get("agent").and_then(Value::as_str).unwrap_or(""),
                    );
                }
            }
        }
        ReviewSubCommand::Diff {
            fleet,
            member,
            repo,
            from,
            stat,
        } => {
            let mut client = McpClient::connect()?;
            let params = match (&fleet, &member, &repo, &from) {
                (Some(f), Some(m), _, _) => serde_json::json!({ "fleet_id": f, "member": m }),
                (_, _, Some(r), Some(s)) => serde_json::json!({ "repo": r, "from": s }),
                _ => {
                    return Err(anyhow!(
                        "need either --fleet + --member, or --repo + --from"
                    ))
                }
            };
            let result = client.call("review.diff", params)?;
            if json_out {
                print_json(&result);
                return Ok(());
            }
            for f in result
                .get("files")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
            {
                println!(
                    "  +{:<5} -{:<5} {}{}",
                    f.get("added").and_then(Value::as_str).unwrap_or("0"),
                    f.get("deleted").and_then(Value::as_str).unwrap_or("0"),
                    f.get("path").and_then(Value::as_str).unwrap_or(""),
                    if f.get("untracked").and_then(Value::as_bool).unwrap_or(false) {
                        " (new)"
                    } else {
                        ""
                    }
                );
            }
            if !stat {
                if let Some(patch) = result.get("patch").and_then(Value::as_str) {
                    println!("{patch}");
                }
            }
        }
        ReviewSubCommand::Merge { fleet, member, force } => {
            let mut client = McpClient::connect()?;
            let result = client.call(
                "review.merge",
                serde_json::json!({ "fleet_id": fleet, "member": member, "force": force }),
            )?;
            if json_out {
                print_json(&result);
            } else {
                println!(
                    "merged {} — staged in {} (commit it yourself)",
                    result.get("branch").and_then(Value::as_str).unwrap_or(""),
                    result
                        .get("staged_in")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                );
            }
        }
        ReviewSubCommand::Verify { fleet, member, command, timeout_secs } => {
            let mut client = McpClient::connect()?;
            let mut params = serde_json::json!({ "fleet_id": fleet, "member": member });
            if let Some(command) = command {
                params["command"] = Value::String(command);
            }
            if let Some(timeout) = timeout_secs {
                params["timeout_secs"] = Value::from(timeout);
            }
            let result = client.call("review.verify", params)?;
            if json_out {
                print_json(&result);
            } else {
                println!(
                    "verification {} queued: {}",
                    result.get("id").and_then(Value::as_str).unwrap_or("?"),
                    result.get("command").and_then(Value::as_str).unwrap_or("")
                );
            }
        }
        ReviewSubCommand::Discard { fleet, member } => {
            let mut client = McpClient::connect()?;
            let result = client.call(
                "review.discard",
                serde_json::json!({ "fleet_id": fleet, "member": member }),
            )?;
            if json_out {
                print_json(&result);
            } else {
                println!("member {member} of {fleet} discarded");
            }
        }
        ReviewSubCommand::Rollback { repo, sha, yes } => {
            if !yes {
                return Err(anyhow!(
                    "rollback discards everything newer than {sha} in {repo}; pass --yes to confirm"
                ));
            }
            let mut client = McpClient::connect()?;
            let result = client.call(
                "review.rollback",
                serde_json::json!({ "repo": repo, "sha": sha }),
            )?;
            if json_out {
                print_json(&result);
            } else {
                println!("rolled back {repo} to {sha}");
            }
        }
        ReviewSubCommand::Open => {
            let ep = super::client::ServerEndpoint::resolve()?;
            let url = format!("http://127.0.0.1:{}#review", ep.http_port);
            #[cfg(target_os = "macos")]
            let opener = "open";
            #[cfg(target_os = "windows")]
            let opener = "explorer";
            #[cfg(all(unix, not(target_os = "macos")))]
            let opener = "xdg-open";
            std::process::Command::new(opener).arg(&url).spawn()?;
            println!("{url}");
        }
    }
    Ok(())
}
