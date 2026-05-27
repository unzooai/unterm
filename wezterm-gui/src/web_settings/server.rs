//! Hand-rolled synchronous HTTP/1.1 server. No async runtime, no extra deps.
//!
//! The protocol surface we care about is small: GET / POST, plain
//! Content-Length bodies, no chunked encoding, no keep-alive (we close after
//! every response). Browsers are happy talking to this just fine; the SPA
//! does no fancy upgrades.
//!
//! Routes are documented in the project spec; in summary:
//!   GET  /                              -> SPA shell
//!   GET  /static/<name>                 -> embedded asset
//!   GET  /bootstrap.json                -> auth token + ports (no auth)
//!   GET  /api/health                    -> liveness check (auth)
//!   GET  /api/state                     -> aggregate snapshot (auth)
//!   POST /api/proxy                     -> proxy_configure / proxy_disable
//!   POST /api/theme                     -> writes ~/.unterm/theme.json
//!   POST /api/recording/start           -> recording::start_recording
//!   POST /api/recording/stop            -> recording::stop_recording
//!   GET  /api/sessions                  -> recording::list_sessions
//!   GET  /api/sessions/:id/markdown     -> recording::read_session_markdown

use crate::mcp::handler::McpHandler;
use crate::server_info::{self, HTTP_PREFERRED_PORT, SERVER_BIND};
use crate::web_settings::assets;
use anyhow::Result;
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

const READ_TIMEOUT: Duration = Duration::from_secs(10);
const WRITE_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_HEADER_BYTES: usize = 16 * 1024;
const MAX_BODY_BYTES: usize = 1024 * 1024;

/// Bind the HTTP server, register its port in `~/.unterm/server.json`, and
/// start serving on a background thread. Returns the bound port.
pub fn start_web_settings_server(auth_token: String) -> u16 {
    let (listener, port) = match server_info::bind_with_fallback(HTTP_PREFERRED_PORT) {
        Ok(pair) => pair,
        Err(e) => {
            log::error!("HTTP settings server failed to bind: {}", e);
            return 0;
        }
    };

    if let Err(e) = server_info::set_http_port(port) {
        log::warn!("Could not stamp http_port in server.json: {}", e);
    }

    let handler = Arc::new(McpHandler::new());
    thread::Builder::new()
        .name("web-settings".into())
        .spawn(move || run_server(listener, auth_token, handler))
        .expect("Failed to spawn web settings thread");

    log::info!(
        "Web settings server listening on http://{}:{}/",
        SERVER_BIND,
        port
    );
    port
}

fn run_server(listener: TcpListener, auth_token: String, handler: Arc<McpHandler>) {
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let token = auth_token.clone();
                let handler = Arc::clone(&handler);
                thread::Builder::new()
                    .name("web-settings-conn".into())
                    .spawn(move || {
                        if let Err(e) = handle_client(stream, &token, &handler) {
                            log::debug!("HTTP client error: {}", e);
                        }
                    })
                    .ok();
            }
            Err(e) => log::warn!("HTTP accept error: {}", e),
        }
    }
}

fn handle_client(
    mut stream: TcpStream,
    auth_token: &str,
    handler: &McpHandler,
) -> Result<()> {
    stream.set_read_timeout(Some(READ_TIMEOUT)).ok();
    stream.set_write_timeout(Some(WRITE_TIMEOUT)).ok();
    stream.set_nodelay(true).ok();

    let req = match parse_request(&mut stream) {
        Ok(r) => r,
        Err(e) => {
            log::debug!("malformed HTTP request: {}", e);
            let _ = write_status_response(&mut stream, 400, "Bad Request", b"bad request");
            return Ok(());
        }
    };

    let resp = route(&req, auth_token, handler);
    write_response(&mut stream, &resp)
}

#[derive(Debug)]
struct Request {
    method: String,
    path: String,
    query: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl Request {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }
}

pub(crate) struct Response {
    status: u16,
    reason: &'static str,
    content_type: &'static str,
    body: Vec<u8>,
    extra_headers: Vec<(&'static str, String)>,
}

impl Response {
    pub(crate) fn json(status: u16, reason: &'static str, value: Value) -> Self {
        Self {
            status,
            reason,
            content_type: "application/json; charset=utf-8",
            body: serde_json::to_vec(&value).unwrap_or_default(),
            extra_headers: Vec::new(),
        }
    }

    fn text(
        status: u16,
        reason: &'static str,
        content_type: &'static str,
        body: Vec<u8>,
    ) -> Self {
        Self {
            status,
            reason,
            content_type,
            body,
            extra_headers: Vec::new(),
        }
    }

    pub(crate) fn ok_json(value: Value) -> Self {
        Self::json(200, "OK", value)
    }

    pub(crate) fn err(status: u16, reason: &'static str, message: &str) -> Self {
        Self::json(status, reason, json!({"error": message}))
    }
}

fn parse_request(stream: &mut TcpStream) -> Result<Request> {
    let mut reader = BufReader::new(stream.try_clone()?);

    // Request line
    let mut line = String::new();
    reader.read_line(&mut line)?;
    let line = line.trim_end_matches(['\r', '\n']);
    let mut parts = line.splitn(3, ' ');
    let method = parts
        .next()
        .ok_or_else(|| anyhow::anyhow!("no method"))?
        .to_string();
    let target = parts
        .next()
        .ok_or_else(|| anyhow::anyhow!("no target"))?
        .to_string();
    let _version = parts.next().unwrap_or("HTTP/1.1");

    let (path, query) = match target.find('?') {
        Some(i) => (target[..i].to_string(), target[i + 1..].to_string()),
        None => (target, String::new()),
    };

    let mut headers: Vec<(String, String)> = Vec::new();
    let mut total = 0usize;
    loop {
        let mut buf = String::new();
        let n = reader.read_line(&mut buf)?;
        if n == 0 {
            break;
        }
        total += n;
        if total > MAX_HEADER_BYTES {
            anyhow::bail!("headers too large");
        }
        let trimmed = buf.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            break;
        }
        if let Some(idx) = trimmed.find(':') {
            let k = trimmed[..idx].trim().to_string();
            let v = trimmed[idx + 1..].trim().to_string();
            headers.push((k, v));
        }
    }

    let content_length = headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("content-length"))
        .and_then(|(_, v)| v.parse::<usize>().ok())
        .unwrap_or(0);
    if content_length > MAX_BODY_BYTES {
        anyhow::bail!("body too large");
    }

    let mut body = vec![0u8; content_length];
    if content_length > 0 {
        reader.read_exact(&mut body)?;
    }

    Ok(Request {
        method,
        path,
        query,
        headers,
        body,
    })
}

fn write_response(stream: &mut TcpStream, resp: &Response) -> Result<()> {
    let mut head = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\n",
        resp.status,
        resp.reason,
        resp.content_type,
        resp.body.len()
    );
    for (k, v) in &resp.extra_headers {
        head.push_str(k);
        head.push_str(": ");
        head.push_str(v);
        head.push_str("\r\n");
    }
    head.push_str("\r\n");
    stream.write_all(head.as_bytes())?;
    stream.write_all(&resp.body)?;
    stream.flush()?;
    Ok(())
}

fn write_status_response(
    stream: &mut TcpStream,
    status: u16,
    reason: &'static str,
    body: &[u8],
) -> Result<()> {
    let resp = Response::text(status, reason, "text/plain; charset=utf-8", body.to_vec());
    write_response(stream, &resp)
}

fn route(req: &Request, auth_token: &str, handler: &McpHandler) -> Response {
    let path = req.path.as_str();

    // Public (no auth) endpoints
    if req.method == "GET" && path == "/" {
        return Response::text(
            200,
            "OK",
            "text/html; charset=utf-8",
            assets::INDEX_HTML.as_bytes().to_vec(),
        );
    }
    if req.method == "GET" && path == "/favicon.ico" {
        // Browsers fetch this automatically; respond with an empty 204 so we
        // don't pollute the console with 401s.
        return Response::text(204, "No Content", "image/x-icon", Vec::new());
    }
    if req.method == "GET" && path == "/bootstrap.json" {
        // Return THIS instance's token + ports, not whichever instance
        // happens to be the registered "active" pointer. Critical for
        // multi-instance: when alpha is the active pointer and bravo's
        // Web Settings page bootstraps, it must get bravo's token —
        // otherwise the SPA tries to talk to its own HTTP server with
        // alpha's token and every subsequent /api/* call 401s.
        //
        // `read_current()` reads this process's per-instance JSON file
        // (`~/.unterm/instances/<id>.json`), so the answer is always
        // self-consistent with the server actually handling the
        // request. We deliberately do NOT fall back to the active
        // pointer when read_current fails — that would re-introduce
        // the cross-instance leak. An empty token field is the safer
        // failure mode: the SPA shows its bootstrap-failed error and
        // the user can investigate, rather than silently auth'ing
        // against the wrong instance.
        let info = server_info::read_current();
        // unterm_cli_path is the absolute path to the unterm-cli binary
        // sibling of this running GUI executable. The SPA emits this in
        // "Copy launch command" so it works even when the user's $PATH
        // doesn't include the .app bundle's MacOS dir (every fresh DMG
        // install on macOS hits this — there's no automatic symlink to
        // /usr/local/bin). Falls back to bare `unterm-cli` if we can't
        // resolve the executable path (extremely unlikely on Unix; might
        // matter for Wine / weird mount scenarios).
        let unterm_cli_path = std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(|d| d.join("unterm-cli")))
            .filter(|p| p.exists())
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "unterm-cli".to_string());
        return Response::ok_json(json!({
            "auth_token": info.auth_token,
            "mcp_port": info.mcp_port,
            "http_port": info.http_port,
            "unterm_cli_path": unterm_cli_path,
        }));
    }
    if req.method == "GET" {
        if let Some(name) = path.strip_prefix("/static/") {
            if let Some((ct, body)) = assets::lookup_static(name) {
                return Response::text(200, "OK", ct, body.as_bytes().to_vec());
            }
            return Response::err(404, "Not Found", "no such static asset");
        }
    }

    // Everything else demands the bearer token.
    if !auth_ok(req, auth_token) {
        return Response::err(401, "Unauthorized", "missing or bad bearer token");
    }

    match (req.method.as_str(), path) {
        ("GET", "/api/health") => Response::ok_json(json!({"ok": true})),
        ("GET", "/api/state") => api_state(handler),
        ("POST", "/api/proxy") => api_proxy(handler, &req.body),
        ("POST", "/api/theme") => api_theme(&req.body),
        ("GET", "/api/scrollback") => api_scrollback_get(),
        ("POST", "/api/scrollback") => api_scrollback_set(&req.body),
        ("GET", "/api/compat") => api_compat_get(),
        ("POST", "/api/compat") => api_compat_set(&req.body),
        ("GET", "/api/updates") => api_updates_get(),
        ("POST", "/api/updates/check") => api_updates_check(),
        ("POST", "/api/recording/start") => api_recording(handler, &req.body, true),
        ("POST", "/api/recording/stop") => api_recording(handler, &req.body, false),
        ("GET", "/api/sessions") => api_sessions(handler, &req.query),
        ("GET", p) if p.starts_with("/api/sessions/") && p.ends_with("/markdown") => {
            api_session_markdown(handler, p)
        }
        ("GET", "/api/reference") => api_reference(),
        ("GET", "/api/i18n") => api_i18n_state(),
        ("POST", "/api/i18n") => api_i18n_set(&req.body),
        ("GET", p) if p.starts_with("/api/i18n/") => {
            let code = &p["/api/i18n/".len()..];
            api_i18n_dict(code)
        }
        // Identity profile management — backs the Settings panel and the
        // onboarding wizard. List/get are pure reads; the rest mutate
        // `~/.unterm/profiles/` and (for secrets) the OS keychain.
        // MCP trust + audit surface for the Settings panel. Read endpoints
        // are cheap (one lock acquire each); write endpoints persist to
        // ~/.unterm/trusted_agents.json synchronously.
        ("GET", "/api/mcp/trusted") => Response::ok_json(crate::mcp::handler::trust_snapshot()),
        ("POST", "/api/mcp/trust") => api_mcp_trust(&req.body),
        ("POST", "/api/mcp/untrust") => api_mcp_untrust(&req.body),
        ("GET", "/api/mcp/audit") => api_mcp_audit(&req.query),
        ("GET", "/api/profile/list") => api_profile_list(),
        ("POST", "/api/profile/create") => api_profile_create(&req.body),
        ("GET", "/api/profile/import-scan") => api_profile_import_scan(),
        ("GET", "/api/profile/ssh-include-status") => api_profile_ssh_include_status(),
        ("POST", "/api/profile/install-ssh-include") => api_profile_install_ssh_include(),
        ("POST", "/api/profile/set-default") => api_profile_set_default(&req.body),
        ("PUT", p) if p.starts_with("/api/profile/") && !p.contains("/secret") => {
            let id = &p["/api/profile/".len()..];
            api_profile_update(id, &req.body)
        }
        ("DELETE", p) if p.starts_with("/api/profile/") && !p.contains("/secret") => {
            let id = &p["/api/profile/".len()..];
            api_profile_delete(id)
        }
        ("POST", p) if p.starts_with("/api/profile/") && p.ends_with("/secret") => {
            let id = &p["/api/profile/".len()..p.len() - "/secret".len()];
            api_profile_set_secret(id, &req.body)
        }
        ("DELETE", p) if p.starts_with("/api/profile/") && p.contains("/secret/") => {
            // /api/profile/{id}/secret/{env_name}
            let rest = &p["/api/profile/".len()..];
            if let Some((id, env_name)) = rest.split_once("/secret/") {
                api_profile_delete_secret(id, env_name)
            } else {
                Response::err(400, "Bad Request", "malformed secret path")
            }
        }

        // ---- AI agent runtime --------------------------------------------
        // Routes wrap the unterm-agents crate so the Web Settings panel can
        // list / configure / install / launch every AI CLI in the signed
        // manifest. See `agents.rs` for the actual handlers.
        ("GET",  "/api/agents/list")             => super::agents::api_list(&req.query),
        ("GET",  "/api/agents/manifest")          => super::agents::api_manifest_info(),
        ("POST", "/api/agents/manifest/refresh")  => super::agents::api_manifest_refresh(),
        ("GET",  p) if p.starts_with("/api/agents/") && p.ends_with("/settings") => {
            let id = &p["/api/agents/".len()..p.len() - "/settings".len()];
            super::agents::api_settings_get(id, &req.query)
        }
        ("PUT",  p) if p.starts_with("/api/agents/") && p.ends_with("/settings") => {
            let id = &p["/api/agents/".len()..p.len() - "/settings".len()];
            super::agents::api_settings_put(id, &req.body)
        }
        ("POST", p) if p.starts_with("/api/agents/") && p.ends_with("/install") => {
            let id = &p["/api/agents/".len()..p.len() - "/install".len()];
            super::agents::api_install(id)
        }
        ("POST", p) if p.starts_with("/api/agents/") && p.ends_with("/uninstall") => {
            let id = &p["/api/agents/".len()..p.len() - "/uninstall".len()];
            super::agents::api_uninstall(id)
        }
        ("POST", p) if p.starts_with("/api/agents/") && p.ends_with("/update") => {
            let id = &p["/api/agents/".len()..p.len() - "/update".len()];
            super::agents::api_update_agent(id)
        }
        ("POST", p) if p.starts_with("/api/agents/") && p.ends_with("/auth") => {
            let id = &p["/api/agents/".len()..p.len() - "/auth".len()];
            super::agents::api_auth(id, &req.body)
        }
        ("GET",  p) if p.starts_with("/api/agents/") && p.ends_with("/import") => {
            let id = &p["/api/agents/".len()..p.len() - "/import".len()];
            super::agents::api_import(id, &req.query)
        }
        ("POST", p) if p.starts_with("/api/agents/") && p.ends_with("/launch-plan") => {
            let id = &p["/api/agents/".len()..p.len() - "/launch-plan".len()];
            super::agents::api_launch_plan(id, &req.body)
        }
        ("GET",  p) if p.starts_with("/api/agents/") && !p[("/api/agents/".len())..].contains('/') => {
            // /api/agents/<id>  — catch-all GET for one agent's full manifest.
            // Match by "no further slash after /api/agents/"; the previous
            // p.matches('/').count() == 2 check was off-by-one (the string
            // has 3 slashes once the trailing id segment is present) and
            // silently 404'd, which made the SPA's openAgent() throw on
            // manifest fetch.
            let id = &p["/api/agents/".len()..];
            super::agents::api_show(id)
        }

        _ => Response::err(404, "Not Found", "no such route"),
    }
}

// --- Profile endpoints ----------------------------------------------------
//
// Profile state lives in three places: the TOML files under
// ~/.unterm/profiles/, the OS keychain (referenced from [secrets] via
// `keychain://...` URLs), and ~/.unterm/ssh/config.unterm. Mutating
// endpoints go through ProfileRegistry which keeps all three in sync
// — write a TOML, sync_ssh_config regenerates the SSH fragment, and
// set_secret round-trips through the keychain.
//
// Secret values are accepted from the SPA over the bearer-token-
// authenticated localhost HTTP server. They are never echoed back in
// any response — list/get endpoints surface keychain reference URLs
// only, so an open browser tab can't be tricked into reading a token.

fn profile_summary_json(
    id: &str,
    profile: &unterm_profile::ProfileFile,
    default_id: Option<&str>,
) -> Value {
    let today = chrono::Local::now().date_naive();
    let warnings: Vec<Value> = profile
        .expiration
        .iter()
        .filter_map(|(env, date_str)| {
            let date = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d").ok()?;
            let days = (date - today).num_days();
            if days <= 7 {
                Some(json!({
                    "env_name": env,
                    "expires_on": date_str,
                    "days_remaining": days,
                }))
            } else {
                None
            }
        })
        .collect();
    json!({
        "id": id,
        "display_name": profile.display_name,
        "accent_color": profile.accent_color,
        "description": profile.description,
        "secret_count": profile.secrets.len(),
        "secret_keys": profile.secrets.keys().collect::<Vec<_>>(),
        "git": {
            "user_name": profile.git.user_name,
            "user_email": profile.git.user_email,
        },
        "env": profile.env,
        "ssh": profile.ssh,
        "expiration": profile.expiration,
        "warnings": warnings,
        "is_default": default_id == Some(id),
    })
}

// --- MCP trust + audit endpoints ------------------------------------------

fn api_mcp_trust(body: &[u8]) -> Response {
    let v = parse_json_body(body);
    let name = match v.get("name").and_then(|x| x.as_str()) {
        Some(s) if !s.trim().is_empty() => s.to_string(),
        _ => return Response::err(400, "Bad Request", "name required"),
    };
    let added = crate::mcp::handler::grant_trust(&name);
    Response::ok_json(json!({ "ok": true, "name": name, "added": added }))
}

fn api_mcp_untrust(body: &[u8]) -> Response {
    let v = parse_json_body(body);
    let name = match v.get("name").and_then(|x| x.as_str()) {
        Some(s) if !s.trim().is_empty() => s.to_string(),
        _ => return Response::err(400, "Bad Request", "name required"),
    };
    let removed = crate::mcp::handler::revoke_trust(&name);
    Response::ok_json(json!({ "ok": true, "name": name, "removed": removed }))
}

fn api_mcp_audit(query: &str) -> Response {
    // ?limit=N caps the response. Default 100, max 1000 so the SPA
    // can't accidentally request the whole 1000-entry log on every
    // refresh. The audit log itself is already capped at the
    // mcp_audit_log_capacity config knob (default 1000).
    let limit = query
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .find_map(|(k, v)| (k == "limit").then(|| v.parse::<usize>().ok()).flatten())
        .unwrap_or(100)
        .min(1000);
    let payload_str = crate::mcp::handler::audit_log_snapshot_json(limit);
    // audit_log_snapshot_json already returns a complete JSON value; we
    // serve it as-is rather than re-parsing + re-serializing.
    Response::text(200, "OK", "application/json; charset=utf-8", payload_str.into_bytes())
}

fn api_profile_list() -> Response {
    let registry = match unterm_profile::ProfileRegistry::load() {
        Ok(r) => r,
        Err(e) => return Response::err(500, "Internal Error", &e.to_string()),
    };
    let default = registry.default_id().map(str::to_string);
    let profiles: Vec<Value> = registry
        .iter_ordered()
        .into_iter()
        .map(|(id, p)| profile_summary_json(id, p, default.as_deref()))
        .collect();
    Response::ok_json(json!({
        "profiles": profiles,
        "default": default,
    }))
}

fn api_profile_create(body: &[u8]) -> Response {
    let v = parse_json_body(body);
    let display_name = match v.get("display_name").and_then(|x| x.as_str()) {
        Some(s) if !s.trim().is_empty() => s.to_string(),
        _ => return Response::err(400, "Bad Request", "display_name required"),
    };
    let accent = v.get("accent_color").and_then(|x| x.as_str());
    let mut registry = match unterm_profile::ProfileRegistry::load() {
        Ok(r) => r,
        Err(e) => return Response::err(500, "Internal Error", &e.to_string()),
    };
    match registry.create(&display_name, accent) {
        Ok(id) => Response::ok_json(json!({"id": id, "display_name": display_name})),
        Err(e) => Response::err(500, "Internal Error", &e.to_string()),
    }
}

fn api_profile_delete(id: &str) -> Response {
    use unterm_profile::SecretKey;
    let mut registry = match unterm_profile::ProfileRegistry::load() {
        Ok(r) => r,
        Err(e) => return Response::err(500, "Internal Error", &e.to_string()),
    };
    let Some(profile) = registry.get(id).cloned() else {
        return Response::err(404, "Not Found", "profile not found");
    };
    // Best-effort keychain cleanup before removing the TOML.
    if let Ok(store) = unterm_profile::default_store() {
        for env in profile.secrets.keys() {
            let _ = store.delete(&SecretKey::new(id, env));
        }
    }
    if let Err(e) = std::fs::remove_file(unterm_profile::profile_path(id)) {
        return Response::err(500, "Internal Error", &format!("remove profile: {e}"));
    }
    // Reload registry so sync_ssh_config regenerates from current state.
    let reloaded = unterm_profile::ProfileRegistry::load()
        .unwrap_or_else(|_| unterm_profile::ProfileRegistry::empty());
    let _ = reloaded.sync_ssh_config();
    Response::ok_json(json!({"ok": true}))
}

fn api_profile_update(id: &str, body: &[u8]) -> Response {
    let v = parse_json_body(body);
    let mut registry = match unterm_profile::ProfileRegistry::load() {
        Ok(r) => r,
        Err(e) => return Response::err(500, "Internal Error", &e.to_string()),
    };
    let Some(mut profile) = registry.get(id).cloned() else {
        return Response::err(404, "Not Found", "profile not found");
    };
    // Patch only the fields present in the body — partial update.
    if let Some(s) = v.get("display_name").and_then(|x| x.as_str()) {
        profile.display_name = s.to_string();
    }
    if let Some(s) = v.get("accent_color").and_then(|x| x.as_str()) {
        profile.accent_color = s.to_string();
    }
    if let Some(s) = v.get("description").and_then(|x| x.as_str()) {
        profile.description = s.to_string();
    }
    if let Some(g) = v.get("git").and_then(|x| x.as_object()) {
        if let Some(s) = g.get("user_name").and_then(|x| x.as_str()) {
            profile.git.user_name = s.to_string();
        }
        if let Some(s) = g.get("user_email").and_then(|x| x.as_str()) {
            profile.git.user_email = s.to_string();
        }
    }
    if let Some(map) = v.get("ssh").and_then(|x| x.as_object()) {
        profile.ssh.clear();
        for (k, val) in map {
            if let Some(s) = val.as_str() {
                profile.ssh.insert(k.clone(), s.to_string());
            }
        }
    }
    if let Some(map) = v.get("env").and_then(|x| x.as_object()) {
        profile.env.clear();
        for (k, val) in map {
            if let Some(s) = val.as_str() {
                profile.env.insert(k.clone(), s.to_string());
            }
        }
    }
    if let Err(e) = registry.save_profile(id, profile) {
        return Response::err(500, "Internal Error", &e.to_string());
    }
    Response::ok_json(json!({"ok": true}))
}

fn api_profile_set_secret(id: &str, body: &[u8]) -> Response {
    let v = parse_json_body(body);
    let env_name = match v.get("env_name").and_then(|x| x.as_str()) {
        Some(s) if !s.is_empty() => s.to_string(),
        _ => return Response::err(400, "Bad Request", "env_name required"),
    };
    let value = match v.get("value").and_then(|x| x.as_str()) {
        Some(s) if !s.is_empty() => s.to_string(),
        _ => return Response::err(400, "Bad Request", "non-empty value required"),
    };
    let mut registry = match unterm_profile::ProfileRegistry::load() {
        Ok(r) => r,
        Err(e) => return Response::err(500, "Internal Error", &e.to_string()),
    };
    let store = match unterm_profile::default_store() {
        Ok(s) => s,
        Err(e) => return Response::err(500, "Internal Error", &format!("keychain: {e}")),
    };
    if let Err(e) = registry.set_secret(store.as_ref(), id, &env_name, &value) {
        return Response::err(500, "Internal Error", &e.to_string());
    }
    Response::ok_json(json!({"ok": true, "env_name": env_name}))
}

fn api_profile_delete_secret(id: &str, env_name: &str) -> Response {
    use unterm_profile::SecretKey;
    let mut registry = match unterm_profile::ProfileRegistry::load() {
        Ok(r) => r,
        Err(e) => return Response::err(500, "Internal Error", &e.to_string()),
    };
    let Some(mut profile) = registry.get(id).cloned() else {
        return Response::err(404, "Not Found", "profile not found");
    };
    if profile.secrets.remove(env_name).is_none() {
        return Response::err(404, "Not Found", "secret not on this profile");
    }
    // Drop the keychain entry too. Idempotent — if it was already
    // removed out-of-band, the delete returns Ok.
    if let Ok(store) = unterm_profile::default_store() {
        let _ = store.delete(&SecretKey::new(id, env_name));
    }
    profile.expiration.remove(env_name);
    if let Err(e) = registry.save_profile(id, profile) {
        return Response::err(500, "Internal Error", &e.to_string());
    }
    Response::ok_json(json!({"ok": true}))
}

fn api_profile_set_default(body: &[u8]) -> Response {
    let v = parse_json_body(body);
    let id = v
        .get("id")
        .and_then(|x| x.as_str())
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    let mut registry = match unterm_profile::ProfileRegistry::load() {
        Ok(r) => r,
        Err(e) => return Response::err(500, "Internal Error", &e.to_string()),
    };
    if let Err(e) = registry.set_default(id.clone()) {
        return Response::err(500, "Internal Error", &e.to_string());
    }
    Response::ok_json(json!({"ok": true, "default": id}))
}

fn api_profile_import_scan() -> Response {
    let candidates = unterm_profile::sniff::scan_all();
    let json_list: Vec<Value> = candidates
        .into_iter()
        .map(|c| {
            json!({
                "source": c.source.as_str(),
                "label": c.label,
                "suggested_env_name": c.suggested_env_name,
                "has_value": c.suggested_value.is_some(),
                // We deliberately DON'T return `suggested_value` here even
                // though the sniffer has it — the SPA shouldn't display
                // raw tokens, and shipping them over HTTP (even on
                // localhost) is an unnecessary exposure. The wizard
                // passes the candidate identity (source/label/env_name)
                // back to the server, which re-runs the sniffer just for
                // that source to get a fresh value. See
                // api_profile_import_scan_value.
                "host": c.host,
                "user": c.user,
            })
        })
        .collect();
    Response::ok_json(json!({"candidates": json_list}))
}

fn api_profile_ssh_include_status() -> Response {
    Response::ok_json(json!({
        "installed": unterm_profile::ssh::user_config_has_include(),
        "user_config_path": unterm_profile::ssh::user_ssh_config_path()
            .map(|p| p.display().to_string()),
        "include_path": unterm_profile::ssh::ssh_config_unterm_path()
            .display()
            .to_string(),
    }))
}

fn api_profile_install_ssh_include() -> Response {
    let Some(path) = unterm_profile::ssh::user_ssh_config_path() else {
        return Response::err(500, "Internal Error", "no home dir");
    };
    // Read existing content (if any) and check for the Include line.
    let existing = std::fs::read_to_string(&path).unwrap_or_default();
    if existing
        .lines()
        .any(|l| l.trim().starts_with("Include") && l.contains("config.unterm"))
    {
        return Response::ok_json(json!({"ok": true, "already_present": true}));
    }
    if let Some(parent) = path.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            return Response::err(500, "Internal Error", &format!("create dir: {e}"));
        }
    }
    // Append (or create) — leading newline so we don't fuse with the
    // previous line if the file didn't end with one. The header
    // comment is preserved so the user knows where the line came from.
    let mut appended = existing;
    if !appended.is_empty() && !appended.ends_with('\n') {
        appended.push('\n');
    }
    appended.push_str(
        "\n# Added by Unterm — exposes per-profile SSH key routing.\n\
         Include ~/.unterm/ssh/config.unterm\n",
    );
    if let Err(e) = std::fs::write(&path, appended) {
        return Response::err(500, "Internal Error", &format!("write ssh config: {e}"));
    }
    Response::ok_json(json!({"ok": true, "already_present": false}))
}

// --- reference endpoint ----------------------------------------------------

/// `GET /api/reference` — proxies the `meta.surface` MCP method so the
/// Reference tab in Web Settings renders the same inventory an agent
/// would receive. No params, no auth state — pure read.
fn api_reference() -> Response {
    match crate::mcp::meta::surface(&serde_json::Value::Null) {
        Ok(v) => Response::ok_json(v),
        Err(e) => Response::err(500, "Internal Server Error", &e.to_string()),
    }
}

// --- i18n endpoints --------------------------------------------------------

fn api_i18n_state() -> Response {
    let current = crate::i18n::current_locale();
    let available: Vec<Value> = crate::i18n::available_locales()
        .iter()
        .map(|(code, name)| json!({"code": code, "name": name}))
        .collect();
    let dict = crate::i18n::dictionary(current)
        .map(|d| {
            let map: serde_json::Map<String, Value> = d
                .iter()
                .map(|(k, v)| (k.clone(), Value::String(v.clone())))
                .collect();
            Value::Object(map)
        })
        .unwrap_or_else(|| json!({}));
    Response::ok_json(json!({
        "current": current,
        "available": available,
        "dict": dict,
    }))
}

fn api_i18n_dict(code: &str) -> Response {
    match crate::i18n::dictionary(code) {
        Some(d) => {
            let map: serde_json::Map<String, Value> = d
                .iter()
                .map(|(k, v)| (k.clone(), Value::String(v.clone())))
                .collect();
            Response::ok_json(Value::Object(map))
        }
        None => Response::err(
            404,
            "Not Found",
            &crate::i18n::t("web.api.unknown_locale"),
        ),
    }
}

fn api_i18n_set(body: &[u8]) -> Response {
    let body = parse_json_body(body);
    let lang = match body.get("lang").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => {
            return Response::err(
                400,
                "Bad Request",
                &crate::i18n::t("web.api.invalid_lang"),
            )
        }
    };
    if !crate::i18n::set_locale(&lang) {
        return Response::err(
            400,
            "Bad Request",
            &crate::i18n::t("web.api.unknown_locale"),
        );
    }
    let current = crate::i18n::current_locale();
    let available: Vec<Value> = crate::i18n::available_locales()
        .iter()
        .map(|(code, name)| json!({"code": code, "name": name}))
        .collect();
    Response::ok_json(json!({
        "current": current,
        "available": available,
    }))
}

fn auth_ok(req: &Request, auth_token: &str) -> bool {
    if auth_token.is_empty() {
        return false;
    }
    match req.header("Authorization") {
        Some(v) => v
            .strip_prefix("Bearer ")
            .map(|t| t.trim() == auth_token)
            .unwrap_or(false),
        None => false,
    }
}

fn parse_json_body(body: &[u8]) -> Value {
    if body.is_empty() {
        return json!({});
    }
    serde_json::from_slice(body).unwrap_or(json!({}))
}

// --- API implementations --------------------------------------------------

fn api_state(handler: &McpHandler) -> Response {
    // Same instance-locality requirement as /bootstrap.json: each Web
    // Settings server must surface ITS OWN pid / started_at / ports,
    // not whichever instance happens to own active.json. The "Status"
    // line in the SPA header otherwise reads e.g. `127.0.0.1:19877`
    // (alpha) while the page is actually being served by bravo on
    // 19879, which is both confusing and breaks the "click to copy
    // a working /api/health curl command" affordance.
    let info = server_info::read_current();
    let ctx = crate::mcp::handler::ConnectionContext::internal("web_settings");
    let proxy = handler
        .handle(&ctx, "proxy.status", &json!({}))
        .unwrap_or_else(|e| json!({"error": e.to_string()}));

    let theme = current_theme_id();
    let project = current_project_info();
    let recording = current_recording_info();
    let sessions_path = sessions_path_string();
    let hostname = hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_default();

    Response::ok_json(json!({
        "version": config::wezterm_version(),
        "hostname": hostname,
        "pid": info.pid,
        "started_at": info.started_at,
        "ports": {
            "mcp": info.mcp_port,
            "http": info.http_port,
        },
        "theme": theme,
        "proxy": proxy,
        "project": project,
        "recording": recording,
        "sessions_path": sessions_path,
        "scrollback": {
            "lines": current_scrollback_lines(),
            "default": 10_000,
            "max": 999_999_999u64,
        },
    }))
}

// --- Scrollback override ---------------------------------------------------
//
// Stored at `~/.unterm/scrollback.json` so config::default_scrollback_lines()
// can read it without going through the lua config layer. Changes only
// affect newly-created panes (the existing VecDeque<Line> per pane has its
// capacity locked at construction), so the UI surfaces a "restart to apply"
// hint after Save.

fn scrollback_path() -> std::path::PathBuf {
    dirs_next::home_dir()
        .unwrap_or_default()
        .join(".unterm")
        .join("scrollback.json")
}

fn current_scrollback_lines() -> u64 {
    if let Ok(content) = std::fs::read_to_string(scrollback_path()) {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(n) = value.get("lines").and_then(|v| v.as_u64()) {
                if n > 0 {
                    return n;
                }
            }
        }
    }
    10_000
}

fn api_scrollback_get() -> Response {
    Response::ok_json(json!({
        "lines": current_scrollback_lines(),
        "default": 10_000,
        "max": 999_999_999u64,
        "min": 100u64,
    }))
}

fn api_scrollback_set(body: &[u8]) -> Response {
    let body = parse_json_body(body);
    let lines = match body.get("lines").and_then(|v| v.as_u64()) {
        Some(n) if n >= 100 && n <= 999_999_999 => n,
        Some(_) => return Response::err(400, "Bad Request", "lines must be in [100, 999999999]"),
        None => return Response::err(400, "Bad Request", "missing lines (u64)"),
    };
    let path = scrollback_path();
    if let Some(parent) = path.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            return Response::err(500, "Internal Error", &e.to_string());
        }
    }
    let payload = serde_json::to_string_pretty(&json!({"lines": lines})).unwrap();
    if let Err(e) = std::fs::write(&path, payload) {
        return Response::err(500, "Internal Error", &e.to_string());
    }
    Response::ok_json(json!({
        "applied": true,
        "lines": lines,
        // Existing panes keep their old buffer; new panes pick up the new
        // value. Tell the client so it can prompt the user.
        "requires_restart_for_existing_panes": true,
    }))
}

// --- Compatibility (TERM_PROGRAM masquerade) -------------------------------
//
// Some third-party tools — Gemini CLI, certain IDE terminal detectors —
// whitelist a fixed set of TERM_PROGRAM values and reject anything else.
// We default to "Unterm" because that's our product identity, but expose
// a per-user override at ~/.unterm/compat.json. config::read_term_program_override
// reads it when spawning each new shell. Existing shells keep their old
// env until the user opens a new tab.

fn compat_path() -> std::path::PathBuf {
    dirs_next::home_dir()
        .unwrap_or_default()
        .join(".unterm")
        .join("compat.json")
}

fn current_term_program() -> String {
    if let Ok(content) = std::fs::read_to_string(compat_path()) {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(s) = value.get("term_program").and_then(|v| v.as_str()) {
                let trimmed = s.trim();
                if !trimmed.is_empty() {
                    return trimmed.to_string();
                }
            }
        }
    }
    "Unterm".to_string()
}

fn api_compat_get() -> Response {
    Response::ok_json(json!({
        "term_program": current_term_program(),
        "default": "Unterm",
        // Common values the UI offers as one-click presets. Custom is
        // implicitly available — any non-listed string is accepted.
        "presets": ["Unterm", "WezTerm", "Apple_Terminal", "iTerm.app", "xterm"],
    }))
}

// --- Update check ---------------------------------------------------------
//
// The actual GitHub poll lives in `crate::update_check` and runs in a
// background thread that writes ~/.unterm/update_check.json every 6 h.
// These endpoints are thin: GET reads that file, POST kicks the
// poller's check_once() so the user can hit "Check now" in the UI
// without waiting on the next 6h tick.

fn api_updates_get() -> Response {
    Response::ok_json(crate::update_check::read_state())
}

fn api_updates_check() -> Response {
    // Run synchronously — the request takes one HTTP round-trip to
    // GitHub plus a small file write, well under a second on any
    // network where we can reach the user at all.
    if let Err(e) = crate::update_check::check_once() {
        return Response::err(502, "Bad Gateway", &format!("github check failed: {e:#}"));
    }
    Response::ok_json(crate::update_check::read_state())
}

fn api_compat_set(body: &[u8]) -> Response {
    let body = parse_json_body(body);
    let term_program = match body.get("term_program").and_then(|v| v.as_str()) {
        Some(s) => {
            let s = s.trim();
            if s.is_empty() {
                return Response::err(400, "Bad Request", "term_program may not be empty — pass \"Unterm\" to reset to default");
            }
            if s.len() > 64 {
                return Response::err(400, "Bad Request", "term_program too long (max 64 chars)");
            }
            // Reject anything that contains nul / newline / control chars,
            // since this string ends up in an env var passed to child
            // processes — corrupt env breaks downstream shells.
            if s.chars().any(|c| c.is_control()) {
                return Response::err(400, "Bad Request", "term_program contains control characters");
            }
            s.to_string()
        }
        None => return Response::err(400, "Bad Request", "missing term_program (string)"),
    };
    let path = compat_path();
    if let Some(parent) = path.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            return Response::err(500, "Internal Error", &e.to_string());
        }
    }
    let payload = serde_json::to_string_pretty(&json!({"term_program": term_program})).unwrap();
    if let Err(e) = std::fs::write(&path, payload) {
        return Response::err(500, "Internal Error", &e.to_string());
    }
    Response::ok_json(json!({
        "applied": true,
        "term_program": term_program,
        // Existing shells keep their old TERM_PROGRAM; only newly spawned
        // shells pick up the change. Tell the client to prompt for new tab.
        "requires_new_shell": true,
    }))
}

fn api_proxy(handler: &McpHandler, body: &[u8]) -> Response {
    let body = parse_json_body(body);
    let enabled = body
        .get("enabled")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let ctx = crate::mcp::handler::ConnectionContext::internal("web_settings");
    let result = if !enabled {
        handler.handle(&ctx, "proxy.disable", &json!({}))
    } else {
        let mut params = serde_json::Map::new();
        params.insert("enabled".into(), Value::Bool(true));
        let mode = if body.get("http_proxy").is_some() || body.get("socks_proxy").is_some() {
            "manual"
        } else {
            "auto"
        };
        params.insert("mode".into(), Value::String(mode.into()));
        for k in ["http_proxy", "socks_proxy", "no_proxy"] {
            if let Some(v) = body.get(k) {
                params.insert(k.into(), v.clone());
            }
        }
        handler.handle(&ctx, "proxy.configure", &Value::Object(params))
    };
    match result {
        Ok(v) => Response::ok_json(v),
        Err(e) => Response::err(400, "Bad Request", &e.to_string()),
    }
}

fn api_theme(body: &[u8]) -> Response {
    let body = parse_json_body(body);
    let name = match body.get("name").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => return Response::err(400, "Bad Request", "missing name"),
    };
    let preset = match find_theme(&name) {
        Some(p) => p,
        None => return Response::err(400, "Bad Request", "unknown theme name"),
    };
    if let Err(e) = save_theme_to_disk(&preset) {
        return Response::err(500, "Internal Error", &e.to_string());
    }
    Response::ok_json(json!({
        "applied": true,
        "theme": preset.id,
        "color_scheme": preset.scheme,
    }))
}

fn api_recording(handler: &McpHandler, body: &[u8], start: bool) -> Response {
    let body = parse_json_body(body);
    let pane_id = match body.get("pane_id").and_then(|v| v.as_u64()) {
        Some(p) => p,
        None => return Response::err(400, "Bad Request", "missing pane_id"),
    };
    let method = if start {
        "session.recording_start"
    } else {
        "session.recording_stop"
    };
    let ctx = crate::mcp::handler::ConnectionContext::internal("web_settings");
    match handler.handle(&ctx, method, &json!({"id": pane_id})) {
        Ok(v) => Response::ok_json(v),
        Err(e) => Response::err(500, "Internal Error", &e.to_string()),
    }
}

fn api_sessions(handler: &McpHandler, query: &str) -> Response {
    let mut params = serde_json::Map::new();
    if let Some(p) = parse_query(query, "project") {
        params.insert("project".into(), Value::String(p));
    }
    let ctx = crate::mcp::handler::ConnectionContext::internal("web_settings");
    match handler.handle(&ctx, "session.recording_list", &Value::Object(params)) {
        // The MCP method returns an array directly; wrap it for the SPA so
        // it can grow `total`/etc. fields later without breaking clients.
        Ok(v) => Response::ok_json(json!({"sessions": v})),
        Err(e) => Response::err(500, "Internal Error", &e.to_string()),
    }
}

fn api_session_markdown(handler: &McpHandler, path: &str) -> Response {
    // /api/sessions/<id>/markdown
    let stripped = match path.strip_prefix("/api/sessions/") {
        Some(s) => s,
        None => return Response::err(404, "Not Found", "no such session"),
    };
    let id = match stripped.strip_suffix("/markdown") {
        Some(s) => s,
        None => return Response::err(404, "Not Found", "no such session"),
    };
    let id = match urldecode(id) {
        Some(s) => s,
        None => return Response::err(400, "Bad Request", "invalid session id encoding"),
    };
    let ctx = crate::mcp::handler::ConnectionContext::internal("web_settings");
    match handler.handle(&ctx, "session.recording_read", &json!({"session_id": id})) {
        Ok(v) => {
            let md = v
                .get("markdown")
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .to_string();
            Response::text(
                200,
                "OK",
                "text/plain; charset=utf-8",
                md.into_bytes(),
            )
        }
        Err(e) => Response::err(404, "Not Found", &e.to_string()),
    }
}

// --- Helpers ---------------------------------------------------------------

fn parse_query(query: &str, key: &str) -> Option<String> {
    for pair in query.split('&') {
        if pair.is_empty() {
            continue;
        }
        let mut it = pair.splitn(2, '=');
        let k = it.next().unwrap_or("");
        let v = it.next().unwrap_or("");
        if k == key {
            return urldecode(v);
        }
    }
    None
}

fn urldecode(s: &str) -> Option<String> {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'%' && i + 2 < bytes.len() {
            let hi = (bytes[i + 1] as char).to_digit(16)?;
            let lo = (bytes[i + 2] as char).to_digit(16)?;
            out.push(((hi << 4) | lo) as u8);
            i += 3;
        } else if b == b'+' {
            out.push(b' ');
            i += 1;
        } else {
            out.push(b);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

struct ThemePreset {
    id: &'static str,
    name: &'static str,
    scheme: &'static str,
}

fn theme_presets() -> &'static [ThemePreset] {
    &[
        ThemePreset {
            id: "standard",
            name: "Standard",
            scheme: "Catppuccin Mocha",
        },
        ThemePreset {
            id: "midnight",
            name: "Midnight",
            scheme: "Tokyo Night",
        },
        ThemePreset {
            id: "daylight",
            name: "Daylight",
            scheme: "Builtin Solarized Light",
        },
        ThemePreset {
            id: "classic",
            name: "Classic",
            scheme: "Builtin Tango Dark",
        },
    ]
}

fn find_theme(id: &str) -> Option<&'static ThemePreset> {
    theme_presets().iter().find(|p| p.id == id)
}

fn theme_config_path() -> std::path::PathBuf {
    dirs_next::home_dir()
        .unwrap_or_default()
        .join(".unterm")
        .join("theme.json")
}

fn save_theme_to_disk(preset: &ThemePreset) -> Result<()> {
    let path = theme_config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let value = json!({
        "theme": preset.id,
        "name": preset.name,
        "color_scheme": preset.scheme,
    });
    std::fs::write(path, serde_json::to_string_pretty(&value)?)?;
    Ok(())
}

fn current_theme_id() -> String {
    let path = theme_config_path();
    std::fs::read_to_string(path)
        .ok()
        .and_then(|content| serde_json::from_str::<Value>(&content).ok())
        .and_then(|v| v.get("theme").and_then(|t| t.as_str()).map(str::to_string))
        .unwrap_or_else(|| "standard".to_string())
}

fn current_project_info() -> Value {
    use mux::pane::CachePolicy;
    let Some(mux) = mux::Mux::try_get() else {
        return json!({"path": null, "slug": null});
    };
    // Pick the lowest-numbered active pane as a best-effort "current".
    let panes = mux.iter_panes();
    let pane = panes.into_iter().min_by_key(|p| p.pane_id());
    let Some(pane) = pane else {
        return json!({"path": null, "slug": null});
    };
    if let Some(url) = pane.get_current_working_dir(CachePolicy::AllowStale) {
        if let Ok(path) = url.to_file_path() {
            let abs = path.display().to_string();
            let slug = path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.to_string());
            return json!({"path": abs, "slug": slug});
        }
    }
    json!({"path": null, "slug": null})
}

fn current_recording_info() -> Value {
    let Some(mux) = mux::Mux::try_get() else {
        return json!({"active": false});
    };
    for pane in mux.iter_panes() {
        if crate::recording::recorder::current_session(pane.pane_id()).is_some() {
            let st = crate::recording::recording_status(pane.pane_id());
            return json!({
                "active": true,
                "pane_id": pane.pane_id(),
                "status": st,
            });
        }
    }
    json!({"active": false})
}

fn sessions_path_string() -> String {
    dirs_next::home_dir()
        .unwrap_or_default()
        .join(".unterm")
        .join("sessions")
        .display()
        .to_string()
}
