//! MCP request handler — bridges JSON-RPC methods to WezTerm's Mux API.
//! Implements all methods required by unterm-cli compatibility.

use crate::ai::client as ai_client;
use crate::ai::models::{ai_state, ChatMessage, ChatRole, InsightCard, InsightType, ModelProvider};
use crate::ghost_text::{ghost_text_state, GhostText};
use anyhow::{anyhow, Context, Result};
use base64::Engine as _;
use config::keyassignment::SpawnTabDomain;
use mux::pane::Pane;
use mux::Mux;
use parking_lot::Mutex;
use serde_json::{json, Value};
use std::net::ToSocketAddrs;
use std::sync::Arc;

/// Audit log entry
#[derive(Clone, serde::Serialize)]
struct AuditEntry {
    timestamp: String,
    method: String,
    session_id: Option<String>,
    detail: String,
    allowed: bool,
}

/// Command execution policy
#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct CommandPolicy {
    enabled: bool,
    blocked_patterns: Vec<String>,
    allowed_patterns: Vec<String>,
}

impl Default for CommandPolicy {
    fn default() -> Self {
        Self {
            enabled: false,
            blocked_patterns: Vec::new(),
            allowed_patterns: Vec::new(),
        }
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct ProxyNodeConfig {
    name: String,
    url: String,
    latency_ms: Option<u64>,
    available: bool,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct ProxySettings {
    enabled: bool,
    mode: String,
    http_proxy: Option<String>,
    socks_proxy: Option<String>,
    no_proxy: String,
    current_node: Option<String>,
    nodes: Vec<ProxyNodeConfig>,
}

impl Default for ProxySettings {
    fn default() -> Self {
        Self {
            enabled: false,
            mode: "off".to_string(),
            http_proxy: None,
            socks_proxy: None,
            no_proxy: "localhost,127.0.0.1,::1".to_string(),
            current_node: None,
            nodes: Vec::new(),
        }
    }
}

/// Global state for audit + policy + workspace
struct McpState {
    audit_log: Vec<AuditEntry>,
    policy: CommandPolicy,
    proxy: ProxySettings,
}

fn mcp_state() -> &'static Mutex<McpState> {
    static STATE: std::sync::OnceLock<Mutex<McpState>> = std::sync::OnceLock::new();
    STATE.get_or_init(|| {
        Mutex::new(McpState {
            audit_log: Vec::new(),
            policy: CommandPolicy::default(),
            proxy: load_proxy_settings(),
        })
    })
}

pub struct McpHandler;

impl McpHandler {
    pub fn new() -> Self {
        Self
    }

    pub fn handle(&self, method: &str, params: &Value) -> Result<Value> {
        match method {
            // Session management
            "session.list" => self.session_list(),
            "session.get" | "session.status" => self.session_get(params),
            "session.create" => self.session_create(params),
            "session.input" => self.session_input(params),
            "session.resize" => self.session_resize(params),
            "session.destroy" => self.session_destroy(params),
            "session.idle" => self.session_idle(params),
            "session.cwd" => self.session_cwd(params),
            "session.env" => self.session_env(params),
            "session.set_env" => self.session_set_env(params),
            "session.history" => self.session_history(params),
            "session.audit_log" => self.session_audit_log(params),
            // Exec
            "exec.run" => self.exec_run(params),
            "exec.send" => self.session_input(params),
            "exec.run_wait" => self.exec_run_wait(params),
            "exec.status" => self.exec_status(params),
            "exec.cancel" => self.exec_cancel(params),
            // Screen
            "screen.read" => self.screen_read(params),
            "screen.text" => self.screen_text(params),
            "screen.cursor" => self.screen_cursor(params),
            "screen.scroll" => self.screen_scroll(params),
            "screen.search" => self.screen_search(params),
            "screen.detect_errors" => self.screen_detect_errors(params),
            // Signal
            "signal.send" => self.signal_send(params),
            // Ghost text
            "ghost_text.set" => self.ghost_text_set(params),
            "ghost_text.clear" => self.ghost_text_clear(params),
            // Orchestrate
            "orchestrate.launch" => self.orchestrate_launch(params),
            "orchestrate.broadcast" => self.orchestrate_broadcast(params),
            "orchestrate.wait" => self.orchestrate_wait(params),
            // Proxy
            "proxy.status" => self.proxy_status(),
            "proxy.nodes" => self.proxy_nodes(),
            "proxy.switch" => self.proxy_switch(params),
            "proxy.speedtest" => self.proxy_speedtest(params),
            "proxy.configure" => self.proxy_configure(params),
            "proxy.disable" => self.proxy_disable(),
            "proxy.env" => self.proxy_env(),
            // Workspace
            "workspace.save" => self.workspace_save(params),
            "workspace.restore" => self.workspace_restore(params),
            "workspace.list" => self.workspace_list(),
            // Capture
            "capture.screen" => self.capture_screen(params),
            "capture.window" => self.capture_window(params),
            "capture.select" => self.capture_select(),
            "capture.clipboard" => self.capture_clipboard(),
            // Policy
            "policy.set" => self.policy_set(params),
            "policy.check" => self.policy_check(params),
            // System
            "system.info" => self.system_info(),
            "system.launch_admin" => self.system_launch_admin(params),
            // AI
            "ai.complete" => self.ai_complete(params),
            "ai.chat" => self.ai_chat(params),
            "ai.analyze_error" => self.ai_analyze_error(params),
            "ai.suggest" => self.ai_suggest(params),
            "ai.suggest_next" => self.ai_suggest_next(params),
            "ai.set_model" => self.ai_set_model(params),
            "ai.get_model" => self.ai_get_model(),
            "ai.toggle_panel" => self.ai_toggle_panel(),
            "ai.set_insight" => self.ai_set_insight(params),
            "ai.focus_chat" => self.ai_focus_chat(params),
            "ai.send_chat_input" => self.ai_send_chat_input(params),
            "ai.panel_state" => self.ai_panel_state(),
            "server.info" => self.server_info(),
            "server.health" => self.server_health(),
            "server.capabilities" => self.server_capabilities(),
            "selftest.run" => self.selftest_run(params),
            _ => Err(anyhow!("Unknown method: {}", method)),
        }
    }

    fn get_mux(&self) -> Result<Arc<Mux>> {
        Mux::try_get().ok_or_else(|| anyhow!("Mux not available"))
    }

    fn get_pane(&self, params: &Value) -> Result<Arc<dyn Pane>> {
        let mux = self.get_mux()?;
        // Accept both numeric "id" and string "session_id"
        let id_val = params.get("id").or_else(|| params.get("session_id"));
        let id = match id_val {
            Some(v) if v.is_u64() => v.as_u64().unwrap() as usize,
            Some(v) if v.is_string() => v
                .as_str()
                .unwrap()
                .parse::<usize>()
                .map_err(|_| anyhow!("Invalid session_id: {}", v))?,
            _ => return Err(anyhow!("Missing 'id' or 'session_id' parameter")),
        };

        mux.get_pane(id)
            .ok_or_else(|| anyhow!("Session {} not found", id))
    }

    fn detect_shell(pane: &Arc<dyn Pane>) -> Value {
        let process_name = pane
            .get_foreground_process_name(mux::pane::CachePolicy::AllowStale)
            .unwrap_or_default();

        let shell_type = if process_name.is_empty() {
            "unknown"
        } else {
            let name = process_name
                .rsplit(['/', '\\'])
                .next()
                .unwrap_or(&process_name)
                .to_lowercase();
            if name.contains("pwsh") || name.contains("powershell") {
                "powershell"
            } else if name.contains("cmd") {
                "cmd"
            } else if name.contains("bash") {
                "bash"
            } else if name.contains("zsh") {
                "zsh"
            } else if name.contains("fish") {
                "fish"
            } else if name.contains("nu") {
                "nushell"
            } else {
                "unknown"
            }
        };

        let cwd = pane
            .get_current_working_dir(mux::pane::CachePolicy::AllowStale)
            .map(|u| u.to_string());

        json!({
            "shell_type": shell_type,
            "process_name": process_name,
            "cwd": cwd,
        })
    }

    fn server_info(&self) -> Result<Value> {
        Ok(json!({
            "name": "Unterm MCP Server",
            "version": "2.0.0",
            "engine": "Unterm (WezTerm)",
            "protocol": "json-rpc-2.0",
        }))
    }

    fn server_health(&self) -> Result<Value> {
        let mux_available = Mux::try_get().is_some();
        let pane_count = Mux::try_get()
            .map(|mux| mux.iter_panes().len())
            .unwrap_or_default();
        let ai = ai_state();
        let config = config::configuration();

        Ok(json!({
            "status": if mux_available { "ok" } else { "degraded" },
            "engine": "Unterm (WezTerm)",
            "mcp": {
                "bind": "127.0.0.1",
                "port": 19876,
                "auth": "token",
            },
            "mux": {
                "available": mux_available,
                "pane_count": pane_count,
            },
            "terminal": {
                "initial_cols": config.initial_cols,
                "initial_rows": config.initial_rows,
                "color_scheme": config.color_scheme,
                "term": config.term,
            },
            "ai": {
                "model": ai.active_model(),
                "provider": ai.provider().display_name(),
                "panel_visible": ai.panel_visible(),
                "chat_focused": ai.chat_focused(),
            },
        }))
    }

    fn server_capabilities(&self) -> Result<Value> {
        Ok(json!({
            "session": [
                "session.list",
                "session.create",
                "session.status",
                "session.input",
                "session.resize",
                "session.destroy",
                "session.idle",
                "session.cwd",
                "session.history",
                "session.audit_log"
            ],
            "exec": [
                "exec.run",
                "exec.send",
                "exec.run_wait",
                "exec.status",
                "exec.cancel",
                "signal.send"
            ],
            "screen": [
                "screen.read",
                "screen.text",
                "screen.cursor",
                "screen.scroll",
                "screen.search",
                "screen.detect_errors"
            ],
            "ai": [
                "ai.complete",
                "ai.chat",
                "ai.analyze_error",
                "ai.suggest",
                "ai.suggest_next",
                "ai.set_model",
                "ai.get_model",
                "ai.toggle_panel",
                "ai.set_insight",
                "ai.focus_chat",
                "ai.send_chat_input",
                "ai.panel_state"
            ],
            "ghost_text": [
                "ghost_text.set",
                "ghost_text.clear"
            ],
            "workspace": [
                "workspace.save",
                "workspace.restore",
                "workspace.list"
            ],
            "capture": [
                "capture.screen",
                "capture.window",
                "capture.select",
                "capture.clipboard"
            ],
            "proxy": [
                "proxy.status",
                "proxy.nodes",
                "proxy.switch",
                "proxy.speedtest",
                "proxy.configure",
                "proxy.disable",
                "proxy.env"
            ],
            "governance": [
                "policy.set",
                "policy.check",
                "server.info",
                "server.health",
                "server.capabilities",
                "selftest.run"
            ],
            "system": [
                "system.info",
                "system.launch_admin"
            ]
        }))
    }

    fn session_list(&self) -> Result<Value> {
        let mux = self.get_mux()?;
        let panes = mux.iter_panes();

        let sessions: Vec<Value> = panes
            .iter()
            .map(|pane| {
                let dims = pane.get_dimensions();
                let cursor = pane.get_cursor_position();
                let is_dead = pane.is_dead();
                let shell = Self::detect_shell(pane);

                json!({
                    "id": pane.pane_id(),
                    "title": pane.get_title(),
                    "cols": dims.cols,
                    "rows": dims.viewport_rows,
                    "cursor": {
                        "x": cursor.x,
                        "y": cursor.y,
                        "visible": cursor.visibility == termwiz::surface::CursorVisibility::Visible,
                    },
                    "is_dead": is_dead,
                    "domain_id": pane.domain_id(),
                    "shell": shell,
                })
            })
            .collect();

        Ok(json!({ "sessions": sessions }))
    }

    fn session_get(&self, params: &Value) -> Result<Value> {
        let pane = self.get_pane(params)?;
        let dims = pane.get_dimensions();
        let cursor = pane.get_cursor_position();
        let shell = Self::detect_shell(&pane);

        Ok(json!({
            "id": pane.pane_id(),
            "title": pane.get_title(),
            "cols": dims.cols,
            "rows": dims.viewport_rows,
            "scrollback_rows": dims.scrollback_rows,
            "cursor": {
                "x": cursor.x,
                "y": cursor.y,
                "visible": cursor.visibility == termwiz::surface::CursorVisibility::Visible,
            },
            "is_dead": pane.is_dead(),
            "domain_id": pane.domain_id(),
            "shell": shell,
        }))
    }

    fn session_create(&self, params: &Value) -> Result<Value> {
        let cols = params.get("cols").and_then(|v| v.as_u64()).unwrap_or(120) as usize;
        let rows = params.get("rows").and_then(|v| v.as_u64()).unwrap_or(30) as usize;
        let command_dir = params
            .get("cwd")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let size = wezterm_term::TerminalSize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
            dpi: 0,
        };

        // Use a channel to get the async result back to this sync context.
        // Two-level spawn pattern (same as wezterm-mux-server-impl) because
        // domain.spawn() returns non-Send futures.
        let (tx, rx) = std::sync::mpsc::channel();

        promise::spawn::spawn_into_main_thread(async move {
            promise::spawn::spawn(async move {
                let result = async {
                    let mux = Mux::get();
                    let window_id = mux
                        .iter_windows()
                        .into_iter()
                        .next()
                        .ok_or_else(|| anyhow!("No windows available"))?;

                    let (_tab, pane, _wid) = mux
                        .spawn_tab_or_window(
                            Some(window_id),
                            SpawnTabDomain::DefaultDomain,
                            None, // default shell
                            command_dir,
                            size,
                            None,
                            String::new(),
                            None,
                        )
                        .await
                        .context("spawn_tab_or_window")?;

                    let dims = pane.get_dimensions();
                    let pid = pane.pane_id();
                    Ok::<Value, anyhow::Error>(json!({
                        "id": pid,
                        "session_id": pid.to_string(),
                        "title": pane.get_title(),
                        "cols": dims.cols,
                        "rows": dims.viewport_rows,
                    }))
                }
                .await;
                tx.send(result).ok();
            })
            .detach();
        })
        .detach();

        // Wait for the spawn to complete (up to 10 seconds)
        let result = rx
            .recv_timeout(std::time::Duration::from_secs(10))
            .map_err(|_| anyhow!("Timeout waiting for session creation"))?;

        result
    }

    fn session_input(&self, params: &Value) -> Result<Value> {
        let pane = self.get_pane(params)?;
        let input = params
            .get("input")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing 'input' parameter"))?;

        pane.writer().write_all(input.as_bytes())?;
        Ok(json!({"status": "ok"}))
    }

    fn session_resize(&self, params: &Value) -> Result<Value> {
        let pane = self.get_pane(params)?;
        let cols = params
            .get("cols")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| anyhow!("Missing 'cols'"))? as usize;
        let rows = params
            .get("rows")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| anyhow!("Missing 'rows'"))? as usize;

        let size = wezterm_term::TerminalSize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
            dpi: 0,
        };
        pane.resize(size)?;
        Ok(json!({"status": "ok"}))
    }

    fn session_destroy(&self, params: &Value) -> Result<Value> {
        let pane = self.get_pane(params)?;
        self.audit(
            "session.destroy",
            Some(&pane.pane_id().to_string()),
            "destroy",
        );
        pane.kill();
        Ok(json!({"status": "ok", "destroyed": true}))
    }

    fn session_idle(&self, params: &Value) -> Result<Value> {
        let pane = self.get_pane(params)?;
        // Heuristic: check if foreground process is the shell itself
        let fg = pane
            .get_foreground_process_name(mux::pane::CachePolicy::AllowStale)
            .unwrap_or_default();
        let name = fg.rsplit(['/', '\\']).next().unwrap_or("").to_lowercase();
        let is_shell = name.contains("powershell")
            || name.contains("pwsh")
            || name.contains("cmd")
            || name.contains("bash")
            || name.contains("zsh")
            || name.contains("fish")
            || name.contains("nu");
        Ok(json!({"idle": is_shell, "foreground_process": fg}))
    }

    fn session_cwd(&self, params: &Value) -> Result<Value> {
        let pane = self.get_pane(params)?;
        let cwd = pane
            .get_current_working_dir(mux::pane::CachePolicy::AllowStale)
            .map(|u| u.to_string())
            .unwrap_or_default();
        Ok(json!({"cwd": cwd}))
    }

    fn session_env(&self, _params: &Value) -> Result<Value> {
        // WezTerm doesn't expose per-pane env vars directly
        Ok(
            json!({"value": null, "message": "Environment variable reading not supported in WezTerm mode"}),
        )
    }

    fn session_set_env(&self, _params: &Value) -> Result<Value> {
        Ok(
            json!({"status": "ok", "message": "Environment variable setting not supported in WezTerm mode"}),
        )
    }

    fn session_history(&self, params: &Value) -> Result<Value> {
        // Return scrollback as "history"
        let pane = self.get_pane(params)?;
        let dims = pane.get_dimensions();
        let limit = params.get("limit").and_then(|v| v.as_u64()).unwrap_or(100) as usize;

        let end = dims.physical_top;
        let start = (end - limit as isize).max(0);
        let (_first, lines) = pane.get_lines(start..end);

        let entries: Vec<Value> = lines
            .iter()
            .map(|line| {
                let text = line.as_str().trim_end().to_string();
                json!({"text": text})
            })
            .filter(|v| !v["text"].as_str().unwrap_or("").is_empty())
            .collect();

        Ok(json!({"entries": entries, "count": entries.len()}))
    }

    fn session_audit_log(&self, params: &Value) -> Result<Value> {
        let limit = params.get("limit").and_then(|v| v.as_u64()).unwrap_or(50) as usize;
        let session_filter = params.get("session_id").and_then(|v| v.as_str());
        let state = mcp_state().lock();
        let entries: Vec<_> = state
            .audit_log
            .iter()
            .rev()
            .filter(|e| session_filter.map_or(true, |sid| e.session_id.as_deref() == Some(sid)))
            .take(limit)
            .cloned()
            .collect();
        Ok(json!(entries))
    }

    fn audit(&self, method: &str, session_id: Option<&str>, detail: &str) {
        let entry = AuditEntry {
            timestamp: chrono::Local::now().to_rfc3339(),
            method: method.to_string(),
            session_id: session_id.map(|s| s.to_string()),
            detail: detail.to_string(),
            allowed: true,
        };
        mcp_state().lock().audit_log.push(entry);
    }

    // --- Exec methods ---

    fn exec_run(&self, params: &Value) -> Result<Value> {
        let pane = self.get_pane(params)?;
        let command = params
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing 'command'"))?;

        // Policy check
        if let Err(e) = self.check_policy_internal(command) {
            return Err(e);
        }

        self.audit("exec.run", Some(&pane.pane_id().to_string()), command);

        // Send command with newline
        let input = format!("{}\r", command);
        pane.writer().write_all(input.as_bytes())?;
        Ok(json!({"sent": true}))
    }

    fn exec_run_wait(&self, params: &Value) -> Result<Value> {
        let pane = self.get_pane(params)?;
        let command = params
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing 'command'"))?;
        let timeout_ms = params
            .get("timeout_ms")
            .and_then(|v| v.as_u64())
            .unwrap_or(30000);

        if let Err(e) = self.check_policy_internal(command) {
            return Err(e);
        }

        self.audit("exec.run_wait", Some(&pane.pane_id().to_string()), command);

        let marker = format!("__UNTERM_DONE_{}__", uuid::Uuid::new_v4().simple());
        let shell = Self::detect_shell(&pane);
        let shell_type = shell["shell_type"].as_str().unwrap_or("unknown");
        let wait_command = wait_wrapped_command(command, shell_type, &marker);

        // Capture screen before
        let before_text = self.read_pane_text(&pane);

        // Send command
        let input = format!("{}\r", wait_command);
        pane.writer().write_all(input.as_bytes())?;

        // Poll until the injected sentinel is rendered. This gives CLI/MCP
        // automation a deterministic completion condition across shells.
        let start = std::time::Instant::now();
        let timeout = std::time::Duration::from_millis(timeout_ms);

        loop {
            std::thread::sleep(std::time::Duration::from_millis(200));

            let current_text = self.read_pane_text(&pane);
            if current_text.contains(&marker) {
                std::thread::sleep(std::time::Duration::from_millis(200));
                let final_text = self.read_pane_text(&pane);
                let output = extract_wait_output(&before_text, &final_text, command, &marker);
                return Ok(json!({
                    "output": output,
                    "exit_status": "completed",
                    "timed_out": false,
                    "marker": marker,
                }));
            }

            if start.elapsed() > timeout {
                let current_text = self.read_pane_text(&pane);
                let output = extract_wait_output(&before_text, &current_text, command, &marker);
                return Ok(json!({
                    "output": output,
                    "exit_status": "timeout",
                    "timed_out": true,
                    "marker": marker,
                }));
            }
        }
    }

    fn exec_status(&self, params: &Value) -> Result<Value> {
        let pane = self.get_pane(params)?;
        let fg = pane
            .get_foreground_process_name(mux::pane::CachePolicy::AllowStale)
            .unwrap_or_default();
        let name = fg.rsplit(['/', '\\']).next().unwrap_or("").to_lowercase();
        let is_shell = name.contains("powershell")
            || name.contains("pwsh")
            || name.contains("cmd")
            || name.contains("bash")
            || name.contains("zsh")
            || name.contains("fish");
        let status = if is_shell { "idle" } else { "running" };
        Ok(json!({"status": status, "foreground_process": fg}))
    }

    fn exec_cancel(&self, params: &Value) -> Result<Value> {
        let pane = self.get_pane(params)?;
        self.audit("exec.cancel", Some(&pane.pane_id().to_string()), "Ctrl+C");
        // Send Ctrl+C
        pane.writer().write_all(b"\x03")?;
        Ok(json!({"cancelled": true}))
    }

    // --- Signal ---

    fn signal_send(&self, params: &Value) -> Result<Value> {
        let pane = self.get_pane(params)?;
        let signal = params
            .get("signal")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing 'signal'"))?;

        self.audit("signal.send", Some(&pane.pane_id().to_string()), signal);

        match signal.to_uppercase().as_str() {
            "SIGINT" | "INT" => pane.writer().write_all(b"\x03")?,
            "SIGTSTP" | "TSTP" => pane.writer().write_all(b"\x1a")?,
            "SIGQUIT" | "QUIT" => pane.writer().write_all(b"\x1c")?,
            "EOF" => pane.writer().write_all(b"\x04")?,
            _ => return Err(anyhow!("Unsupported signal: {}", signal)),
        }
        Ok(json!({"sent": true, "signal": signal}))
    }

    // --- Screen extensions ---

    fn screen_scroll(&self, params: &Value) -> Result<Value> {
        let pane = self.get_pane(params)?;
        let offset = params.get("offset").and_then(|v| v.as_u64()).unwrap_or(0) as isize;
        let count = params.get("count").and_then(|v| v.as_u64()).unwrap_or(100) as isize;

        let start = offset;
        let end = offset + count;
        let (_first, lines) = pane.get_lines(start..end);

        let text_lines: Vec<String> = lines
            .iter()
            .map(|line| line.as_str().trim_end().to_string())
            .collect();

        Ok(json!({"lines": text_lines, "offset": offset, "count": text_lines.len()}))
    }

    fn screen_search(&self, params: &Value) -> Result<Value> {
        let pane = self.get_pane(params)?;
        let pattern = params
            .get("pattern")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing 'pattern'"))?;
        let max_results = params
            .get("max_results")
            .and_then(|v| v.as_u64())
            .unwrap_or(50) as usize;

        let dims = pane.get_dimensions();
        let start = 0isize;
        let end = dims.physical_top + dims.viewport_rows as isize;
        let (_first, lines) = pane.get_lines(start..end);

        let mut matches: Vec<Value> = Vec::new();
        for (row_idx, line) in lines.iter().enumerate() {
            let text = line.as_str().to_string();
            if text.contains(pattern) {
                matches.push(json!({
                    "row": row_idx,
                    "text": text.trim_end(),
                }));
                if matches.len() >= max_results {
                    break;
                }
            }
        }

        Ok(json!({"matches": matches, "total": matches.len()}))
    }

    // --- Orchestrate ---

    fn orchestrate_launch(&self, params: &Value) -> Result<Value> {
        // Create a new tab and run the command
        let result = self.session_create(params)?;
        let id = result.get("id").and_then(|v| v.as_u64());
        if let Some(pane_id) = id {
            if let Some(command) = params.get("command").and_then(|v| v.as_str()) {
                // Brief delay to let shell initialize
                std::thread::sleep(std::time::Duration::from_millis(500));
                if let Ok(mux) = self.get_mux() {
                    if let Some(pane) = mux.get_pane(pane_id as usize) {
                        let input = format!("{}\r", command);
                        let _ = pane.writer().write_all(input.as_bytes());
                    }
                }
            }
        }
        Ok(result)
    }

    fn orchestrate_broadcast(&self, params: &Value) -> Result<Value> {
        let command = params
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing 'command'"))?;
        let sessions = params
            .get("sessions")
            .and_then(|v| v.as_array())
            .ok_or_else(|| anyhow!("Missing 'sessions'"))?;

        let mux = self.get_mux()?;
        let mut results = Vec::new();
        let input = format!("{}\r", command);

        for sid in sessions {
            let id_str = sid.as_str().unwrap_or("");
            if let Ok(id) = id_str.parse::<usize>() {
                if let Some(pane) = mux.get_pane(id) {
                    match pane.writer().write_all(input.as_bytes()) {
                        Ok(_) => results.push(json!({"session_id": id_str, "sent": true})),
                        Err(e) => {
                            results.push(json!({"session_id": id_str, "error": e.to_string()}))
                        }
                    }
                } else {
                    results.push(json!({"session_id": id_str, "error": "not found"}));
                }
            }
        }

        Ok(json!({"results": results}))
    }

    fn orchestrate_wait(&self, params: &Value) -> Result<Value> {
        let pane = self.get_pane(params)?;
        let pattern = params
            .get("pattern")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing 'pattern'"))?;
        let timeout_ms = params
            .get("timeout_ms")
            .and_then(|v| v.as_u64())
            .unwrap_or(10000);

        let start = std::time::Instant::now();
        let timeout = std::time::Duration::from_millis(timeout_ms);

        loop {
            let text = self.read_pane_text(&pane);
            if text.contains(pattern) {
                return Ok(json!({"matched": true, "pattern": pattern}));
            }
            if start.elapsed() > timeout {
                return Ok(json!({"matched": false, "timed_out": true, "pattern": pattern}));
            }
            std::thread::sleep(std::time::Duration::from_millis(200));
        }
    }

    // --- Proxy ---

    fn proxy_status(&self) -> Result<Value> {
        let settings = mcp_state().lock().proxy.clone();
        Ok(json!({
            "enabled": settings.enabled,
            "mode": settings.mode,
            "http_proxy": settings.http_proxy,
            "socks_proxy": settings.socks_proxy,
            "no_proxy": settings.no_proxy,
            "current_node": settings.current_node,
            "node_count": settings.nodes.len(),
        }))
    }

    fn proxy_nodes(&self) -> Result<Value> {
        let settings = mcp_state().lock().proxy.clone();
        Ok(json!({
            "current_node": settings.current_node,
            "nodes": settings.nodes,
        }))
    }

    fn proxy_configure(&self, params: &Value) -> Result<Value> {
        let mut state = mcp_state().lock();
        let mut settings = state.proxy.clone();

        let enabled = params
            .get("enabled")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        let mode = params
            .get("mode")
            .and_then(|v| v.as_str())
            .unwrap_or("manual")
            .to_string();

        settings.enabled = enabled;
        settings.mode = if enabled { mode } else { "off".to_string() };

        if let Some(http) = params.get("http_proxy").and_then(|v| v.as_str()) {
            settings.http_proxy = Some(http.to_string());
        }
        if let Some(socks) = params.get("socks_proxy").and_then(|v| v.as_str()) {
            settings.socks_proxy = Some(socks.to_string());
        }
        if let Some(no_proxy) = params.get("no_proxy").and_then(|v| v.as_str()) {
            settings.no_proxy = no_proxy.to_string();
        }

        if let Some(nodes) = params.get("nodes").and_then(|v| v.as_array()) {
            settings.nodes = nodes
                .iter()
                .filter_map(|node| {
                    let name = node.get("name")?.as_str()?.to_string();
                    let url = node
                        .get("url")
                        .or_else(|| node.get("http_proxy"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    if url.is_empty() {
                        return None;
                    }
                    Some(ProxyNodeConfig {
                        name,
                        url,
                        latency_ms: None,
                        available: true,
                    })
                })
                .collect();
        }

        if let Some(node_name) = params.get("current_node").and_then(|v| v.as_str()) {
            settings.current_node = Some(node_name.to_string());
            if let Some(node) = settings.nodes.iter().find(|node| node.name == node_name) {
                settings.http_proxy = Some(node.url.clone());
            }
        }

        save_proxy_settings(&settings)?;
        state.proxy = settings.clone();
        drop(state);

        Ok(json!({
            "configured": true,
            "status": self.proxy_status()?,
        }))
    }

    fn proxy_disable(&self) -> Result<Value> {
        let mut state = mcp_state().lock();
        let mut settings = state.proxy.clone();
        settings.enabled = false;
        settings.mode = "off".to_string();
        save_proxy_settings(&settings)?;
        state.proxy = settings;
        Ok(json!({"disabled": true}))
    }

    fn proxy_switch(&self, params: &Value) -> Result<Value> {
        let node_name = params
            .get("node_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing 'node_name'"))?;

        let mut state = mcp_state().lock();
        let mut settings = state.proxy.clone();
        let node = settings
            .nodes
            .iter()
            .find(|node| node.name == node_name)
            .cloned()
            .ok_or_else(|| anyhow!("Proxy node '{}' not found", node_name))?;

        settings.enabled = true;
        settings.mode = "manual".to_string();
        settings.current_node = Some(node.name.clone());
        settings.http_proxy = Some(node.url.clone());
        if settings.socks_proxy.is_none() && node.url.starts_with("socks") {
            settings.socks_proxy = Some(node.url.clone());
        }
        save_proxy_settings(&settings)?;
        state.proxy = settings;

        Ok(json!({
            "switched": true,
            "current_node": node.name,
            "http_proxy": node.url,
        }))
    }

    fn proxy_env(&self) -> Result<Value> {
        let settings = mcp_state().lock().proxy.clone();
        let mut env = serde_json::Map::new();
        if settings.enabled {
            if let Some(http) = settings.http_proxy {
                env.insert("HTTP_PROXY".to_string(), json!(http));
                env.insert("HTTPS_PROXY".to_string(), json!(http));
            }
            if let Some(socks) = settings.socks_proxy {
                env.insert("ALL_PROXY".to_string(), json!(socks));
            }
            env.insert("NO_PROXY".to_string(), json!(settings.no_proxy));
        }
        Ok(json!({
            "enabled": settings.enabled,
            "env": env,
        }))
    }

    fn proxy_speedtest(&self, params: &Value) -> Result<Value> {
        let target_name = params.get("node_name").and_then(|v| v.as_str());
        let timeout_ms = params
            .get("timeout_ms")
            .and_then(|v| v.as_u64())
            .unwrap_or(3000);

        let mut state = mcp_state().lock();
        let mut settings = state.proxy.clone();
        let mut results = Vec::new();

        for node in &mut settings.nodes {
            if target_name.map_or(false, |name| node.name != name) {
                continue;
            }
            let start = std::time::Instant::now();
            let available = probe_proxy_endpoint(&node.url, timeout_ms);
            node.available = available;
            node.latency_ms = if available {
                Some(start.elapsed().as_millis() as u64)
            } else {
                None
            };
            results.push(json!({
                "name": node.name,
                "url": node.url,
                "available": node.available,
                "latency_ms": node.latency_ms,
            }));
        }

        if results.is_empty() && target_name.is_some() {
            return Err(anyhow!("Proxy node '{}' not found", target_name.unwrap()));
        }

        save_proxy_settings(&settings)?;
        state.proxy = settings;
        Ok(json!({"results": results}))
    }

    // --- Workspace ---

    fn workspace_save(&self, params: &Value) -> Result<Value> {
        let name = params
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing 'name'"))?;

        let mux = self.get_mux()?;
        let panes = mux.iter_panes();
        let sessions: Vec<Value> = panes
            .iter()
            .map(|pane| {
                let cwd = pane
                    .get_current_working_dir(mux::pane::CachePolicy::AllowStale)
                    .map(|u| u.to_string());
                json!({
                    "id": pane.pane_id(),
                    "title": pane.get_title(),
                    "cwd": cwd,
                })
            })
            .collect();

        let workspace = json!({
            "name": name,
            "sessions": sessions,
            "saved_at": chrono::Local::now().to_rfc3339(),
        });

        // Save to ~/.unterm/workspaces/<name>.json
        let dir = dirs_next::home_dir()
            .unwrap_or_default()
            .join(".unterm")
            .join("workspaces");
        std::fs::create_dir_all(&dir)?;
        let path = dir.join(format!("{}.json", name));
        std::fs::write(&path, serde_json::to_string_pretty(&workspace)?)?;

        Ok(json!({"saved": true, "name": name, "sessions": sessions.len()}))
    }

    fn workspace_restore(&self, params: &Value) -> Result<Value> {
        let name = params
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing 'name'"))?;

        let path = dirs_next::home_dir()
            .unwrap_or_default()
            .join(".unterm")
            .join("workspaces")
            .join(format!("{}.json", name));

        if !path.exists() {
            return Err(anyhow!("Workspace '{}' not found", name));
        }

        let data = std::fs::read_to_string(&path)?;
        let workspace: Value = serde_json::from_str(&data)?;

        // For now just return the saved info — full restore would need
        // to create tabs with specified cwds
        Ok(json!({
            "restored": true,
            "name": name,
            "workspace": workspace,
            "message": "Workspace data loaded. Use session.create with cwd to recreate sessions.",
        }))
    }

    fn workspace_list(&self) -> Result<Value> {
        let dir = dirs_next::home_dir()
            .unwrap_or_default()
            .join(".unterm")
            .join("workspaces");

        let mut workspaces = Vec::new();
        if dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().map_or(false, |e| e == "json") {
                        let name = path
                            .file_stem()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string();
                        workspaces.push(json!({"name": name}));
                    }
                }
            }
        }

        Ok(json!({"workspaces": workspaces}))
    }

    // --- Capture ---

    fn capture_screen(&self, params: &Value) -> Result<Value> {
        let mux = self.get_mux()?;
        let panes = mux.iter_panes();
        let mut captures = Vec::new();

        for pane in &panes {
            let text = self.read_pane_text(pane);
            captures.push(json!({
                "session_id": pane.pane_id().to_string(),
                "title": pane.get_title(),
                "screen": text,
                "type": "text",
            }));
        }

        let include_base64 = params
            .get("include_base64")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let image = capture_screen_image(include_base64)?;
        Ok(json!({
            "captures": captures,
            "image": image,
            "type": "image/png",
            "text_snapshot": true,
        }))
    }

    fn capture_window(&self, params: &Value) -> Result<Value> {
        let title_filter = params.get("title").and_then(|v| v.as_str());
        let pid_filter = params.get("pid").and_then(|v| v.as_u64()).map(|v| v as u32);
        let mux = self.get_mux()?;
        let panes = mux.iter_panes();

        let include_base64 = params
            .get("include_base64")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let image = capture_window_image(title_filter, pid_filter, include_base64)?;

        for pane in &panes {
            let pane_title = pane.get_title();
            let matches = title_filter.map_or(true, |t| {
                pane_title.contains(t) || pane.pane_id().to_string().contains(t)
            });
            if matches {
                let text = self.read_pane_text(pane);
                return Ok(json!({
                    "session_id": pane.pane_id().to_string(),
                    "title": pane_title,
                    "screen": text,
                    "image": image,
                    "type": "image/png",
                    "text_snapshot": true,
                }));
            }
        }

        Ok(json!({
            "image": image,
            "type": "image/png",
            "text_snapshot": false,
        }))
    }

    fn capture_select(&self) -> Result<Value> {
        let image = capture_screen_image(false)?;
        Ok(json!({
            "image": image,
            "type": "image/png",
            "mode": "screen_fallback",
            "message": "Interactive region selection is not available in headless MCP mode; captured the screen instead.",
        }))
    }

    fn capture_clipboard(&self) -> Result<Value> {
        #[cfg(windows)]
        {
            return clipboard_read_win32();
        }
        #[cfg(not(windows))]
        {
            Err(anyhow!("Clipboard reading only supported on Windows"))
        }
    }

    // --- Policy ---

    fn policy_set(&self, params: &Value) -> Result<Value> {
        let policy: CommandPolicy =
            serde_json::from_value(params.clone()).map_err(|e| anyhow!("Invalid policy: {}", e))?;
        self.audit("policy.set", None, &format!("enabled={}", policy.enabled));
        mcp_state().lock().policy = policy;
        Ok(json!({"set": true}))
    }

    fn policy_check(&self, params: &Value) -> Result<Value> {
        let command = params
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing 'command'"))?;

        let state = mcp_state().lock();
        if !state.policy.enabled {
            return Ok(json!({"allowed": true, "reason": "Policy disabled"}));
        }

        for pattern in &state.policy.blocked_patterns {
            if command.contains(pattern) {
                return Ok(json!({
                    "allowed": false,
                    "reason": format!("Blocked by pattern: {}", pattern),
                }));
            }
        }

        Ok(json!({"allowed": true}))
    }

    fn check_policy_internal(&self, command: &str) -> Result<()> {
        let state = mcp_state().lock();
        if !state.policy.enabled {
            return Ok(());
        }
        for pattern in &state.policy.blocked_patterns {
            if command.contains(pattern) {
                return Err(anyhow!("Command blocked by policy: {}", pattern));
            }
        }
        Ok(())
    }

    // --- System ---

    fn system_info(&self) -> Result<Value> {
        let mux = self.get_mux()?;
        let pane_count = mux.iter_panes().len();
        Ok(json!({
            "name": "Unterm",
            "version": "2.0.0",
            "engine": "Unterm (WezTerm)",
            "platform": std::env::consts::OS,
            "arch": std::env::consts::ARCH,
            "active_sessions": pane_count,
            "hostname": hostname::get()
                .map(|h| h.to_string_lossy().to_string())
                .unwrap_or_default(),
        }))
    }

    fn system_launch_admin(&self, params: &Value) -> Result<Value> {
        #[cfg(windows)]
        {
            let dry_run = params
                .get("dry_run")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let shell = params
                .get("shell")
                .and_then(|v| v.as_str())
                .unwrap_or("pwsh");
            let args = elevated_unterm_command_args(shell)?;

            if !dry_run {
                std::process::Command::new(&args[0])
                    .args(&args[1..])
                    .spawn()
                    .context("launch elevated Unterm window via PowerShell RunAs")?;
            }

            Ok(json!({
                "status": if dry_run { "dry_run" } else { "launched" },
                "requires_uac": true,
                "command": args,
            }))
        }

        #[cfg(not(windows))]
        {
            let _ = params;
            Err(anyhow!("Administrator launch is only supported on Windows"))
        }
    }

    // --- Helpers ---

    fn read_pane_text(&self, pane: &Arc<dyn Pane>) -> String {
        let dims = pane.get_dimensions();
        let first_row = dims.physical_top;
        let last_row = first_row + dims.viewport_rows as isize;
        let (_first, lines) = pane.get_lines(first_row..last_row);
        lines
            .iter()
            .map(|line| line.as_str().trim_end().to_string())
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn screen_read(&self, params: &Value) -> Result<Value> {
        let pane = self.get_pane(params)?;
        let dims = pane.get_dimensions();
        let cursor = pane.get_cursor_position();

        // Read visible lines
        let first_row = dims.physical_top;
        let last_row = first_row + dims.viewport_rows as isize;
        let (first, lines) = pane.get_lines(first_row..last_row);

        let cells: Vec<Value> = lines
            .iter()
            .enumerate()
            .map(|(row_idx, line)| {
                let text = line.as_str().to_string();
                let text = text.trim_end().to_string();
                json!({
                    "row": first as i64 + row_idx as i64,
                    "text": text,
                })
            })
            .collect();

        Ok(json!({
            "cells": cells,
            "cursor": {
                "x": cursor.x,
                "y": cursor.y,
                "visible": cursor.visibility == termwiz::surface::CursorVisibility::Visible,
            },
            "cols": dims.cols,
            "rows": dims.viewport_rows,
            "scrollback_rows": dims.scrollback_rows,
        }))
    }

    fn screen_text(&self, params: &Value) -> Result<Value> {
        let pane = self.get_pane(params)?;
        let dims = pane.get_dimensions();
        let cursor = pane.get_cursor_position();

        let first_row = dims.physical_top;
        let last_row = first_row + dims.viewport_rows as isize;
        let (_first, lines) = pane.get_lines(first_row..last_row);

        let text_lines: Vec<String> = lines
            .iter()
            .map(|line| line.as_str().trim_end().to_string())
            .collect();

        Ok(json!({
            "lines": text_lines,
            "cursor": { "x": cursor.x, "y": cursor.y },
            "cols": dims.cols,
            "rows": dims.viewport_rows,
        }))
    }

    fn screen_cursor(&self, params: &Value) -> Result<Value> {
        let pane = self.get_pane(params)?;
        let cursor = pane.get_cursor_position();

        Ok(json!({
            "x": cursor.x,
            "y": cursor.y,
            "visible": cursor.visibility == termwiz::surface::CursorVisibility::Visible,
            "shape": format!("{:?}", cursor.shape),
        }))
    }

    fn ghost_text_set(&self, params: &Value) -> Result<Value> {
        let pane = self.get_pane(params)?;
        let text = params
            .get("text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing 'text' parameter"))?
            .to_string();

        let cursor = pane.get_cursor_position();

        ghost_text_state().set(GhostText {
            pane_id: pane.pane_id(),
            text,
            cursor_x: cursor.x,
            cursor_y: cursor.y as i64,
        });

        Ok(json!({"status": "ok"}))
    }

    fn ghost_text_clear(&self, _params: &Value) -> Result<Value> {
        ghost_text_state().clear();
        Ok(json!({"status": "ok"}))
    }

    fn screen_detect_errors(&self, params: &Value) -> Result<Value> {
        let pane = self.get_pane(params)?;
        let dims = pane.get_dimensions();

        let first_row = dims.physical_top;
        let last_row = first_row + dims.viewport_rows as isize;
        let (_first, lines) = pane.get_lines(first_row..last_row);

        let error_patterns = [
            "error:",
            "Error:",
            "ERROR:",
            "error[",
            "fatal:",
            "Fatal:",
            "FATAL:",
            "panic:",
            "PANIC:",
            "not found",
            "command not found",
            "Permission denied",
            "permission denied",
            "No such file or directory",
            "failed",
            "FAILED",
            "traceback",
            "Traceback",
            "Exception",
            "exception:",
            "segfault",
            "Segmentation fault",
        ];

        let mut errors: Vec<Value> = Vec::new();

        for (row_idx, line) in lines.iter().enumerate() {
            let text = line.as_str().to_string();
            let trimmed = text.trim();
            if trimmed.is_empty() {
                continue;
            }
            for pattern in &error_patterns {
                if trimmed.contains(pattern) {
                    errors.push(json!({
                        "row": first_row as i64 + row_idx as i64,
                        "text": trimmed,
                        "pattern": pattern,
                    }));
                    break;
                }
            }
        }

        Ok(json!({
            "has_errors": !errors.is_empty(),
            "errors": errors,
        }))
    }

    // --- AI Methods ---

    fn get_ai_config(&self) -> (ModelProvider, String, String) {
        let config = config::configuration();
        let model_name = ai_state().active_model();
        let provider = ModelProvider::from_name(&model_name);
        let (api_key, model_id) = match &provider {
            ModelProvider::Claude => (
                config.ai_claude_api_key.clone(),
                config.ai_claude_model.clone(),
            ),
            ModelProvider::OpenAI => (
                config.ai_openai_api_key.clone(),
                config.ai_openai_model.clone(),
            ),
            ModelProvider::Gemini => (
                config.ai_gemini_api_key.clone(),
                config.ai_gemini_model.clone(),
            ),
            ModelProvider::Custom => (String::new(), String::new()),
        };
        (provider, api_key, model_id)
    }

    fn ai_complete(&self, params: &Value) -> Result<Value> {
        let prompt = params
            .get("prompt")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing 'prompt'"))?;
        let system = params
            .get("system")
            .and_then(|v| v.as_str())
            .unwrap_or("You are a helpful assistant.");

        let (provider, api_key, model_id) = self.get_ai_config();
        let response = ai_client::complete(&provider, &api_key, &model_id, system, prompt)?;
        Ok(json!({"response": response}))
    }

    fn ai_chat(&self, params: &Value) -> Result<Value> {
        let pane = self.get_pane(params)?;
        let message = params
            .get("message")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing 'message'"))?;

        let screen_context = self.read_pane_text(&pane);
        let shell = Self::detect_shell(&pane);
        let shell_type = shell["shell_type"].as_str().unwrap_or("unknown");

        // Add user message to chat history
        ai_state().add_chat_message(ChatMessage {
            role: ChatRole::User,
            content: message.to_string(),
        });

        let (provider, api_key, model_id) = self.get_ai_config();
        let response = ai_client::chat(
            &provider,
            &api_key,
            &model_id,
            shell_type,
            &screen_context,
            message,
        )?;

        // Add AI response to chat history
        ai_state().add_chat_message(ChatMessage {
            role: ChatRole::Assistant,
            content: response.clone(),
        });

        // Store as insight card
        ai_state().set_insight(InsightCard {
            title: "AI Chat".to_string(),
            content: response.clone(),
            card_type: InsightType::Chat,
            command: None,
        });

        Ok(json!({"response": response}))
    }

    fn ai_analyze_error(&self, params: &Value) -> Result<Value> {
        let pane = self.get_pane(params)?;
        let screen_context = self.read_pane_text(&pane);
        let shell = Self::detect_shell(&pane);
        let shell_type = shell["shell_type"].as_str().unwrap_or("unknown");

        let (provider, api_key, model_id) = self.get_ai_config();
        let (explanation, fix_command) =
            ai_client::analyze_error(&provider, &api_key, &model_id, shell_type, &screen_context)?;

        // Store as insight card
        ai_state().set_insight(InsightCard {
            title: "Error Analysis".to_string(),
            content: explanation.clone(),
            card_type: InsightType::Error,
            command: fix_command.clone(),
        });

        Ok(json!({
            "explanation": explanation,
            "fix_command": fix_command,
        }))
    }

    fn ai_suggest(&self, params: &Value) -> Result<Value> {
        let pane = self.get_pane(params)?;
        let partial = params.get("partial").and_then(|v| v.as_str()).unwrap_or("");

        let screen_context = self.read_pane_text(&pane);
        let shell = Self::detect_shell(&pane);
        let shell_type = shell["shell_type"].as_str().unwrap_or("unknown");
        let cwd = shell["cwd"].as_str().unwrap_or("~");

        let (provider, api_key, model_id) = self.get_ai_config();
        let suggestion = ai_client::ghost_text_complete(
            &provider,
            &api_key,
            &model_id,
            shell_type,
            cwd,
            &screen_context,
            partial,
        )?;

        let suggestion = suggestion.trim().to_string();
        if !suggestion.is_empty() {
            let cursor = pane.get_cursor_position();
            ghost_text_state().set(GhostText {
                pane_id: pane.pane_id(),
                text: suggestion.clone(),
                cursor_x: cursor.x,
                cursor_y: cursor.y as i64,
            });
        }

        Ok(json!({"suggestion": suggestion}))
    }

    fn ai_suggest_next(&self, params: &Value) -> Result<Value> {
        let pane = self.get_pane(params)?;
        let screen_context = self.read_pane_text(&pane);
        let shell = Self::detect_shell(&pane);
        let shell_type = shell["shell_type"].as_str().unwrap_or("unknown");

        let (provider, api_key, model_id) = self.get_ai_config();
        let (suggestion, command) = ai_client::suggest_next_step(
            &provider,
            &api_key,
            &model_id,
            shell_type,
            &screen_context,
        )?;

        // Store as insight card
        ai_state().set_insight(InsightCard {
            title: "Next Step".to_string(),
            content: suggestion.clone(),
            card_type: InsightType::Suggestion,
            command: command.clone(),
        });

        Ok(json!({
            "suggestion": suggestion,
            "command": command,
        }))
    }

    fn ai_set_model(&self, params: &Value) -> Result<Value> {
        let model = params
            .get("model")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing 'model'"))?;

        ai_state().set_model(model);
        Ok(json!({
            "status": "ok",
            "model": model,
            "provider": format!("{:?}", ModelProvider::from_name(model)),
        }))
    }

    fn ai_get_model(&self) -> Result<Value> {
        let model = ai_state().active_model();
        let provider = ai_state().provider();
        Ok(json!({
            "model": model,
            "provider": format!("{:?}", provider),
            "icon": provider.display_icon(),
        }))
    }

    fn ai_set_insight(&self, params: &Value) -> Result<Value> {
        let title = params
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Insight")
            .to_string();
        let content = params
            .get("content")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let card_type = match params
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("info")
        {
            "error" => InsightType::Error,
            "suggestion" => InsightType::Suggestion,
            "chat" => InsightType::Chat,
            _ => InsightType::Info,
        };
        let command = params
            .get("command")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        ai_state().set_insight(InsightCard {
            title,
            content,
            card_type,
            command,
        });
        Ok(json!({"status": "ok"}))
    }

    fn ai_toggle_panel(&self) -> Result<Value> {
        let visible = ai_state().toggle_panel();
        Ok(json!({
            "visible": visible,
        }))
    }

    fn ai_focus_chat(&self, params: &Value) -> Result<Value> {
        let focused = params
            .get("focused")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        let state = ai_state();
        // Ensure panel is visible when focusing chat
        if focused && !state.panel_visible() {
            state.set_panel_visible(true);
        }
        state.set_chat_focused(focused);
        Ok(json!({
            "focused": focused,
            "panel_visible": state.panel_visible(),
        }))
    }

    fn ai_send_chat_input(&self, params: &Value) -> Result<Value> {
        let text = params
            .get("text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing 'text' parameter"))?;
        let state = ai_state();
        // Ensure panel is visible and chat is focused
        if !state.panel_visible() {
            state.set_panel_visible(true);
        }
        state.set_chat_focused(true);
        for c in text.chars() {
            state.chat_input_push(c);
        }
        Ok(json!({
            "input": state.chat_input(),
            "focused": true,
        }))
    }

    fn ai_panel_state(&self) -> Result<Value> {
        let state = ai_state();
        Ok(json!({
            "panel_visible": state.panel_visible(),
            "chat_focused": state.chat_focused(),
            "chat_input": state.chat_input(),
            "model": state.active_model(),
            "provider": state.provider().display_name(),
            "has_insight": state.get_insight().is_some(),
            "chat_history_count": state.chat_history().len(),
        }))
    }

    fn selftest_run(&self, params: &Value) -> Result<Value> {
        let mut checks: Vec<Value> = Vec::new();

        let mux_available = Mux::try_get().is_some();
        checks.push(json!({
            "name": "mux.available",
            "ok": mux_available,
            "detail": if mux_available { "Mux is available" } else { "Mux is not initialized" },
        }));

        let health = self.server_health()?;
        checks.push(json!({
            "name": "server.health",
            "ok": health["status"] == "ok",
            "detail": health,
        }));

        let caps = self.server_capabilities()?;
        let has_ai = caps
            .get("ai")
            .and_then(|v| v.as_array())
            .is_some_and(|v| !v.is_empty());
        let has_screen = caps
            .get("screen")
            .and_then(|v| v.as_array())
            .is_some_and(|v| !v.is_empty());
        checks.push(json!({
            "name": "server.capabilities",
            "ok": has_ai && has_screen,
            "detail": {
                "has_ai": has_ai,
                "has_screen": has_screen,
            },
        }));

        let ai_panel = self.ai_panel_state()?;
        checks.push(json!({
            "name": "ai.panel_state",
            "ok": ai_panel.get("model").and_then(|v| v.as_str()).is_some(),
            "detail": ai_panel,
        }));

        let policy = self.policy_check(&json!({"command": "echo unterm-selftest"}));
        checks.push(json!({
            "name": "policy.check",
            "ok": policy.is_ok(),
            "detail": match policy {
                Ok(value) => value,
                Err(err) => json!({"error": err.to_string()}),
            },
        }));

        let admin = self.system_launch_admin(&json!({"dry_run": true, "shell": "pwsh"}));
        checks.push(json!({
            "name": "system.launch_admin",
            "ok": admin.is_ok(),
            "detail": match admin {
                Ok(value) => value,
                Err(err) => json!({"error": err.to_string()}),
            },
        }));

        let proxy = self.proxy_status();
        checks.push(json!({
            "name": "proxy.status",
            "ok": proxy.is_ok(),
            "detail": match proxy {
                Ok(value) => value,
                Err(err) => json!({"error": err.to_string()}),
            },
        }));

        let capture = self.capture_window(&json!({"pid": std::process::id()}));
        checks.push(json!({
            "name": "capture.window",
            "ok": capture
                .as_ref()
                .ok()
                .and_then(|value| value.pointer("/image/path"))
                .and_then(|value| value.as_str())
                .map(|path| std::path::Path::new(path).exists())
                .unwrap_or(false),
            "detail": match capture {
                Ok(value) => value,
                Err(err) => json!({"error": err.to_string()}),
            },
        }));

        if let Some(session_id) = params.get("session_id").and_then(|v| v.as_str()) {
            let session_params = json!({ "session_id": session_id });

            let session = self.session_get(&session_params);
            checks.push(json!({
                "name": "session.status",
                "ok": session.is_ok(),
                "detail": match session {
                    Ok(value) => value,
                    Err(err) => json!({"error": err.to_string()}),
                },
            }));

            let screen = self.screen_text(&session_params);
            checks.push(json!({
                "name": "screen.text",
                "ok": screen.is_ok(),
                "detail": match screen {
                    Ok(value) => value,
                    Err(err) => json!({"error": err.to_string()}),
                },
            }));

            let detect = self.screen_detect_errors(&session_params);
            checks.push(json!({
                "name": "screen.detect_errors",
                "ok": detect.is_ok(),
                "detail": match detect {
                    Ok(value) => value,
                    Err(err) => json!({"error": err.to_string()}),
                },
            }));
        }

        let ok = checks
            .iter()
            .all(|check| check.get("ok").and_then(|v| v.as_bool()).unwrap_or(false));

        Ok(json!({
            "ok": ok,
            "checks": checks,
        }))
    }
}

fn proxy_config_path() -> std::path::PathBuf {
    dirs_next::home_dir()
        .unwrap_or_default()
        .join(".unterm")
        .join("proxy.json")
}

fn load_proxy_settings() -> ProxySettings {
    let path = proxy_config_path();
    match std::fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => ProxySettings::default(),
    }
}

fn save_proxy_settings(settings: &ProxySettings) -> Result<()> {
    let path = proxy_config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, serde_json::to_string_pretty(settings)?)?;
    Ok(())
}

fn probe_proxy_endpoint(url: &str, timeout_ms: u64) -> bool {
    let Some(rest) = url.split("://").nth(1) else {
        return false;
    };
    let host_port = rest
        .split(['/', '?', '#'])
        .next()
        .unwrap_or(rest)
        .rsplit('@')
        .next()
        .unwrap_or(rest);
    let mut parts = host_port.rsplitn(2, ':');
    let Some(port) = parts.next().and_then(|p| p.parse::<u16>().ok()) else {
        return false;
    };
    let host = parts.next().unwrap_or("127.0.0.1");
    let Ok(addrs) = (host, port).to_socket_addrs() else {
        return false;
    };
    let timeout = std::time::Duration::from_millis(timeout_ms);
    addrs
        .into_iter()
        .any(|addr| std::net::TcpStream::connect_timeout(&addr, timeout).is_ok())
}

#[cfg(windows)]
fn capture_output_dir() -> Result<std::path::PathBuf> {
    let dir = dirs_next::home_dir()
        .unwrap_or_default()
        .join(".unterm")
        .join("screenshots");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Read clipboard content using Win32 API.
/// Supports both text (CF_UNICODETEXT) and image (CF_DIB) formats.
/// For images: reads DIB data, converts BGR → RGBA, encodes PNG,
/// saves to ~/.unterm/clipboard/ and returns the path + dimensions.
/// IMPORTANT: Do NOT use PowerShell for clipboard access — it steals window focus.
#[cfg(windows)]
fn clipboard_read_win32() -> Result<Value> {
    use std::ptr;
    use winapi::um::winuser::{
        CloseClipboard, GetClipboardData, IsClipboardFormatAvailable, OpenClipboard, CF_DIB,
        CF_UNICODETEXT,
    };
    use winapi::um::winbase::GlobalLock;
    use winapi::um::winbase::GlobalUnlock;
    use winapi::um::winbase::GlobalSize;
    use winapi::shared::minwindef::HGLOBAL;
    use winapi::um::wingdi::BITMAPINFOHEADER;

    // Try image first (CF_DIB), then fall back to text (CF_UNICODETEXT)
    let has_image = unsafe { IsClipboardFormatAvailable(CF_DIB as u32) != 0 };
    let has_text = unsafe { IsClipboardFormatAvailable(CF_UNICODETEXT as u32) != 0 };

    if !has_image && !has_text {
        return Err(anyhow!("Clipboard is empty or contains unsupported format"));
    }

    // Open clipboard (pass NULL for current task)
    let opened = unsafe { OpenClipboard(ptr::null_mut()) };
    if opened == 0 {
        return Err(anyhow!("Failed to open clipboard (it may be locked by another application)"));
    }

    // Ensure we close clipboard on all exit paths
    struct ClipboardGuard;
    impl Drop for ClipboardGuard {
        fn drop(&mut self) {
            unsafe { CloseClipboard(); }
        }
    }
    let _guard = ClipboardGuard;

    if has_image {
        let handle: HGLOBAL = unsafe { GetClipboardData(CF_DIB as u32) as HGLOBAL };
        if handle.is_null() {
            if has_text {
                // Fall through to text path below
            } else {
                return Err(anyhow!("GetClipboardData(CF_DIB) returned NULL"));
            }
        } else {
            let ptr = unsafe { GlobalLock(handle) };
            if ptr.is_null() {
                return Err(anyhow!("GlobalLock failed on clipboard DIB data"));
            }

            let data_size = unsafe { GlobalSize(handle) };
            if data_size < std::mem::size_of::<BITMAPINFOHEADER>() {
                unsafe { GlobalUnlock(handle); }
                return Err(anyhow!("Clipboard DIB data too small"));
            }

            // Read BITMAPINFOHEADER
            let bih = unsafe { &*(ptr as *const BITMAPINFOHEADER) };
            let width = bih.biWidth as u32;
            // biHeight can be negative (top-down) or positive (bottom-up)
            let height_signed = bih.biHeight;
            let height = height_signed.unsigned_abs();
            let bit_count = bih.biBitCount;
            let compression = bih.biCompression;

            // We only support uncompressed 24-bit or 32-bit DIBs
            if compression != 0 {
                // BI_RGB = 0; BI_BITFIELDS = 3 for 32-bit is sometimes used
                // For simplicity, only handle BI_RGB
                unsafe { GlobalUnlock(handle); }
                return Err(anyhow!(
                    "Unsupported DIB compression: {}. Only uncompressed (BI_RGB) is supported.",
                    compression
                ));
            }

            if bit_count != 24 && bit_count != 32 {
                unsafe { GlobalUnlock(handle); }
                return Err(anyhow!(
                    "Unsupported DIB bit depth: {}. Only 24-bit and 32-bit are supported.",
                    bit_count
                ));
            }

            let bytes_per_pixel = (bit_count / 8) as usize;
            // DIB rows are padded to 4-byte boundaries
            let row_stride = ((width as usize * bytes_per_pixel + 3) / 4) * 4;

            // Pixel data starts after the header (and color table, but for 24/32-bit there's none with BI_RGB)
            let header_size = bih.biSize as usize;
            let pixel_offset = header_size;

            let total_pixel_bytes = row_stride * height as usize;
            if pixel_offset + total_pixel_bytes > data_size {
                unsafe { GlobalUnlock(handle); }
                return Err(anyhow!("DIB pixel data exceeds clipboard buffer size"));
            }

            let pixel_data = unsafe {
                std::slice::from_raw_parts(
                    (ptr as *const u8).add(pixel_offset),
                    total_pixel_bytes,
                )
            };

            // Convert BGR(A) → RGBA, handling bottom-up vs top-down
            let mut rgba_buf = vec![0u8; (width * height * 4) as usize];
            let bottom_up = height_signed > 0;

            for y in 0..height as usize {
                let src_y = if bottom_up { height as usize - 1 - y } else { y };
                let src_row = &pixel_data[src_y * row_stride..src_y * row_stride + width as usize * bytes_per_pixel];
                let dst_offset = y * width as usize * 4;

                for x in 0..width as usize {
                    let si = x * bytes_per_pixel;
                    let di = dst_offset + x * 4;
                    // DIB stores BGR or BGRA
                    rgba_buf[di] = src_row[si + 2];     // R
                    rgba_buf[di + 1] = src_row[si + 1]; // G
                    rgba_buf[di + 2] = src_row[si];     // B
                    rgba_buf[di + 3] = if bytes_per_pixel == 4 {
                        src_row[si + 3]                  // A
                    } else {
                        255                              // opaque
                    };
                }
            }

            unsafe { GlobalUnlock(handle); }

            // Encode as PNG
            let img = image::RgbaImage::from_raw(width, height, rgba_buf)
                .ok_or_else(|| anyhow!("Failed to create image buffer from DIB data"))?;

            // Save to ~/.unterm/clipboard/
            let clipboard_dir = dirs_next::home_dir()
                .unwrap_or_default()
                .join(".unterm")
                .join("clipboard");
            std::fs::create_dir_all(&clipboard_dir)
                .context("Failed to create clipboard output directory")?;

            let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S_%3f");
            let filename = format!("clipboard_{}.png", timestamp);
            let file_path = clipboard_dir.join(&filename);

            img.save(&file_path)
                .context("Failed to save clipboard image as PNG")?;

            let path_str = file_path.to_string_lossy().to_string();

            // Also produce base64 for inline use — read back the saved PNG file
            let png_bytes = std::fs::read(&file_path)
                .context("Failed to read saved clipboard PNG for base64 encoding")?;
            let b64 = base64::engine::general_purpose::STANDARD.encode(&png_bytes);

            return Ok(json!({
                "type": "image",
                "format": "png",
                "image_path": path_str,
                "width": width,
                "height": height,
                "bit_depth": bit_count,
                "size_bytes": png_bytes.len(),
                "base64": b64,
            }));
        }
    }

    // Text path: CF_UNICODETEXT
    if has_text {
        let handle: HGLOBAL = unsafe { GetClipboardData(CF_UNICODETEXT as u32) as HGLOBAL };
        if handle.is_null() {
            return Err(anyhow!("GetClipboardData(CF_UNICODETEXT) returned NULL"));
        }

        let ptr = unsafe { GlobalLock(handle) };
        if ptr.is_null() {
            return Err(anyhow!("GlobalLock failed on clipboard text data"));
        }

        // Read null-terminated UTF-16 string
        let wchar_ptr = ptr as *const u16;
        let mut len = 0usize;
        unsafe {
            while *wchar_ptr.add(len) != 0 {
                len += 1;
            }
        }
        let wstr = unsafe { std::slice::from_raw_parts(wchar_ptr, len) };
        let text = String::from_utf16_lossy(wstr);

        unsafe { GlobalUnlock(handle); }

        return Ok(json!({"type": "text", "content": text}));
    }

    Err(anyhow!("Clipboard is empty"))
}

#[cfg(windows)]
fn ps_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

#[cfg(windows)]
fn run_powershell_json(script: &str) -> Result<Value> {
    let script = format!(
        "[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)\n$OutputEncoding = [Console]::OutputEncoding\n{}",
        script
    );
    let mut bytes = Vec::with_capacity(script.len() * 2);
    for unit in script.encode_utf16() {
        bytes.extend_from_slice(&unit.to_le_bytes());
    }
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    let mut command = std::process::Command::new("powershell.exe");
    command.args([
        "-NoProfile",
        "-NonInteractive",
        "-ExecutionPolicy",
        "Bypass",
        "-EncodedCommand",
        &encoded,
    ]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let output = command.output().context("run PowerShell capture helper")?;
    if !output.status.success() {
        return Err(anyhow!(
            "PowerShell helper failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let value: Value =
        serde_json::from_str(stdout.trim()).context("parse PowerShell helper JSON output")?;
    Ok(value)
}

#[cfg(windows)]
fn append_base64_if_requested(mut value: Value, include_base64: bool) -> Result<Value> {
    if include_base64 {
        if let Some(path) = value.get("path").and_then(|v| v.as_str()) {
            let bytes = std::fs::read(path)?;
            value["base64"] = json!(base64::engine::general_purpose::STANDARD.encode(bytes));
        }
    }
    Ok(value)
}

#[cfg(windows)]
fn capture_screen_image(include_base64: bool) -> Result<Value> {
    let path = capture_output_dir()?.join(format!(
        "screen_{}.png",
        chrono::Local::now().format("%Y%m%d_%H%M%S_%3f")
    ));
    let path = path.display().to_string();
    let qpath = ps_single_quote(&path);
    let script = format!(
        r#"
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms
$bounds = [System.Windows.Forms.SystemInformation]::VirtualScreen
$bmp = New-Object System.Drawing.Bitmap $bounds.Width, $bounds.Height
$gfx = [System.Drawing.Graphics]::FromImage($bmp)
$gfx.CopyFromScreen($bounds.Left, $bounds.Top, 0, 0, $bmp.Size)
$bmp.Save({qpath}, [System.Drawing.Imaging.ImageFormat]::Png)
$gfx.Dispose()
$bmp.Dispose()
[pscustomobject]@{{
  path = {qpath}
  width = $bounds.Width
  height = $bounds.Height
  left = $bounds.Left
  top = $bounds.Top
}} | ConvertTo-Json -Compress
"#
    );
    append_base64_if_requested(run_powershell_json(&script)?, include_base64)
}

#[cfg(not(windows))]
fn capture_screen_image(_include_base64: bool) -> Result<Value> {
    Err(anyhow!("Image capture is only supported on Windows"))
}

#[cfg(windows)]
fn capture_window_image(
    title_filter: Option<&str>,
    pid_filter: Option<u32>,
    include_base64: bool,
) -> Result<Value> {
    let path = capture_output_dir()?.join(format!(
        "window_{}.png",
        chrono::Local::now().format("%Y%m%d_%H%M%S_%3f")
    ));
    let path = path.display().to_string();
    let qpath = ps_single_quote(&path);
    let title = title_filter
        .map(ps_single_quote)
        .unwrap_or_else(|| "$null".to_string());
    let pid = pid_filter.unwrap_or_else(std::process::id);
    let script = format!(
        r#"
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class UntermCapture {{
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT rect);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
}}
public struct RECT {{ public int Left; public int Top; public int Right; public int Bottom; }}
"@
$pidFilter = {pid}
$titleFilter = {title}
if ($titleFilter -ne $null) {{
  $proc = Get-Process | Where-Object {{ $_.MainWindowHandle -ne 0 -and $_.MainWindowTitle -like "*$titleFilter*" }} | Select-Object -First 1
}} else {{
  $proc = Get-Process -Id $pidFilter -ErrorAction Stop
}}
if ($null -eq $proc -or $proc.MainWindowHandle -eq 0) {{ throw "No matching window found" }}
[UntermCapture]::SetForegroundWindow($proc.MainWindowHandle) | Out-Null
Start-Sleep -Milliseconds 150
$rect = New-Object RECT
[UntermCapture]::GetWindowRect($proc.MainWindowHandle, [ref]$rect) | Out-Null
$width = $rect.Right - $rect.Left
$height = $rect.Bottom - $rect.Top
if ($width -le 0 -or $height -le 0) {{ throw "Invalid window bounds" }}
$bmp = New-Object System.Drawing.Bitmap $width, $height
$gfx = [System.Drawing.Graphics]::FromImage($bmp)
$gfx.CopyFromScreen($rect.Left, $rect.Top, 0, 0, $bmp.Size)
$bmp.Save({qpath}, [System.Drawing.Imaging.ImageFormat]::Png)
$gfx.Dispose()
$bmp.Dispose()
[pscustomobject]@{{
  path = {qpath}
  width = $width
  height = $height
  left = $rect.Left
  top = $rect.Top
  pid = $proc.Id
  title = $proc.MainWindowTitle
}} | ConvertTo-Json -Compress
"#
    );
    append_base64_if_requested(run_powershell_json(&script)?, include_base64)
}

#[cfg(not(windows))]
fn capture_window_image(
    _title_filter: Option<&str>,
    _pid_filter: Option<u32>,
    _include_base64: bool,
) -> Result<Value> {
    Err(anyhow!("Image capture is only supported on Windows"))
}

#[cfg(windows)]
fn elevated_unterm_command_args(shell: &str) -> Result<Vec<String>> {
    let gui_exe = std::env::current_exe().context("resolve current Unterm GUI executable")?;
    let gui_exe = admin_launcher_exe(&gui_exe);
    let shell_args: Vec<String> = match shell.to_ascii_lowercase().as_str() {
        "powershell" | "windows-powershell" | "windows_powershell" => {
            vec!["powershell.exe".to_string(), "-NoLogo".to_string()]
        }
        "pwsh" | "powershell7" | "powershell-7" | "powershell_7" => {
            let pwsh = "C:\\Program Files\\PowerShell\\7\\pwsh.exe";
            if std::path::Path::new(pwsh).exists() {
                vec![pwsh.to_string(), "-NoLogo".to_string()]
            } else {
                vec!["powershell.exe".to_string(), "-NoLogo".to_string()]
            }
        }
        other => return Err(anyhow!("Unsupported elevated shell: {other}")),
    };

    let script = r#"
$exe = $args[0]
$argv = @()
if ($args.Length -gt 1) {
  $argv = $args[1..($args.Length - 1)]
}
Start-Process -Verb RunAs -FilePath $exe -ArgumentList $argv
"#;

    let mut args = vec![
        "powershell.exe".to_string(),
        "-NoProfile".to_string(),
        "-ExecutionPolicy".to_string(),
        "Bypass".to_string(),
        "-Command".to_string(),
        script.to_string(),
        gui_exe.display().to_string(),
        "start".to_string(),
        "--always-new-process".to_string(),
        "--".to_string(),
    ];
    args.extend(shell_args);
    Ok(args)
}

#[cfg(windows)]
fn admin_launcher_exe(gui_exe: &std::path::Path) -> std::path::PathBuf {
    let Some(dir) = gui_exe.parent() else {
        return gui_exe.to_path_buf();
    };
    let launcher = dir.join("Unterm.exe");
    let should_copy = match (std::fs::metadata(gui_exe), std::fs::metadata(&launcher)) {
        (Ok(src), Ok(dst)) => src.len() != dst.len() || src.modified().ok() != dst.modified().ok(),
        (Ok(_), Err(_)) => true,
        _ => false,
    };

    if should_copy {
        if let Err(err) = std::fs::copy(gui_exe, &launcher) {
            log::warn!(
                "failed to prepare Unterm.exe admin launcher at {}: {err:#}",
                launcher.display()
            );
        }
    }

    if launcher.exists() {
        launcher
    } else {
        gui_exe.to_path_buf()
    }
}

/// Extract new output by comparing before/after screen text
fn diff_output(before: &str, after: &str) -> String {
    let before_lines: Vec<&str> = before.lines().collect();
    let after_lines: Vec<&str> = after.lines().collect();

    // Find where they diverge
    let common_prefix = before_lines
        .iter()
        .zip(after_lines.iter())
        .take_while(|(a, b)| a == b)
        .count();

    // New content is everything after the common prefix, minus the last line (prompt)
    let new_lines: Vec<&str> = after_lines[common_prefix..].to_vec();
    if new_lines.is_empty() {
        return String::new();
    }

    // Skip the command echo (first new line) and the new prompt (last line)
    let output_lines = if new_lines.len() > 2 {
        &new_lines[1..new_lines.len() - 1]
    } else if new_lines.len() > 1 {
        &new_lines[1..]
    } else {
        &new_lines[..]
    };

    output_lines.join("\n")
}

fn wait_wrapped_command(command: &str, shell_type: &str, marker: &str) -> String {
    match shell_type {
        "powershell" => format!("{}; Write-Output '{}'", command, marker),
        "cmd" => format!("{} & echo {}", command, marker),
        _ => format!("{}; echo {}", command, marker),
    }
}

fn extract_wait_output(before: &str, after: &str, command: &str, marker: &str) -> String {
    let diff = diff_output(before, after);
    let mut lines = Vec::new();

    for line in diff.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.contains(marker) {
            continue;
        }
        if trimmed.contains(command) {
            continue;
        }
        lines.push(trimmed.to_string());
    }

    if !lines.is_empty() {
        return lines.join("\n");
    }

    after
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter(|line| !line.contains(marker))
        .filter(|line| !line.contains(command))
        .filter(|line| !before.contains(line))
        .collect::<Vec<_>>()
        .join("\n")
}
