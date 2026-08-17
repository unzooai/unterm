//! `unterm-cli scope|artifact|evidence|audit …` — where work may happen, what
//! it produced, and whether the record still holds together.
//!
//! Three commands rather than one because they are three jobs a person does
//! at different times: setting up a workspace, looking at what accumulated,
//! and handing somebody a bundle. All of them go through MCP, like every
//! other client.

use super::client::McpClient;
use super::output::{print_json, print_kv};
use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use serde_json::json;

#[derive(Debug, Parser, Clone)]
pub struct ScopeCommand {
    #[command(subcommand)]
    pub sub: ScopeSubCommand,
}

#[derive(Debug, Subcommand, Clone)]
pub enum ScopeSubCommand {
    /// Workspaces and their roots.
    List,
    /// Make a directory into a workspace.
    Create { name: String, path: String },
    /// Whether a workspace may touch a path.
    Check {
        workspace: String,
        path: String,
        /// read (default) or write.
        #[arg(long, default_value = "read")]
        access: String,
    },
    /// Stop using a workspace without forgetting where it was.
    Archive { workspace: String },
}

#[derive(Debug, Parser, Clone)]
pub struct ArtifactCommand {
    #[command(subcommand)]
    pub sub: ArtifactSubCommand,
}

#[derive(Debug, Subcommand, Clone)]
pub enum ArtifactSubCommand {
    /// What tasks produced.
    List {
        #[arg(long)]
        task: Option<String>,
    },
    /// How much is stored, and what deduplication saved.
    Usage,
    /// Whether an artifact's bytes still match their hash.
    Verify { artifact: String },
    /// Drop an artifact, and its bytes if nothing else refers to them.
    Forget { artifact: String },
}

#[derive(Debug, Parser, Clone)]
pub struct EvidenceCommand {
    #[command(subcommand)]
    pub sub: EvidenceSubCommand,
}

#[derive(Debug, Subcommand, Clone)]
pub enum EvidenceSubCommand {
    /// Write one task's whole story into a directory.
    Export { task: String, path: String },
    /// Recompute a bundle's hashes and report what does not hold.
    Verify { path: String },
    /// Walk the audit hash-chain and report the first break.
    Audit,
}

pub fn run_scope(cmd: ScopeCommand, json_out: bool) -> Result<()> {
    let mut client = McpClient::connect()?;
    match cmd.sub {
        ScopeSubCommand::List => {
            let result = client.call("scope.list", json!({}))?;
            if json_out {
                print_json(&result);
                return Ok(());
            }
            let list = result["workspaces"].as_array().cloned().unwrap_or_default();
            if list.is_empty() {
                println!("No workspaces.");
                return Ok(());
            }
            for workspace in list {
                println!(
                    "{}  {:<16} {}{}",
                    workspace["id"].as_str().unwrap_or("?"),
                    workspace["name"].as_str().unwrap_or("?"),
                    workspace["root"].as_str().unwrap_or("?"),
                    if workspace["archived_at"].is_string() {
                        "  (archived)"
                    } else {
                        ""
                    }
                );
            }
        }
        ScopeSubCommand::Create { name, path } => {
            let result = client.call("scope.create", json!({"name": name, "path": path}))?;
            if json_out {
                print_json(&result);
            } else {
                print_kv("Workspace", result["workspace"]["id"].as_str().unwrap_or("?"));
                print_kv("Root", result["workspace"]["root"].as_str().unwrap_or("?"));
            }
        }
        ScopeSubCommand::Check {
            workspace,
            path,
            access,
        } => {
            let result = client.call(
                "scope.check",
                json!({"workspace": workspace, "path": path, "access": access}),
            )?;
            if json_out {
                print_json(&result);
                return Ok(());
            }
            print_kv(
                if result["allowed"].as_bool().unwrap_or(false) {
                    "Allowed"
                } else {
                    "Denied"
                },
                result["reason"].as_str().unwrap_or("?"),
            );
            if let Some(resolved) = result["resolved_path"].as_str() {
                // The resolved path is the interesting half: it is what the
                // check actually judged, and rarely what was typed.
                print_kv("Resolved to", resolved);
            }
            if !result["allowed"].as_bool().unwrap_or(false) {
                return Err(anyhow!("{}", result["code"].as_str().unwrap_or("denied")));
            }
        }
        ScopeSubCommand::Archive { workspace } => {
            let result = client.call("scope.archive", json!({"workspace": workspace}))?;
            if json_out {
                print_json(&result);
            } else if result["archived"].as_bool().unwrap_or(false) {
                print_kv("Archived", &workspace);
            } else {
                print_kv("Already archived", &workspace);
            }
        }
    }
    Ok(())
}

pub fn run_artifact(cmd: ArtifactCommand, json_out: bool) -> Result<()> {
    let mut client = McpClient::connect()?;
    match cmd.sub {
        ArtifactSubCommand::List { task } => {
            let mut params = json!({});
            if let Some(task) = task {
                params["task_id"] = json!(task);
            }
            let result = client.call("artifact.list", params)?;
            if json_out {
                print_json(&result);
                return Ok(());
            }
            let list = result["artifacts"].as_array().cloned().unwrap_or_default();
            if list.is_empty() {
                println!("Nothing stored.");
                return Ok(());
            }
            for artifact in list {
                println!(
                    "{}  {:>9}  {:<14} {}",
                    artifact["id"].as_str().unwrap_or("?"),
                    artifact["bytes"].as_i64().unwrap_or(0),
                    artifact["origin"].as_str().unwrap_or("?"),
                    artifact["name"].as_str().unwrap_or(""),
                );
            }
        }
        ArtifactSubCommand::Usage => {
            let result = client.call("artifact.usage", json!({}))?;
            if json_out {
                print_json(&result);
                return Ok(());
            }
            let usage = &result["usage"];
            print_kv("Artifacts", &usage["artifacts"].to_string());
            print_kv("Distinct blobs", &usage["blobs"].to_string());
            print_kv("Bytes", &usage["bytes"].to_string());
            let saved = usage["bytes"].as_i64().unwrap_or(0) - usage["unique_bytes"].as_i64().unwrap_or(0);
            if saved > 0 {
                print_kv("Saved by sharing", &saved.to_string());
            }
        }
        ArtifactSubCommand::Verify { artifact } => {
            let result = client.call("artifact.verify", json!({"artifact": artifact}))?;
            if json_out {
                print_json(&result);
                return Ok(());
            }
            if result["intact"].as_bool().unwrap_or(false) {
                print_kv("Intact", result["sha256"].as_str().unwrap_or("?"));
            } else {
                // Non-zero exit: a script checking a store wants to know.
                return Err(anyhow!(
                    "{artifact} does not match its hash, or its bytes are gone"
                ));
            }
        }
        ArtifactSubCommand::Forget { artifact } => {
            let result = client.call("artifact.forget", json!({"artifact": artifact}))?;
            if json_out {
                print_json(&result);
            } else if result["forgotten"].as_bool().unwrap_or(false) {
                print_kv("Forgotten", &artifact);
            } else {
                print_kv("No such artifact", &artifact);
            }
        }
    }
    Ok(())
}

pub fn run_evidence(cmd: EvidenceCommand, json_out: bool) -> Result<()> {
    let mut client = McpClient::connect()?;
    match cmd.sub {
        EvidenceSubCommand::Export { task, path } => {
            let result =
                client.call("task.export_evidence", json!({"task_id": task, "path": path}))?;
            if json_out {
                print_json(&result);
                return Ok(());
            }
            let manifest = &result["manifest"];
            print_kv("Bundle", &path);
            print_kv("Record hash", manifest["record_sha256"].as_str().unwrap_or("?"));
            for key in ["runs", "steps", "leases", "calls", "artifacts", "audit"] {
                print_kv(key, &manifest["counts"][key].to_string());
            }
        }
        EvidenceSubCommand::Verify { path } => {
            let result = client.call("task.verify_evidence", json!({"path": path}))?;
            if json_out {
                print_json(&result);
                return Ok(());
            }
            let problems = result["problems"].as_array().cloned().unwrap_or_default();
            for problem in &problems {
                println!("FAIL {}", problem.as_str().unwrap_or("?"));
            }
            print_kv(
                "Artifacts checked",
                &result["artifacts_checked"].to_string(),
            );
            if result["artifacts_missing"].as_i64().unwrap_or(0) > 0 {
                // Distinguished from a failure: the export said these were
                // already gone, and saying so is not the same as a mismatch.
                print_kv(
                    "Artifacts known missing",
                    &result["artifacts_missing"].to_string(),
                );
            }
            if !result["intact"].as_bool().unwrap_or(false) {
                return Err(anyhow!("the bundle does not hold together"));
            }
            print_kv("Intact", result["task_id"].as_str().unwrap_or("?"));
        }
        EvidenceSubCommand::Audit => {
            let result = client.call("audit.verify", json!({}))?;
            if json_out {
                print_json(&result);
                return Ok(());
            }
            print_kv("Entries", &result["entries"].to_string());
            if result["intact"].as_bool().unwrap_or(false) {
                print_kv("Chain", "intact");
                return Ok(());
            }
            if let Some(at) = result["broken_at"].as_i64() {
                print_kv("First break at entry", &at.to_string());
            }
            return Err(anyhow!(
                "{}",
                result["detail"].as_str().unwrap_or("the chain does not verify")
            ));
        }
    }
    Ok(())
}
