//! `unterm-cli provider ...` — the things Unterm can reach outside itself.
//!
//! Everything goes through the MCP surface rather than through the registry
//! directly, for the same reason the settings page does: the CLI is a client
//! like any other, and a CLI that reached past the server would be testing
//! something no agent can do.

use super::client::McpClient;
use super::output::{print_json, print_kv};
use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use serde_json::{json, Value};

#[derive(Debug, Parser, Clone)]
pub struct ProviderCommand {
    #[command(subcommand)]
    pub sub: ProviderSubCommand,
}

#[derive(Debug, Subcommand, Clone)]
pub enum ProviderSubCommand {
    /// What can be reached, and how each one stands.
    List {
        /// Look again for providers that appeared, moved or went away.
        #[arg(long)]
        rediscover: bool,
    },
    /// Contact a provider and, the first time, remember who answered.
    Bind {
        provider: String,
    },
    /// Stop using one without forgetting it. Outstanding leases are revoked.
    Pause {
        provider: String,
    },
    /// Undo a pause and bind again.
    Resume {
        provider: String,
    },
    /// Forget a binding entirely: leases revoked, identity unpinned.
    Unbind {
        provider: String,
    },
    /// Bind, lease and make one harmless call, reporting each check.
    Diagnose {
        provider: String,
        /// The method to probe with. The default only reads.
        #[arg(long)]
        method: Option<String>,
    },
    /// Capability leases, newest first.
    Leases {
        /// Only the ones that still work.
        #[arg(long)]
        live: bool,
    },
    /// Ask for a lease on a capability.
    Acquire {
        /// browser, profile or computer.
        capability: String,
        /// Who is asking, for the audit trail and for actor-scoped grants.
        #[arg(long)]
        actor: Option<String>,
        #[arg(long)]
        task: Option<String>,
        #[arg(long)]
        ttl: Option<i64>,
    },
    /// Do one thing through a provider, under a lease.
    Call {
        lease: String,
        /// The provider's own tool name, e.g. tab_list.
        method: String,
        /// This use's sequence number. Must be higher than the last one.
        #[arg(long)]
        seq: i64,
        #[arg(long)]
        capability: String,
        /// JSON arguments for the tool.
        #[arg(long, default_value = "{}")]
        params: String,
        /// Repeating a key returns the first answer instead of acting twice.
        #[arg(long)]
        idempotency_key: Option<String>,
    },
    /// Questions the gateway is waiting on a person to answer.
    Approvals,
    /// Take one lease back.
    Revoke {
        lease: String,
    },
    /// Everything that authorised a lease, and what was done with it.
    Chain {
        lease: String,
    },
}

pub fn run(cmd: ProviderCommand, json_out: bool) -> Result<()> {
    let mut client = McpClient::connect()?;
    match cmd.sub {
        ProviderSubCommand::List { rediscover } => {
            let result = client.call("provider.list", json!({"rediscover": rediscover}))?;
            if json_out {
                print_json(&result);
                return Ok(());
            }
            let providers = result
                .get("providers")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            if providers.is_empty() {
                println!("No providers found.");
                // Not an error, and worth saying why rather than leaving the
                // user staring at an empty list.
                println!("Providers announce themselves; install one, or point at it with UNTERM_PROVIDER_<ID>.");
                return Ok(());
            }
            for provider in providers {
                let id = provider["id"].as_str().unwrap_or("?");
                let state = provider["state"].as_str().unwrap_or("?");
                println!("{id}  {state}");
                if let Some(detail) = provider["detail"].as_str() {
                    print_kv("  ", detail);
                }
                print_kv(
                    "  capabilities",
                    &provider["capabilities"]
                        .as_array()
                        .map(|items| {
                            items
                                .iter()
                                .filter_map(Value::as_str)
                                .collect::<Vec<_>>()
                                .join(", ")
                        })
                        .unwrap_or_default(),
                );
                print_kv("  endpoint", provider["endpoint"].as_str().unwrap_or("?"));
                print_kv("  found via", provider["source"].as_str().unwrap_or("?"));
                if let Some(pinned) = provider["pinned"].as_str() {
                    print_kv("  pinned to", pinned);
                }
                print_kv(
                    "  live leases",
                    &provider["live_leases"].as_i64().unwrap_or(0).to_string(),
                );
            }
        }

        ProviderSubCommand::Bind { provider } => {
            let result = client.call("provider.bind", json!({"provider": provider}))?;
            if json_out {
                print_json(&result);
            } else {
                print_kv(
                    "Bound to",
                    &format!(
                        "{} {}",
                        result["identity"]["name"].as_str().unwrap_or("?"),
                        result["identity"]["version"].as_str().unwrap_or("?")
                    ),
                );
                print_kv("Protocol", result["protocol"].as_str().unwrap_or("?"));
            }
        }

        ProviderSubCommand::Pause { provider } => {
            let result = client.call("provider.pause", json!({"provider": provider}))?;
            report_revocations(&result, json_out, "Paused");
        }

        ProviderSubCommand::Resume { provider } => {
            let result = client.call("provider.resume", json!({"provider": provider}))?;
            if json_out {
                print_json(&result);
            } else {
                print_kv("Resumed", result["provider"].as_str().unwrap_or("?"));
            }
        }

        ProviderSubCommand::Unbind { provider } => {
            let result = client.call("provider.unbind", json!({"provider": provider}))?;
            report_revocations(&result, json_out, "Unbound");
        }

        ProviderSubCommand::Diagnose { provider, method } => {
            let mut params = json!({"provider": provider});
            if let Some(method) = method {
                params["method"] = json!(method);
            }
            let result = client.call("provider.diagnose", params)?;
            if json_out {
                print_json(&result);
                return Ok(());
            }
            for check in result["checks"].as_array().cloned().unwrap_or_default() {
                println!(
                    "{} {:<12} {}",
                    if check["passed"].as_bool().unwrap_or(false) {
                        "ok  "
                    } else {
                        "FAIL"
                    },
                    check["name"].as_str().unwrap_or("?"),
                    check["detail"].as_str().unwrap_or("")
                );
            }
            if !result["passed"].as_bool().unwrap_or(false) {
                // A non-zero exit, so a script can rely on it.
                return Err(anyhow!("{provider} did not pass every check"));
            }
        }

        ProviderSubCommand::Leases { live } => {
            let result = client.call("provider.leases", json!({"live_only": live}))?;
            if json_out {
                print_json(&result);
                return Ok(());
            }
            let leases = result["leases"].as_array().cloned().unwrap_or_default();
            if leases.is_empty() {
                println!("No leases.");
                return Ok(());
            }
            for lease in leases {
                let state = if lease["revoked_at"].is_string() {
                    "revoked"
                } else {
                    "live"
                };
                println!(
                    "{}  {:<8} {:<9} {:<8} expires {}",
                    lease["id"].as_str().unwrap_or("?"),
                    lease["provider"].as_str().unwrap_or("?"),
                    lease["capability"].as_str().unwrap_or("?"),
                    state,
                    lease["expires_at"].as_str().unwrap_or("?"),
                );
            }
        }

        ProviderSubCommand::Acquire {
            capability,
            actor,
            task,
            ttl,
        } => {
            let mut params = json!({"capability": capability});
            if let Some(actor) = actor {
                params["actor"] = json!(actor);
            }
            if let Some(task) = task {
                params["task_id"] = json!(task);
            }
            if let Some(ttl) = ttl {
                params["ttl_seconds"] = json!(ttl);
            }
            let result = client.call("provider.acquire", params)?;
            if json_out {
                print_json(&result);
                return Ok(());
            }
            match result["state"].as_str().unwrap_or("?") {
                "ready" => {
                    print_kv("Lease", result["lease"]["id"].as_str().unwrap_or("?"));
                    print_kv("Epoch", &result["lease"]["epoch"].to_string());
                    print_kv("Until", result["lease"]["expires_at"].as_str().unwrap_or("?"));
                }
                "waiting" => {
                    // Two very different waits, and the reason says which.
                    print_kv("Waiting", result["reason"].as_str().unwrap_or("?"));
                    if let Some(detail) = result["detail"].as_str() {
                        print_kv("Detail", detail);
                    }
                }
                _ => {
                    print_kv("Denied", result["reason"].as_str().unwrap_or("?"));
                    if let Some(detail) = result["detail"].as_str() {
                        print_kv("Reason", detail);
                    }
                }
            }
        }

        ProviderSubCommand::Call {
            lease,
            method,
            seq,
            capability,
            params,
            idempotency_key,
        } => {
            let parsed: Value = serde_json::from_str(&params)
                .map_err(|error| anyhow!("--params is not JSON: {error}"))?;
            let mut request = json!({
                "lease": lease,
                "method": method,
                "seq": seq,
                "capability": capability,
                "params": parsed,
            });
            if let Some(key) = idempotency_key {
                request["idempotency_key"] = json!(key);
            }
            let result = client.call("provider.call", request)?;
            if json_out {
                print_json(&result);
            } else {
                print_kv(
                    "Evidence",
                    &format!(
                        "request {} response {}",
                        short(&result["evidence"]["request_sha256"]),
                        short(&result["evidence"]["response_sha256"])
                    ),
                );
                if result["replayed_from_record"].as_bool().unwrap_or(false) {
                    print_kv("Note", "answered from the record of an identical call");
                }
                print_json(&result["value"]);
            }
        }

        ProviderSubCommand::Approvals => {
            let result = client.call("approval.list", json!({}))?;
            if json_out {
                print_json(&result);
                return Ok(());
            }
            let asks = result["approvals"].as_array().cloned().unwrap_or_default();
            if asks.is_empty() {
                println!("Nothing is waiting.");
                return Ok(());
            }
            for ask in asks {
                println!(
                    "{}  {:<22} {:<14} {}",
                    ask["id"].as_str().unwrap_or("?"),
                    ask["method"].as_str().unwrap_or("?"),
                    ask["risk"].as_str().unwrap_or("?"),
                    ask["actor"].as_str().unwrap_or("someone"),
                );
            }
            // Said here rather than left for the user to discover: answering
            // over MCP is refused on purpose, and the CLI is on the wrong
            // side of that line for the same reason an agent is.
            println!("\nAnswer these in Unterm's settings — Providers.");
        }

        ProviderSubCommand::Revoke { lease } => {
            let result = client.call("provider.revoke_lease", json!({"lease": lease}))?;
            if json_out {
                print_json(&result);
            } else if result["revoked"].as_bool().unwrap_or(false) {
                print_kv("Revoked", &lease);
            } else {
                // Distinguished on purpose: nothing was taken back because
                // there was nothing left to take.
                print_kv("Already over", &lease);
            }
        }

        ProviderSubCommand::Chain { lease } => {
            let result = client.call("provider.chain", json!({"lease": lease}))?;
            if json_out {
                print_json(&result);
                return Ok(());
            }
            print_kv("Lease", result["lease"]["id"].as_str().unwrap_or("?"));
            print_kv(
                "Capability",
                &format!(
                    "{} on {}",
                    result["lease"]["capability"].as_str().unwrap_or("?"),
                    result["lease"]["provider"].as_str().unwrap_or("?")
                ),
            );
            match result["grant"]["id"].as_str() {
                Some(id) => print_kv(
                    "Granted by",
                    &format!(
                        "{id} ({} scope)",
                        result["grant"]["scope"].as_str().unwrap_or("?")
                    ),
                ),
                // Said out loud rather than left blank: a lease resting on no
                // standing permission is a fact about how it was issued.
                None => print_kv("Granted by", "no standing grant"),
            }
            match result["approval"]["id"].as_str() {
                Some(id) => print_kv(
                    "Approved by",
                    &format!(
                        "{} ({id})",
                        result["approval"]["decided_by"].as_str().unwrap_or("?")
                    ),
                ),
                None => print_kv("Approved by", "nobody was asked"),
            }
            if let Some(task) = result["task"]["title"].as_str() {
                print_kv("For task", task);
            }
            let calls = result["calls"].as_array().cloned().unwrap_or_default();
            print_kv("Calls made", &calls.len().to_string());
            for call in calls {
                println!(
                    "  {:<10} {:<10} {}",
                    call["method"].as_str().unwrap_or("?"),
                    call["state"].as_str().unwrap_or("?"),
                    call["response_sha256"]
                        .as_str()
                        .map(|hash| &hash[..hash.len().min(12)])
                        .unwrap_or("-"),
                );
            }
        }
    }
    Ok(())
}

fn short(value: &Value) -> String {
    value
        .as_str()
        .map(|hash| hash[..hash.len().min(8)].to_string())
        .unwrap_or_else(|| "-".into())
}

fn report_revocations(result: &Value, json_out: bool, verb: &str) {
    if json_out {
        print_json(result);
        return;
    }
    let revoked = result["leases_revoked"].as_i64().unwrap_or(0);
    print_kv(verb, result["provider"].as_str().unwrap_or("?"));
    if revoked > 0 {
        print_kv("Leases revoked", &revoked.to_string());
    }
}
