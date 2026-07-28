//! `unterm-cli policy ...` — MCP write-policy probes.

use super::client::McpClient;
use super::output::{print_json, print_kv};
use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use serde_json::{json, Value};

#[derive(Debug, Parser, Clone)]
pub struct PolicyCommand {
    #[command(subcommand)]
    pub sub: PolicySubCommand,
}

#[derive(Debug, Subcommand, Clone)]
pub enum PolicySubCommand {
    /// Check whether a command would be allowed by the current MCP policy.
    Check {
        /// Command to test. Use `--` before commands with flags.
        #[arg(
            value_name = "COMMAND",
            trailing_var_arg = true,
            allow_hyphen_values = true
        )]
        command: Vec<String>,
    },
}

pub fn run(cmd: PolicyCommand, json_out: bool) -> Result<()> {
    let mut client = McpClient::connect()?;
    match cmd.sub {
        PolicySubCommand::Check { command } => {
            let command = command.join(" ");
            if command.trim().is_empty() {
                return Err(anyhow!("policy check needs COMMAND"));
            }
            let result = client.call("policy.check", json!({ "command": command }))?;
            if json_out {
                print_json(&result);
            } else {
                println!(
                    "{}",
                    result
                        .get("allowed")
                        .and_then(Value::as_bool)
                        .unwrap_or(false)
                );
                if let Some(reason) = result.get("reason").and_then(Value::as_str) {
                    print_kv("Reason", reason);
                }
            }
        }
    }
    Ok(())
}
