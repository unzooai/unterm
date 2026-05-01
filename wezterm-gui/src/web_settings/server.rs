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

struct Response {
    status: u16,
    reason: &'static str,
    content_type: &'static str,
    body: Vec<u8>,
    extra_headers: Vec<(&'static str, String)>,
}

impl Response {
    fn json(status: u16, reason: &'static str, value: Value) -> Self {
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

    fn ok_json(value: Value) -> Self {
        Self::json(200, "OK", value)
    }

    fn err(status: u16, reason: &'static str, message: &str) -> Self {
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
        // Re-read from disk so the response always matches the on-disk
        // server.json (in case anything edits it externally).
        let info = server_info::read();
        return Response::ok_json(json!({
            "auth_token": info.auth_token,
            "mcp_port": info.mcp_port,
            "http_port": info.http_port,
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
        ("POST", "/api/recording/start") => api_recording(handler, &req.body, true),
        ("POST", "/api/recording/stop") => api_recording(handler, &req.body, false),
        ("GET", "/api/sessions") => api_sessions(handler, &req.query),
        ("GET", p) if p.starts_with("/api/sessions/") && p.ends_with("/markdown") => {
            api_session_markdown(handler, p)
        }
        ("GET", "/api/i18n") => api_i18n_state(),
        ("POST", "/api/i18n") => api_i18n_set(&req.body),
        ("GET", p) if p.starts_with("/api/i18n/") => {
            let code = &p["/api/i18n/".len()..];
            api_i18n_dict(code)
        }
        _ => Response::err(404, "Not Found", "no such route"),
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
    let info = server_info::read();
    let proxy = handler
        .handle("proxy.status", &json!({}))
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

fn api_proxy(handler: &McpHandler, body: &[u8]) -> Response {
    let body = parse_json_body(body);
    let enabled = body
        .get("enabled")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let result = if !enabled {
        handler.handle("proxy.disable", &json!({}))
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
        handler.handle("proxy.configure", &Value::Object(params))
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
    match handler.handle(method, &json!({"id": pane_id})) {
        Ok(v) => Response::ok_json(v),
        Err(e) => Response::err(500, "Internal Error", &e.to_string()),
    }
}

fn api_sessions(handler: &McpHandler, query: &str) -> Response {
    let mut params = serde_json::Map::new();
    if let Some(p) = parse_query(query, "project") {
        params.insert("project".into(), Value::String(p));
    }
    match handler.handle("session.recording_list", &Value::Object(params)) {
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
    match handler.handle("session.recording_read", &json!({"session_id": id})) {
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
