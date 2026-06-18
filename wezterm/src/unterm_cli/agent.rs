//! `unterm-cli agent ...` — discover, install, authenticate, configure, and
//! launch AI coding-agent CLIs from inside Unterm.
//!
//! The CLI is the source-of-truth surface (per global rule #4): the GUI's
//! Web Settings panel calls the same code path indirectly via REST routes,
//! so every behavior reachable from the picker is also reachable from a
//! script.
//!
//! Subcommand map:
//!
//!     unterm agent list                            list manifests + install state
//!     unterm agent show <id>                       full detail for one agent
//!     unterm agent install <id>                    run platform install steps
//!     unterm agent uninstall <id>                  reverse of install
//!     unterm agent update <id>                     pull a newer version of the binary
//!     unterm agent auth <id>                       start the auth flow (oauth/api key)
//!     unterm agent configure <id> --show
//!     unterm agent configure <id> --set k=v ...    write settings to disk + keychain
//!     unterm agent configure <id> --reset          restore manifest defaults
//!     unterm agent import <id>                     pull existing agent config into Unterm
//!     unterm agent launch <id>                     exec the agent with all env wired
//!     unterm agent run <id> <prompt>               run supported agents headlessly
//!     unterm agent manifest fetch                  hit the signed envelope endpoint
//!     unterm agent manifest verify                 verify the on-disk cache / baked fallback
//!     unterm agent manifest show                   pretty-print the active envelope

use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};
use serde_json::Value;
use std::collections::BTreeMap;
use std::io::Read;
use unterm_agents::manifest::SettingKind;
use unterm_agents::{
    fetch_manifests, fetch_or_fallback, installer, registry::SettingsState, AgentManifest,
    ManifestSet,
};

use super::output::print_json;

#[derive(Debug, Parser, Clone)]
pub struct AgentCommand {
    #[command(subcommand)]
    pub sub: AgentSubCommand,
}

#[derive(Debug, Subcommand, Clone)]
pub enum AgentSubCommand {
    /// List all AI agents in the current manifest + their install state.
    List {
        /// Show only agents that are currently installed.
        #[arg(long)]
        installed: bool,
    },
    /// Print a single agent's full manifest + current settings.
    Show { id: String },
    /// Run the platform-specific install steps for an agent.
    Install {
        id: String,
        /// Skip the `requires` precondition check (node ≥ N etc.). Use only
        /// if you know what you're doing.
        #[arg(long)]
        no_check_requires: bool,
    },
    /// Run the manifest's update command for an installed agent.
    Update { id: String },
    /// Run the uninstall command. Does NOT delete agent settings — use
    /// `unterm agent configure <id> --reset` if you want that.
    Uninstall { id: String },
    /// Authenticate. With no flags, runs the manifest's primary auth
    /// (typically `<bin> login`). Use `--api-key` to skip OAuth and store
    /// a raw key in the keychain.
    Auth {
        id: String,
        /// Read API key from stdin (no terminal echo). Useful for CI.
        #[arg(long)]
        api_key_stdin: bool,
        /// Force the api_key_env auth path; prompts on tty without stdin.
        #[arg(long)]
        api_key: bool,
        /// Profile to bind the credential to. Defaults to the active one.
        #[arg(long)]
        profile: Option<String>,
    },
    /// Read / write the per-profile settings for an agent.
    Configure {
        id: String,
        /// Print current effective settings as JSON.
        #[arg(long)]
        show: bool,
        /// `key=value` pairs. Repeatable. Use `key=null` to remove.
        #[arg(long = "set", value_name = "KEY=VALUE", action = clap::ArgAction::Append)]
        set: Vec<String>,
        /// Restore all settings to manifest defaults.
        #[arg(long)]
        reset: bool,
        #[arg(long)]
        profile: Option<String>,
    },
    /// Import the agent's existing config file (if any) into Unterm's
    /// settings — useful when you'd already set things up by hand.
    Import {
        id: String,
        #[arg(long)]
        profile: Option<String>,
    },
    /// Print the LaunchPlan that would be exec'd if you ran `launch`,
    /// without actually launching. Set `UNTERM_AGENT_PRINT_PLAN=1` env to
    /// effectively make `launch` itself a dry-run.
    Plan {
        id: String,
        #[arg(long)]
        profile: Option<String>,
        #[arg(long)]
        cwd: Option<String>,
    },
    /// exec(2) the agent in the current process with all env wired.
    Launch {
        id: String,
        #[arg(long)]
        profile: Option<String>,
        #[arg(long)]
        cwd: Option<String>,
    },
    /// Run an agent task non-interactively and wait for it to finish.
    ///
    /// Supported today: `codex-cli` (`codex exec`), `claude-code`
    /// (`claude -p`), `gemini-cli` (`gemini -p`), and `opencode`
    /// (`opencode run`). The command reuses the same profile, auth, launch
    /// flags, and MCP autowiring as `agent launch`.
    Run {
        id: String,
        #[arg(long)]
        profile: Option<String>,
        #[arg(long)]
        cwd: Option<String>,
        /// Read the prompt from stdin. Useful for piping diffs, logs, or
        /// exported sessions into Codex / Claude Code.
        #[arg(long)]
        stdin: bool,
        /// Print the exact command that would run, with sensitive env redacted.
        #[arg(long)]
        dry_run: bool,
        #[arg(trailing_var_arg = true)]
        prompt: Vec<String>,
    },
    /// Manifest catalog operations: fetch, verify, show.
    Manifest {
        #[command(subcommand)]
        sub: ManifestSubCommand,
    },
}

#[derive(Debug, Subcommand, Clone)]
pub enum ManifestSubCommand {
    /// Force a fresh fetch from the network (ignores ETag).
    Fetch,
    /// Verify the on-disk cache (or baked fallback) parses + verifies.
    Verify,
    /// Print the active envelope. Source tag tells you network/cache/baked.
    Show,
}

pub fn run(cmd: AgentCommand, json_out: bool) -> Result<()> {
    match cmd.sub {
        AgentSubCommand::List { installed } => run_list(installed, json_out),
        AgentSubCommand::Show { id } => run_show(&id, json_out),
        AgentSubCommand::Install {
            id,
            no_check_requires,
        } => run_install(&id, no_check_requires, json_out),
        AgentSubCommand::Update { id } => run_update(&id, json_out),
        AgentSubCommand::Uninstall { id } => run_uninstall(&id, json_out),
        AgentSubCommand::Auth {
            id,
            api_key_stdin,
            api_key,
            profile,
        } => run_auth(&id, profile.as_deref(), api_key, api_key_stdin, json_out),
        AgentSubCommand::Configure {
            id,
            show,
            set,
            reset,
            profile,
        } => run_configure(&id, profile.as_deref(), show, &set, reset, json_out),
        AgentSubCommand::Import { id, profile } => run_import(&id, profile.as_deref(), json_out),
        AgentSubCommand::Plan { id, profile, cwd } => {
            run_plan(&id, profile.as_deref(), cwd.as_deref(), json_out)
        }
        AgentSubCommand::Launch { id, profile, cwd } => {
            run_launch(&id, profile.as_deref(), cwd.as_deref())
        }
        AgentSubCommand::Run {
            id,
            profile,
            cwd,
            stdin,
            dry_run,
            prompt,
        } => run_headless(
            &id,
            profile.as_deref(),
            cwd.as_deref(),
            stdin,
            dry_run,
            &prompt,
            json_out,
        ),
        AgentSubCommand::Manifest { sub } => run_manifest(sub, json_out),
    }
}

// ---------- list / show ----------

fn run_list(only_installed: bool, json_out: bool) -> Result<()> {
    let set = fetch_manifests().map_err(|e| anyhow!(e.to_string()))?;
    let manifests = set.for_current_platform();
    let rows: Vec<Value> = manifests
        .iter()
        .map(|m| {
            let detect = installer::detect(&m.detect);
            serde_json::json!({
                "id": m.id,
                "name": m.name,
                "vendor": m.vendor,
                "version": m.version,
                "popularity_rank": m.popularity_rank,
                "category": m.category,
                "installed": detect.ok,
                "detected_version": detect.version,
                "binary_path": detect.binary_path,
            })
        })
        .filter(|row| !only_installed || row["installed"].as_bool().unwrap_or(false))
        .collect();
    if json_out {
        print_json(&serde_json::json!({
            "source": format!("{:?}", set.source),
            "envelope_issued_at": set.envelope.issued_at,
            "envelope_expires_at": set.envelope.expires_at,
            "agents": rows,
        }));
    } else {
        println!(
            "{:<14} {:<24} {:<12} {}",
            "ID", "NAME", "INSTALLED", "VERSION"
        );
        for row in &rows {
            println!(
                "{:<14} {:<24} {:<12} {}",
                row["id"].as_str().unwrap_or(""),
                row["name"].as_str().unwrap_or(""),
                if row["installed"].as_bool().unwrap_or(false) {
                    "yes"
                } else {
                    "no"
                },
                row["detected_version"].as_str().unwrap_or("-"),
            );
        }
        eprintln!(
            "\nmanifest envelope source: {:?}, issued {}",
            set.source, set.envelope.issued_at
        );
    }
    Ok(())
}

fn run_show(id: &str, json_out: bool) -> Result<()> {
    let (set, manifest) = lookup(id)?;
    let detect = installer::detect(&manifest.detect);
    if json_out {
        print_json(&serde_json::json!({
            "manifest_source": format!("{:?}", set.source),
            "manifest": manifest,
            "detect": serde_json::json!({
                "ok": detect.ok,
                "version": detect.version,
                "binary_path": detect.binary_path,
                "stderr_sample": detect.stderr_sample,
            })
        }));
    } else {
        println!(
            "{} ({}) — v{}",
            manifest.name, manifest.vendor, manifest.version
        );
        println!(
            "  homepage: {}",
            manifest.homepage.as_deref().unwrap_or("-")
        );
        println!("  installed: {}", detect.ok);
        if let Some(v) = &detect.version {
            println!("  detected version: {v}");
        }
        if let Some(p) = &detect.binary_path {
            println!("  binary path: {p}");
        }
        println!("\nSchema:");
        for s in &manifest.settings_schema {
            println!(
                "  {:<20} {:<10} default={}",
                s.key,
                format!("{:?}", s.kind),
                s.default
                    .as_ref()
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "—".into())
            );
        }
    }
    Ok(())
}

fn lookup(id: &str) -> Result<(ManifestSet, AgentManifest)> {
    let set = fetch_manifests().map_err(|e| anyhow!(e.to_string()))?;
    let m = set
        .find(id)
        .ok_or_else(|| anyhow!("agent {id:?} not in current manifest set"))?
        .clone();
    Ok((set, m))
}

// ---------- install / update / uninstall ----------

fn run_install(id: &str, _no_check_requires: bool, json_out: bool) -> Result<()> {
    let (_, manifest) = lookup(id)?;
    let reports = installer::run_install(&manifest).map_err(|e| anyhow!(e.to_string()))?;
    if json_out {
        let arr: Vec<_> = reports
            .iter()
            .map(|r| {
                serde_json::json!({
                    "label": r.label,
                    "success": r.success,
                    "exit_code": r.exit_code,
                    "stdout_tail": r.stdout_tail,
                    "stderr_tail": r.stderr_tail,
                })
            })
            .collect();
        print_json(&serde_json::json!({ "id": manifest.id, "steps": arr }));
    } else {
        for r in &reports {
            println!(
                "  [{}] {} (exit {:?})",
                if r.success { "ok" } else { "fail" },
                r.label,
                r.exit_code
            );
        }
        println!("\nInstalled {} ({})", manifest.name, manifest.id);
    }
    Ok(())
}

fn run_update(id: &str, json_out: bool) -> Result<()> {
    let (_, manifest) = lookup(id)?;
    let report = installer::run_update(&manifest).map_err(|e| anyhow!(e.to_string()))?;
    if json_out {
        print_json(&serde_json::json!({
            "label": report.label,
            "success": report.success,
            "exit_code": report.exit_code,
        }));
    } else {
        println!("update: {} (exit {:?})", report.label, report.exit_code);
    }
    Ok(())
}

fn run_uninstall(id: &str, json_out: bool) -> Result<()> {
    let (_, manifest) = lookup(id)?;
    let report = installer::run_uninstall(&manifest).map_err(|e| anyhow!(e.to_string()))?;
    if json_out {
        print_json(&serde_json::json!({
            "label": report.label,
            "success": report.success,
        }));
    } else {
        println!("uninstall: {} (exit {:?})", report.label, report.exit_code);
    }
    Ok(())
}

// ---------- auth ----------

fn run_auth(
    id: &str,
    profile: Option<&str>,
    force_api_key: bool,
    api_key_stdin: bool,
    json_out: bool,
) -> Result<()> {
    let (_, manifest) = lookup(id)?;
    let profile_id = profile_or_default(profile)?;

    let use_api_key = force_api_key || api_key_stdin;
    if use_api_key {
        let key = if api_key_stdin {
            use std::io::Read;
            let mut s = String::new();
            std::io::stdin().read_to_string(&mut s)?;
            s.trim().to_string()
        } else {
            // Prompt on tty (no echo).
            rpassword::prompt_password(format!("API key for {id}: "))?
        };
        let outcome = unterm_agents::authn::run_api_key(&manifest.auth, &profile_id, &key)
            .map_err(|e| anyhow!(e.to_string()))?;
        if json_out {
            print_json(
                &serde_json::json!({ "method": outcome.method_used, "profile": profile_id }),
            );
        } else {
            println!(
                "Stored key for {} in profile {profile_id}'s keychain ({}).",
                manifest.name, outcome.method_used
            );
        }
    } else {
        let outcome = unterm_agents::authn::run_oauth_browser(&manifest.auth)
            .map_err(|e| anyhow!(e.to_string()))?;
        if json_out {
            print_json(&serde_json::json!({ "method": outcome.method_used }));
        } else {
            println!(
                "OAuth completed for {} via vendor flow.\n  {}",
                manifest.name, outcome.stdout_tail
            );
        }
    }
    Ok(())
}

// ---------- configure ----------

fn run_configure(
    id: &str,
    profile: Option<&str>,
    show: bool,
    set: &[String],
    reset: bool,
    json_out: bool,
) -> Result<()> {
    let (_, manifest) = lookup(id)?;
    let profile_id = profile_or_default(profile)?;
    let mut state =
        SettingsState::load(&profile_id, &manifest.id).map_err(|e| anyhow!(e.to_string()))?;
    state.merge_defaults(&manifest.settings_schema);

    if reset {
        let mut fresh = SettingsState::default();
        fresh.merge_defaults(&manifest.settings_schema);
        fresh
            .save(&profile_id, &manifest.id)
            .map_err(|e| anyhow!(e.to_string()))?;
        println!("Reset {} settings to manifest defaults.", manifest.id);
        return Ok(());
    }

    if !set.is_empty() {
        let updates = parse_kv_pairs(set, &manifest)?;
        let outcome =
            unterm_agents::registry::apply_updates(&manifest, &profile_id, &mut state, updates)
                .map_err(|e| anyhow!(e.to_string()))?;
        if json_out {
            print_json(&serde_json::json!({
                "written_files": outcome.written_files,
                "written_secrets": outcome.written_secrets,
                "skipped_unknown": outcome.skipped_unknown,
            }));
        } else {
            for f in &outcome.written_files {
                println!("  wrote {}", f);
            }
            for s in &outcome.written_secrets {
                println!("  stored secret -> keychain ({s})");
            }
            for s in &outcome.skipped_unknown {
                eprintln!("  warning: {s} not in schema; skipped");
            }
            println!("\nApplied {} setting(s).", set.len());
        }
    }

    if show || set.is_empty() && !reset {
        // Display only — redact secrets.
        let mut display: BTreeMap<String, Value> = BTreeMap::new();
        for (k, v) in &state.values {
            let redacted = if manifest
                .settings_schema
                .iter()
                .any(|s| s.key == *k && matches!(s.kind, SettingKind::Secret))
            {
                Value::String("***".into())
            } else {
                v.clone()
            };
            display.insert(k.clone(), redacted);
        }
        if json_out {
            print_json(&serde_json::json!({
                "agent": manifest.id,
                "profile": profile_id,
                "manifest_version": state.manifest_version,
                "values": display,
            }));
        } else {
            println!("Settings for {} in profile {profile_id}:", manifest.id);
            for (k, v) in &display {
                println!("  {k:<24} {v}");
            }
        }
    }
    Ok(())
}

fn parse_kv_pairs(set: &[String], manifest: &AgentManifest) -> Result<BTreeMap<String, Value>> {
    let mut out = BTreeMap::new();
    for pair in set {
        let (k, v) = pair
            .split_once('=')
            .ok_or_else(|| anyhow!("--set expects key=value, got {pair:?}"))?;
        let value = parse_value_for_key(manifest, k, v)?;
        out.insert(k.to_string(), value);
    }
    Ok(out)
}

fn parse_value_for_key(manifest: &AgentManifest, key: &str, raw: &str) -> Result<Value> {
    // null → JSON Null (will be saved as such; storage layer leaves the
    // file entry in place but marks the value cleared).
    if raw == "null" {
        return Ok(Value::Null);
    }
    let Some(spec) = manifest.settings_schema.iter().find(|s| s.key == key) else {
        return Ok(Value::String(raw.to_string())); // permissive fallback
    };
    Ok(match spec.kind {
        SettingKind::Bool => Value::Bool(raw == "true" || raw == "1" || raw == "yes"),
        SettingKind::Int => Value::Number(
            raw.parse::<i64>()
                .with_context(|| format!("expected integer for {key}, got {raw:?}"))?
                .into(),
        ),
        SettingKind::Float => Value::Number(
            serde_json::Number::from_f64(
                raw.parse::<f64>()
                    .with_context(|| format!("expected float for {key}, got {raw:?}"))?,
            )
            .ok_or_else(|| anyhow!("float value out of range"))?,
        ),
        SettingKind::MultiEnum => {
            // Comma-separated list of values
            Value::Array(
                raw.split(',')
                    .map(|s| Value::String(s.trim().into()))
                    .collect(),
            )
        }
        SettingKind::KeyValueList => {
            // a=1,b=2 form
            let mut m = serde_json::Map::new();
            for chunk in raw.split(',') {
                if let Some((k, v)) = chunk.split_once('=') {
                    m.insert(k.trim().to_string(), Value::String(v.trim().to_string()));
                }
            }
            Value::Object(m)
        }
        _ => Value::String(raw.to_string()),
    })
}

// ---------- import ----------

fn run_import(id: &str, profile: Option<&str>, json_out: bool) -> Result<()> {
    let (_, manifest) = lookup(id)?;
    let profile_id = profile_or_default(profile)?;
    let snap = unterm_agents::registry::snapshot_existing(&manifest, &profile_id)
        .map_err(|e| anyhow!(e.to_string()))?;
    if json_out {
        print_json(&serde_json::json!({
            "agent": manifest.id,
            "profile": profile_id,
            "imported": snap,
        }));
    } else if snap.is_empty() {
        println!(
            "No existing {} config found under this profile (nothing to import).",
            manifest.id
        );
    } else {
        let mut state =
            SettingsState::load(&profile_id, &manifest.id).map_err(|e| anyhow!(e.to_string()))?;
        for (k, v) in &snap {
            state.values.insert(k.clone(), v.clone());
        }
        state
            .save(&profile_id, &manifest.id)
            .map_err(|e| anyhow!(e.to_string()))?;
        println!("Imported {} key(s):", snap.len());
        for (k, _) in &snap {
            println!("  {k}");
        }
    }
    Ok(())
}

// ---------- launch / plan ----------

fn run_plan(id: &str, profile: Option<&str>, cwd: Option<&str>, json_out: bool) -> Result<()> {
    let (_, manifest) = lookup(id)?;
    let profile_id = profile_or_default(profile)?;
    let mut state =
        SettingsState::load(&profile_id, &manifest.id).map_err(|e| anyhow!(e.to_string()))?;
    state.merge_defaults(&manifest.settings_schema);
    let plan = unterm_agents::launcher::build_launch_plan(&unterm_agents::launcher::LaunchInputs {
        manifest: &manifest,
        profile_id: &profile_id,
        settings: &state,
        cwd,
        project_root: cwd, // CLI doesn't compute project root; GUI does.
        // `plan` is a dry-run preview; don't write MCP config files here.
        mcp: None,
    })
    .map_err(|e| anyhow!(e.to_string()))?;
    if json_out {
        print_json(&serde_json::json!({
            "exec": plan.exec,
            "args": plan.args,
            "env_set": plan.env_set.iter().map(|(k,v)| {
                let redacted = if k.ends_with("_API_KEY") || k.ends_with("_TOKEN") {
                    "***".to_string()
                } else { v.clone() };
                (k.clone(), redacted)
            }).collect::<BTreeMap<_,_>>(),
            "cwd": plan.cwd,
        }));
    } else {
        println!("$ {} {}", plan.exec, plan.args.join(" "));
        if let Some(c) = &plan.cwd {
            println!("  cwd: {c}");
        }
        for (k, v) in &plan.env_set {
            let display = if k.ends_with("_API_KEY") || k.ends_with("_TOKEN") {
                "***"
            } else {
                v.as_str()
            };
            println!("  env: {k}={display}");
        }
    }
    Ok(())
}

/// Resolve the running instance's MCP endpoint + this binary's path into the
/// shape the launcher needs to auto-wire an agent at `unterm-cli mcp-stdio`.
/// Returns None if the GUI isn't reachable (launch proceeds without wiring).
fn mcp_wire_info() -> Option<unterm_agents::launcher::McpWireInfo> {
    let ep = super::client::ServerEndpoint::resolve().ok()?;
    let cli = std::env::current_exe()
        .ok()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "unterm-cli".to_string());
    Some(unterm_agents::launcher::McpWireInfo {
        host: "127.0.0.1".to_string(),
        port: ep.port,
        token: ep.token,
        unterm_cli_path: cli,
    })
}

fn run_launch(id: &str, profile: Option<&str>, cwd: Option<&str>) -> Result<()> {
    let (_, manifest) = lookup(id)?;
    let profile_id = profile_or_default(profile)?;
    let mut state =
        SettingsState::load(&profile_id, &manifest.id).map_err(|e| anyhow!(e.to_string()))?;
    state.merge_defaults(&manifest.settings_schema);
    // Resolve the running instance's MCP endpoint + our own binary path so the
    // launcher can auto-wire the agent at `unterm-cli mcp-stdio`. Best-effort:
    // if the GUI isn't reachable we still launch, just without MCP wiring.
    let wire = mcp_wire_info();
    let plan = unterm_agents::launcher::build_launch_plan(&unterm_agents::launcher::LaunchInputs {
        manifest: &manifest,
        profile_id: &profile_id,
        settings: &state,
        cwd,
        project_root: cwd,
        mcp: wire.as_ref(),
    })
    .map_err(|e| anyhow!(e.to_string()))?;

    // Honour env injection then exec into the agent — Unterm should drop
    // out of the process tree once the agent is up.
    let mut cmd = std::process::Command::new(&plan.exec);
    cmd.args(&plan.args);
    if let Some(dir) = &plan.cwd {
        cmd.current_dir(dir);
    }
    for (k, v) in &plan.env_set {
        cmd.env(k, v);
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        return Err(anyhow!(cmd.exec()));
    }
    #[cfg(not(unix))]
    {
        let status = cmd.status()?;
        std::process::exit(status.code().unwrap_or(0));
    }
}

fn build_plan_for_run(
    id: &str,
    profile: Option<&str>,
    cwd: Option<&str>,
) -> Result<unterm_agents::launcher::LaunchPlan> {
    let (_, manifest) = lookup(id)?;
    let profile_id = profile_or_default(profile)?;
    let mut state =
        SettingsState::load(&profile_id, &manifest.id).map_err(|e| anyhow!(e.to_string()))?;
    state.merge_defaults(&manifest.settings_schema);
    let wire = mcp_wire_info();
    unterm_agents::launcher::build_launch_plan(&unterm_agents::launcher::LaunchInputs {
        manifest: &manifest,
        profile_id: &profile_id,
        settings: &state,
        cwd,
        project_root: cwd,
        mcp: wire.as_ref(),
    })
    .map_err(|e| anyhow!(e.to_string()))
}

fn headless_args(id: &str, mut base_args: Vec<String>, prompt: &str) -> Result<Vec<String>> {
    match id {
        "claude-code" => {
            base_args.push("-p".to_string());
            base_args.push(prompt.to_string());
            Ok(base_args)
        }
        "codex-cli" => {
            base_args.push("exec".to_string());
            base_args.push(prompt.to_string());
            Ok(base_args)
        }
        "gemini-cli" => {
            base_args.push("-p".to_string());
            base_args.push(prompt.to_string());
            Ok(base_args)
        }
        "opencode" => {
            base_args.push("run".to_string());
            base_args.push(prompt.to_string());
            Ok(base_args)
        }
        other => Err(anyhow!(
            "agent run currently supports codex-cli, claude-code, gemini-cli, and opencode, not {other}"
        )),
    }
}

fn run_headless(
    id: &str,
    profile: Option<&str>,
    cwd: Option<&str>,
    read_stdin: bool,
    dry_run: bool,
    prompt_parts: &[String],
    json_out: bool,
) -> Result<()> {
    let mut prompt = prompt_parts.join(" ");
    if read_stdin {
        let mut buf = String::new();
        std::io::stdin()
            .read_to_string(&mut buf)
            .context("reading agent run prompt from stdin")?;
        if !prompt.is_empty() && !buf.trim().is_empty() {
            prompt.push('\n');
        }
        prompt.push_str(&buf);
    }
    if prompt.trim().is_empty() {
        return Err(anyhow!(
            "agent run needs a prompt argument, or pass --stdin and pipe one in"
        ));
    }
    let plan = build_plan_for_run(id, profile, cwd)?;
    let args = headless_args(id, plan.args.clone(), &prompt)?;

    if dry_run {
        if json_out {
            let env_set = plan
                .env_set
                .iter()
                .map(|(k, v)| {
                    (
                        k.clone(),
                        if is_sensitive_env(k) {
                            Value::String("***".to_string())
                        } else {
                            Value::String(v.clone())
                        },
                    )
                })
                .collect::<serde_json::Map<_, _>>();
            print_json(&serde_json::json!({
                "agent": id,
                "profile": profile_or_default(profile)?,
                "cwd": plan.cwd,
                "exec": plan.exec,
                "args": args,
                "env_set": env_set,
                "prompt_chars": prompt.chars().count(),
                "dry_run": true,
            }));
            return Ok(());
        }
        println!("$ {} {}", plan.exec, shell_join(&args));
        if let Some(dir) = &plan.cwd {
            println!("  cwd: {dir}");
        }
        for (k, v) in &plan.env_set {
            let display = if is_sensitive_env(k) {
                "***"
            } else {
                v.as_str()
            };
            println!("  env: {k}={display}");
        }
        return Ok(());
    }

    let mut cmd = std::process::Command::new(&plan.exec);
    cmd.args(&args);
    if let Some(dir) = &plan.cwd {
        cmd.current_dir(dir);
    }
    for (k, v) in &plan.env_set {
        cmd.env(k, v);
    }
    let status = cmd.status().with_context(|| {
        format!(
            "failed to run headless agent command: {} {}",
            plan.exec,
            args.join(" ")
        )
    })?;
    if !status.success() {
        return Err(anyhow!(
            "headless agent exited with status {}",
            status
                .code()
                .map(|c| c.to_string())
                .unwrap_or_else(|| "signal".to_string())
        ));
    }
    Ok(())
}

fn is_sensitive_env(name: &str) -> bool {
    name.ends_with("_API_KEY") || name.ends_with("_TOKEN") || name == "UNTERM_MCP_TOKEN"
}

fn shell_join(args: &[String]) -> String {
    #[cfg(windows)]
    {
        return args
            .iter()
            .map(|arg| cmd_quote(arg))
            .collect::<Vec<_>>()
            .join(" ");
    }
    #[cfg(not(windows))]
    {
        args.iter()
            .map(|arg| shell_quote(arg))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

#[cfg(windows)]
fn cmd_quote(s: &str) -> String {
    if s.is_empty() {
        return "\"\"".to_string();
    }
    if s.bytes().all(|b| {
        b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.' | b'/' | b'\\' | b':' | b'=')
    }) {
        return s.to_string();
    }

    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    let mut backslashes = 0;
    for ch in s.chars() {
        match ch {
            '\\' => backslashes += 1,
            '"' => {
                out.push_str(&"\\".repeat(backslashes * 2 + 1));
                out.push('"');
                backslashes = 0;
            }
            _ => {
                if backslashes > 0 {
                    out.push_str(&"\\".repeat(backslashes));
                    backslashes = 0;
                }
                out.push(ch);
            }
        }
    }
    if backslashes > 0 {
        out.push_str(&"\\".repeat(backslashes * 2));
    }
    out.push('"');
    out
}

fn shell_quote(s: &str) -> String {
    if s.is_empty() {
        return "''".to_string();
    }
    if s.bytes()
        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.' | b'/' | b':' | b'='))
    {
        return s.to_string();
    }
    format!("'{}'", s.replace('\'', "'\\''"))
}

#[cfg(test)]
mod tests {
    use super::headless_args;

    #[test]
    fn agent_headless_args_cover_supported_adapters() {
        let prompt = "review this diff";
        assert_eq!(
            headless_args("codex-cli", vec!["--model".into(), "gpt-5".into()], prompt).unwrap(),
            vec!["--model", "gpt-5", "exec", prompt]
        );
        assert_eq!(
            headless_args(
                "claude-code",
                vec!["--model".into(), "sonnet".into()],
                prompt,
            )
            .unwrap(),
            vec!["--model", "sonnet", "-p", prompt]
        );
        assert_eq!(
            headless_args(
                "gemini-cli",
                vec!["--model".into(), "gemini-pro".into()],
                prompt,
            )
            .unwrap(),
            vec!["--model", "gemini-pro", "-p", prompt]
        );
        assert_eq!(
            headless_args(
                "opencode",
                vec!["--model".into(), "openai/gpt-5".into()],
                prompt,
            )
            .unwrap(),
            vec!["--model", "openai/gpt-5", "run", prompt]
        );
    }

    #[test]
    fn agent_headless_args_reject_unknown_agents() {
        let err = headless_args("aider", Vec::new(), "hello").unwrap_err();
        assert!(err.to_string().contains("gemini-cli"));
        assert!(err.to_string().contains("opencode"));
    }
}

// ---------- manifest catalog ----------

fn run_manifest(sub: ManifestSubCommand, json_out: bool) -> Result<()> {
    match sub {
        ManifestSubCommand::Fetch => {
            // Bust the on-disk etag so we force a fresh GET.
            if let Ok(etag) = unterm_agents::paths::manifest_etag_path() {
                let _ = std::fs::remove_file(etag);
            }
            let res = fetch_or_fallback().map_err(|e| anyhow!(e.to_string()))?;
            println!(
                "envelope source={:?} issued_at={} agents={}",
                res.source,
                res.envelope.issued_at,
                res.envelope.manifests.len()
            );
        }
        ManifestSubCommand::Verify => {
            let res = fetch_or_fallback().map_err(|e| anyhow!(e.to_string()))?;
            println!(
                "ok — envelope source={:?}, expires {}, key id {}",
                res.source, res.envelope.expires_at, res.envelope.signature.key_id
            );
        }
        ManifestSubCommand::Show => {
            let res = fetch_or_fallback().map_err(|e| anyhow!(e.to_string()))?;
            if json_out {
                print_json(&serde_json::to_value(&res.envelope)?);
            } else {
                println!("Source: {:?}", res.source);
                println!("Issued at:  {}", res.envelope.issued_at);
                println!("Expires at: {}", res.envelope.expires_at);
                println!("Min Unterm: {}", res.envelope.min_unterm_version);
                println!(
                    "Signature:  {} ({})",
                    res.envelope.signature.alg, res.envelope.signature.key_id
                );
                println!("Agents:");
                for m in &res.envelope.manifests {
                    println!("  {:<14} v{:<3} {}", m.id, m.version, m.name);
                }
            }
        }
    }
    Ok(())
}

// ---------- helpers ----------

fn profile_or_default(explicit: Option<&str>) -> Result<String> {
    if let Some(p) = explicit {
        return Ok(p.to_string());
    }
    // Fall back to $UNTERM_PROFILE if set (this is the env that profile
    // spawn injects when you `unterm profile spawn <id>`), else "default".
    Ok(std::env::var("UNTERM_PROFILE").unwrap_or_else(|_| "default".into()))
}
