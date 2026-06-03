//! Minimal client for the Clash / mihomo external controller.
//!
//! The whole point of "auto-rotation you build by clicking" is that Unterm
//! reads the proxy nodes straight out of the user's proxy software instead of
//! making them hand-type endpoint URLs. The de-facto standard local control
//! plane is the Clash / mihomo REST API (Clash, Clash Verge, ClashX, mihomo,
//! sing-box's clash-api) — so we speak that.
//!
//! Two transports:
//!   * **Unix socket** — Clash Verge rev's default (`external-controller-unix`,
//!     e.g. `/var/tmp/verge/verge-mihomo.sock`); no auth on the socket.
//!   * **TCP** — classic `external-controller: 127.0.0.1:9090` with an optional
//!     bearer `secret`.
//!
//! We discover the endpoint (known socket path → config files → common ports)
//! and talk just enough HTTP/1.1 to GET `/version` + `/proxies`, run a
//! `/proxies/{name}/delay` health check, and `PUT /proxies/{group}` to switch
//! the active node. No external HTTP crate so this stays dependency-free and
//! works over a unix socket (which most HTTP clients can't do).

use anyhow::{anyhow, bail, Result};
use serde_json::Value;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::time::Duration;

#[cfg(unix)]
use std::os::unix::net::UnixStream;

/// A resolved Clash/mihomo controller we can talk to.
#[derive(Clone, Debug, serde::Serialize)]
#[serde(tag = "transport", rename_all = "lowercase")]
pub enum ClashEndpoint {
    /// Unix domain socket (Clash Verge rev). No auth.
    #[cfg(unix)]
    Unix { path: String },
    /// TCP `host:port` with an optional bearer secret.
    Tcp {
        addr: String,
        #[serde(skip)]
        secret: Option<String>,
    },
}

impl ClashEndpoint {
    /// Human label for the status panel.
    pub fn label(&self) -> String {
        match self {
            #[cfg(unix)]
            ClashEndpoint::Unix { path } => format!("unix:{path}"),
            ClashEndpoint::Tcp { addr, .. } => format!("tcp:{addr}"),
        }
    }
}

/// Default URL mihomo uses to time a node, and a sane health-check timeout.
pub const DELAY_TEST_URL: &str = "http://www.gstatic.com/generate_204";
const CONNECT_TIMEOUT: Duration = Duration::from_millis(800);
const IO_TIMEOUT: Duration = Duration::from_millis(4000);

/// Find a live controller. Tries, in order: the known Clash Verge socket, any
/// `external-controller-unix` / `external-controller` found in common config
/// files, then common TCP ports. Returns the first that answers `/version`.
pub fn discover() -> Option<ClashEndpoint> {
    for ep in candidate_endpoints() {
        if version(&ep).is_ok() {
            return Some(ep);
        }
    }
    None
}

static CACHED: std::sync::Mutex<Option<ClashEndpoint>> = std::sync::Mutex::new(None);

/// Like [`discover`] but remembers the last working endpoint so the status
/// panel and the rotation monitor don't re-probe every TCP port each call.
/// Re-validates the cached endpoint with a cheap `/version`; on failure it
/// re-discovers (the user may have restarted/switched proxy software).
pub fn discover_cached() -> Option<ClashEndpoint> {
    if let Ok(mut guard) = CACHED.lock() {
        if let Some(ep) = guard.as_ref() {
            if version(ep).is_ok() {
                return Some(ep.clone());
            }
        }
        let found = discover();
        *guard = found.clone();
        found
    } else {
        discover()
    }
}

fn candidate_endpoints() -> Vec<ClashEndpoint> {
    let mut out: Vec<ClashEndpoint> = Vec::new();

    // 1) Clash Verge rev's well-known unix socket.
    #[cfg(unix)]
    {
        for p in [
            "/var/tmp/verge/verge-mihomo.sock",
            "/tmp/verge/verge-mihomo.sock",
        ] {
            if std::path::Path::new(p).exists() {
                out.push(ClashEndpoint::Unix { path: p.to_string() });
            }
        }
    }

    // 2) Controllers declared in config files.
    for (ctrl, sock, secret) in scan_configs() {
        #[cfg(unix)]
        if let Some(s) = sock {
            if std::path::Path::new(&s).exists()
                && !out.iter().any(|e| matches!(e, ClashEndpoint::Unix { path } if path == &s))
            {
                out.push(ClashEndpoint::Unix { path: s });
            }
        }
        #[cfg(not(unix))]
        let _ = sock;
        if let Some(addr) = ctrl.filter(|a| !a.is_empty()) {
            let addr = normalize_addr(&addr);
            out.push(ClashEndpoint::Tcp { addr, secret: secret.clone() });
        }
    }

    // 3) Common TCP controller ports (no secret).
    for port in [9090u16, 9097, 9091, 6170] {
        let addr = format!("127.0.0.1:{port}");
        if !out.iter().any(|e| matches!(e, ClashEndpoint::Tcp { addr: a, .. } if a == &addr)) {
            out.push(ClashEndpoint::Tcp { addr, secret: None });
        }
    }
    out
}

/// `:9090` / `0.0.0.0:9090` → `127.0.0.1:9090`; otherwise as-is.
fn normalize_addr(a: &str) -> String {
    let a = a.trim();
    if let Some(rest) = a.strip_prefix(':') {
        format!("127.0.0.1:{rest}")
    } else if let Some(rest) = a.strip_prefix("0.0.0.0:") {
        format!("127.0.0.1:{rest}")
    } else {
        a.to_string()
    }
}

/// Pull `(external-controller, external-controller-unix, secret)` out of the
/// usual Clash/mihomo config locations. Best-effort line scraping — we don't
/// pull in a YAML parser for three fields.
fn scan_configs() -> Vec<(Option<String>, Option<String>, Option<String>)> {
    let mut results = Vec::new();
    let home = dirs_next::home_dir().unwrap_or_default();
    let mut paths: Vec<PathBuf> = vec![
        home.join(".config/clash/config.yaml"),
        home.join(".config/mihomo/config.yaml"),
    ];
    // Clash Verge rev (macOS / Linux).
    let verge = home.join("Library/Application Support/io.github.clash-verge-rev.clash-verge-rev");
    paths.push(verge.join("clash-verge.yaml"));
    paths.push(verge.join("config.yaml"));
    paths.push(home.join(".local/share/io.github.clash-verge-rev.clash-verge-rev/clash-verge.yaml"));

    for p in paths {
        let Ok(text) = std::fs::read_to_string(&p) else { continue };
        let mut ctrl = None;
        let mut sock = None;
        let mut secret = None;
        for line in text.lines() {
            let l = line.trim();
            if let Some(v) = l.strip_prefix("external-controller-unix:") {
                sock = Some(unquote(v));
            } else if let Some(v) = l.strip_prefix("external-controller:") {
                ctrl = Some(unquote(v));
            } else if let Some(v) = l.strip_prefix("secret:") {
                let s = unquote(v);
                if !s.is_empty() {
                    secret = Some(s);
                }
            }
        }
        if ctrl.is_some() || sock.is_some() {
            results.push((ctrl, sock, secret));
        }
    }
    results
}

fn unquote(v: &str) -> String {
    v.trim()
        .trim_matches('"')
        .trim_matches('\'')
        .trim()
        .to_string()
}

// ---------------------------------------------------------------------------
// API calls
// ---------------------------------------------------------------------------

/// `GET /version` → mihomo version string.
pub fn version(ep: &ClashEndpoint) -> Result<String> {
    let v = request(ep, "GET", "/version", None)?;
    Ok(v.get("version")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string())
}

/// `GET /proxies` → the raw proxies map (`{name: {type, now, all, history…}}`).
pub fn proxies(ep: &ClashEndpoint) -> Result<Value> {
    let v = request(ep, "GET", "/proxies", None)?;
    Ok(v.get("proxies").cloned().unwrap_or(v))
}

/// `GET /proxies/{name}/delay` → latency in ms, or Err if it timed out / failed.
pub fn delay(ep: &ClashEndpoint, name: &str, url: &str, timeout_ms: u64) -> Result<u64> {
    let path = format!(
        "/proxies/{}/delay?url={}&timeout={}",
        urlencode(name),
        urlencode(url),
        timeout_ms
    );
    let v = request(ep, "GET", &path, None)?;
    v.get("delay")
        .and_then(|d| d.as_u64())
        .ok_or_else(|| anyhow!("node {name} did not return a delay"))
}

/// `PUT /proxies/{group}` `{"name": node}` — point a Selector group at a node.
pub fn select(ep: &ClashEndpoint, group: &str, name: &str) -> Result<()> {
    let body = serde_json::json!({ "name": name }).to_string();
    let path = format!("/proxies/{}", urlencode(group));
    request(ep, "PUT", &path, Some(&body))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Tiny HTTP/1.1 over Unix socket or TCP
// ---------------------------------------------------------------------------

fn request(ep: &ClashEndpoint, method: &str, path: &str, body: Option<&str>) -> Result<Value> {
    let secret = match ep {
        ClashEndpoint::Tcp { secret, .. } => secret.clone(),
        #[cfg(unix)]
        ClashEndpoint::Unix { .. } => None,
    };
    let raw = build_request(method, path, body, secret.as_deref());

    let resp = match ep {
        #[cfg(unix)]
        ClashEndpoint::Unix { path: sock } => {
            let stream = UnixStream::connect(sock)?;
            stream.set_read_timeout(Some(IO_TIMEOUT))?;
            stream.set_write_timeout(Some(IO_TIMEOUT))?;
            roundtrip(stream, &raw)?
        }
        ClashEndpoint::Tcp { addr, .. } => {
            let sockaddr = addr
                .parse()
                .map_err(|_| anyhow!("bad controller address {addr}"))?;
            let stream = TcpStream::connect_timeout(&sockaddr, CONNECT_TIMEOUT)?;
            stream.set_read_timeout(Some(IO_TIMEOUT))?;
            stream.set_write_timeout(Some(IO_TIMEOUT))?;
            roundtrip(stream, &raw)?
        }
    };

    let (status, body) = parse_response(&resp)?;
    if !(200..300).contains(&status) {
        bail!("controller returned HTTP {status}");
    }
    if body.trim().is_empty() {
        return Ok(Value::Null); // e.g. PUT select → 204
    }
    Ok(serde_json::from_str(&body)?)
}

fn build_request(method: &str, path: &str, body: Option<&str>, secret: Option<&str>) -> Vec<u8> {
    let body = body.unwrap_or("");
    let auth = secret
        .map(|s| format!("Authorization: Bearer {s}\r\n"))
        .unwrap_or_default();
    format!(
        "{method} {path} HTTP/1.1\r\n\
         Host: localhost\r\n\
         {auth}\
         Content-Type: application/json\r\n\
         Content-Length: {len}\r\n\
         Connection: close\r\n\
         \r\n\
         {body}",
        len = body.len()
    )
    .into_bytes()
}

/// Write the request, read the whole response (the server closes the
/// connection because we send `Connection: close`).
fn roundtrip<S: Read + Write>(mut stream: S, raw: &[u8]) -> Result<Vec<u8>> {
    stream.write_all(raw)?;
    stream.flush()?;
    let mut buf = Vec::new();
    // read_to_end stops at EOF (server honours Connection: close); the read
    // timeout is a backstop against a server that doesn't.
    match stream.read_to_end(&mut buf) {
        Ok(_) => Ok(buf),
        // A timeout after we already have data is fine — mihomo sometimes holds
        // the socket; we parse what we got.
        Err(e) if !buf.is_empty() => {
            log::debug!("clash controller read ended early: {e}");
            Ok(buf)
        }
        Err(e) => Err(e.into()),
    }
}

/// Split an HTTP response into `(status_code, decoded_body)`, handling
/// `Transfer-Encoding: chunked`.
fn parse_response(raw: &[u8]) -> Result<(u16, String)> {
    let split = raw
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .ok_or_else(|| anyhow!("malformed HTTP response (no header terminator)"))?;
    let head = String::from_utf8_lossy(&raw[..split]);
    let body_bytes = &raw[split + 4..];

    let status = head
        .lines()
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|c| c.parse::<u16>().ok())
        .ok_or_else(|| anyhow!("malformed HTTP status line"))?;

    let chunked = head
        .lines()
        .any(|l| l.to_ascii_lowercase().starts_with("transfer-encoding:") && l.to_ascii_lowercase().contains("chunked"));

    let body = if chunked {
        dechunk(body_bytes)
    } else {
        String::from_utf8_lossy(body_bytes).to_string()
    };
    Ok((status, body))
}

/// Decode HTTP chunked transfer encoding into the raw body string.
fn dechunk(mut data: &[u8]) -> String {
    let mut out = Vec::new();
    loop {
        // chunk-size line up to CRLF
        let Some(nl) = data.windows(2).position(|w| w == b"\r\n") else { break };
        let size_str = String::from_utf8_lossy(&data[..nl]);
        let size = usize::from_str_radix(size_str.trim().split(';').next().unwrap_or("0").trim(), 16)
            .unwrap_or(0);
        data = &data[nl + 2..];
        if size == 0 || data.len() < size {
            if size > 0 && !data.is_empty() {
                out.extend_from_slice(&data[..data.len().min(size)]);
            }
            break;
        }
        out.extend_from_slice(&data[..size]);
        data = &data[size..];
        // trailing CRLF after chunk
        if data.len() >= 2 {
            data = &data[2..];
        }
    }
    String::from_utf8_lossy(&out).to_string()
}

/// Percent-encode a path/query component (node and group names are full of
/// CJK, spaces, and emoji like `♻️ 手动选择节点`).
fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3);
    for b in s.as_bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_addr_forms() {
        assert_eq!(normalize_addr(":9090"), "127.0.0.1:9090");
        assert_eq!(normalize_addr("0.0.0.0:9097"), "127.0.0.1:9097");
        assert_eq!(normalize_addr("127.0.0.1:9090"), "127.0.0.1:9090");
    }

    #[test]
    fn urlencode_cjk_and_emoji() {
        assert_eq!(urlencode("a-b_c.~"), "a-b_c.~");
        assert!(urlencode("♻️ 手动").contains('%'));
        assert!(!urlencode("lite-香港-01").starts_with("lite%"));
    }

    #[test]
    fn dechunk_basic() {
        // "Wiki" + "pedia" chunked
        let raw = b"4\r\nWiki\r\n5\r\npedia\r\n0\r\n\r\n";
        assert_eq!(dechunk(raw), "Wikipedia");
    }

    #[test]
    fn parse_response_content_length() {
        let raw = b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n{}";
        let (status, body) = parse_response(raw).unwrap();
        assert_eq!(status, 200);
        assert_eq!(body, "{}");
    }
}
