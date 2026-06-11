//! Auto-detect the user's system proxy. Used so Unterm "just works" out of
//! the box for users who already configured a proxy in System Preferences /
//! Settings — no need to copy URLs into `~/.unterm/proxy.json` manually.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetectedProxy {
    pub http: Option<String>,
    pub https: Option<String>,
    pub socks: Option<String>,
    pub no_proxy: Option<String>,
    /// Free-form description of where we found the proxy, for logging /
    /// status-bar tooltips. Examples: "macOS system pref", "scan:7897".
    pub source: &'static str,
}

impl DetectedProxy {
    pub fn primary_http(&self) -> Option<&str> {
        self.https.as_deref().or(self.http.as_deref())
    }
}

/// Try, in priority order:
///   1. The OS's own configured proxy (`scutil --proxy` on macOS,
///      `gsettings`/env on Linux, registry on Windows).
///   2. The current process's `HTTPS_PROXY`/`HTTP_PROXY`/`ALL_PROXY` env.
///   3. A short scan of the most common local proxy ports.
/// Return None when nothing is reachable.
///
/// Results are cached for a few seconds. detect() is on the spawn path of
/// every pane/tab AND on the GUI startup path, and each uncached probe pass
/// costs real wall-clock time when nothing is listening: on Windows a TCP
/// connect to a closed loopback port only fails after winsock's internal
/// retry (~2s, clamped by our connect_timeout). Without the cache, a user
/// whose proxy toggle is on while their proxy app is closed pays that price
/// twice before the first prompt — the original "Unterm takes seconds to
/// start on Windows" bug.
pub fn detect() -> Option<DetectedProxy> {
    use std::sync::Mutex;
    use std::time::{Duration, Instant};

    static CACHE: Mutex<Option<(Instant, Option<DetectedProxy>)>> = Mutex::new(None);
    const TTL: Duration = Duration::from_secs(5);

    if let Some((at, cached)) = CACHE.lock().unwrap().as_ref() {
        if at.elapsed() < TTL {
            return cached.clone();
        }
    }
    let started = Instant::now();
    let fresh = detect_uncached();
    log::debug!(
        "system proxy detect: {:?} in {:?}",
        fresh.as_ref().map(|f| f.source),
        started.elapsed()
    );
    *CACHE.lock().unwrap() = Some((Instant::now(), fresh.clone()));
    fresh
}

/// The three detection stages are independent, so run them concurrently and
/// pick the winner by priority (OS > env > scan) when joining. Sequentially
/// they cost up to probe(150ms) + probe(150ms) + scan(~1s); concurrently the
/// whole pass is bounded by the slowest single stage (~150ms).
fn detect_uncached() -> Option<DetectedProxy> {
    let os = std::thread::spawn(|| {
        detect_os().filter(|found| probe_endpoint(found).unwrap_or(false))
    });
    let env = std::thread::spawn(|| {
        detect_env().filter(|found| probe_endpoint(found).unwrap_or(false))
    });
    let scan = std::thread::spawn(scan_common_ports);

    // Join in priority order and return on the first hit: when the OS proxy
    // is configured and alive (the everyday Clash/V2Ray case) this returns in
    // single-digit ms without waiting out the port sweep's timeout. Skipped
    // joins just leave their thread to expire its own connect_timeout.
    if let Some(found) = os.join().ok().flatten() {
        return Some(found);
    }
    if let Some(found) = env.join().ok().flatten() {
        return Some(found);
    }
    scan.join().ok().flatten()
}

#[cfg(target_os = "macos")]
fn detect_os() -> Option<DetectedProxy> {
    let output = std::process::Command::new("/usr/sbin/scutil")
        .arg("--proxy")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    parse_scutil(&text)
}

#[cfg(target_os = "macos")]
fn parse_scutil(text: &str) -> Option<DetectedProxy> {
    let mut http_enable = false;
    let mut http_host: Option<String> = None;
    let mut http_port: Option<u16> = None;
    let mut https_enable = false;
    let mut https_host: Option<String> = None;
    let mut https_port: Option<u16> = None;
    let mut socks_enable = false;
    let mut socks_host: Option<String> = None;
    let mut socks_port: Option<u16> = None;
    let mut exceptions: Vec<String> = Vec::new();

    for line in text.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("HTTPEnable :") {
            http_enable = rest.trim() == "1";
        } else if let Some(rest) = line.strip_prefix("HTTPProxy :") {
            http_host = Some(rest.trim().to_string());
        } else if let Some(rest) = line.strip_prefix("HTTPPort :") {
            http_port = rest.trim().parse().ok();
        } else if let Some(rest) = line.strip_prefix("HTTPSEnable :") {
            https_enable = rest.trim() == "1";
        } else if let Some(rest) = line.strip_prefix("HTTPSProxy :") {
            https_host = Some(rest.trim().to_string());
        } else if let Some(rest) = line.strip_prefix("HTTPSPort :") {
            https_port = rest.trim().parse().ok();
        } else if let Some(rest) = line.strip_prefix("SOCKSEnable :") {
            socks_enable = rest.trim() == "1";
        } else if let Some(rest) = line.strip_prefix("SOCKSProxy :") {
            socks_host = Some(rest.trim().to_string());
        } else if let Some(rest) = line.strip_prefix("SOCKSPort :") {
            socks_port = rest.trim().parse().ok();
        } else if line.contains(':')
            && (line.starts_with('0')
                || line.starts_with('1')
                || line.starts_with('2')
                || line.starts_with('3')
                || line.starts_with('4')
                || line.starts_with('5')
                || line.starts_with('6')
                || line.starts_with('7')
                || line.starts_with('8')
                || line.starts_with('9'))
        {
            // Looks like an `<index> : <value>` exceptions list line.
            if let Some(rest) = line.split_once(':').map(|(_, v)| v.trim()) {
                if !rest.is_empty() {
                    exceptions.push(rest.to_string());
                }
            }
        }
    }

    let http = if http_enable {
        match (http_host, http_port) {
            (Some(host), Some(port)) => Some(format!("http://{}:{}", host, port)),
            _ => None,
        }
    } else {
        None
    };
    let https = if https_enable {
        match (https_host, https_port) {
            (Some(host), Some(port)) => Some(format!("http://{}:{}", host, port)),
            _ => None,
        }
    } else {
        None
    };
    let socks = if socks_enable {
        match (socks_host, socks_port) {
            (Some(host), Some(port)) => Some(format!("socks5://{}:{}", host, port)),
            _ => None,
        }
    } else {
        None
    };

    if http.is_none() && https.is_none() && socks.is_none() {
        return None;
    }
    let no_proxy = if exceptions.is_empty() {
        None
    } else {
        Some(exceptions.join(","))
    };
    Some(DetectedProxy {
        http,
        https,
        socks,
        no_proxy,
        source: "macOS system pref",
    })
}

#[cfg(target_os = "windows")]
fn detect_os() -> Option<DetectedProxy> {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = hkcu
        .open_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Internet Settings")
        .ok()?;
    let enabled: u32 = key.get_value("ProxyEnable").ok()?;
    if enabled == 0 {
        return None;
    }
    let server: String = key.get_value("ProxyServer").ok()?;
    // "host:port" or "http=h:p;https=h:p;socks=h:p"
    let mut http = None;
    let mut https = None;
    let mut socks = None;
    if server.contains('=') {
        for part in server.split(';') {
            if let Some((scheme, addr)) = part.split_once('=') {
                let url = format!("http://{}", addr);
                match scheme.to_lowercase().as_str() {
                    "http" => http = Some(url),
                    "https" => https = Some(url),
                    "socks" => socks = Some(format!("socks5://{}", addr)),
                    _ => {}
                }
            }
        }
    } else {
        let url = format!("http://{}", server);
        http = Some(url.clone());
        https = Some(url);
    }
    Some(DetectedProxy {
        http,
        https,
        socks,
        no_proxy: key.get_value::<String, _>("ProxyOverride").ok(),
        source: "Windows registry",
    })
}

#[cfg(all(unix, not(target_os = "macos")))]
fn detect_os() -> Option<DetectedProxy> {
    // Probe gsettings (GNOME) — KDE-specific detection lives in env_proxy below.
    let mode = std::process::Command::new("gsettings")
        .args(["get", "org.gnome.system.proxy", "mode"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())?;
    if !mode.contains("manual") {
        return None;
    }
    fn read(schema: &str, key: &str) -> Option<String> {
        let s = std::process::Command::new("gsettings")
            .args(["get", schema, key])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())?;
        Some(s.trim().trim_matches('\'').to_string())
    }
    let http_host = read("org.gnome.system.proxy.http", "host")?;
    let http_port = read("org.gnome.system.proxy.http", "port")?;
    let socks_host = read("org.gnome.system.proxy.socks", "host").unwrap_or_default();
    let socks_port = read("org.gnome.system.proxy.socks", "port").unwrap_or_default();
    let http_url = format!("http://{}:{}", http_host, http_port);
    let socks_url = if !socks_host.is_empty() && socks_port != "0" {
        Some(format!("socks5://{}:{}", socks_host, socks_port))
    } else {
        None
    };
    Some(DetectedProxy {
        http: Some(http_url.clone()),
        https: Some(http_url),
        socks: socks_url,
        no_proxy: None,
        source: "GNOME gsettings",
    })
}

fn detect_env() -> Option<DetectedProxy> {
    let http = std::env::var("HTTP_PROXY")
        .ok()
        .or_else(|| std::env::var("http_proxy").ok());
    let https = std::env::var("HTTPS_PROXY")
        .ok()
        .or_else(|| std::env::var("https_proxy").ok());
    let socks = std::env::var("ALL_PROXY")
        .ok()
        .or_else(|| std::env::var("all_proxy").ok());
    if http.is_none() && https.is_none() && socks.is_none() {
        return None;
    }
    Some(DetectedProxy {
        http,
        https,
        socks,
        no_proxy: std::env::var("NO_PROXY").ok(),
        source: "process env",
    })
}

/// Last-ditch: probe well-known local proxy ports. Order based on what's
/// popular in the wild — Clash newer / older defaults, V2Ray, Surge, Privoxy.
///
/// Deliberately NOT in the list: 8080 and 8888. They're the default ports
/// of countless dev servers (vite preview, Tomcat, Jupyter, mitmproxy-less
/// HTTP tools), and a TCP accept is the only signal this scan has — it
/// can't tell a proxy from a web server. A false positive here injects
/// `HTTP_PROXY=127.0.0.1:8080` into every spawned shell and silently
/// routes all of its traffic into someone's dev server, which reads as
/// "Unterm broke my network". Users who really proxy on those ports can
/// set the URL explicitly in proxy.json (manual mode).
///
/// Ports are probed in parallel: each closed port eats its full 120ms
/// connect_timeout on Windows (loopback RST is swallowed by winsock's
/// connect retry, so the timeout always elapses), and a serial sweep
/// adds ~1s to whatever path called detect(). Priority among
/// concurrently-open ports is preserved by COMMON's ordering at join time.
fn scan_common_ports() -> Option<DetectedProxy> {
    const COMMON: &[u16] = &[7897, 7890, 1087, 7070, 8118, 1080];
    let probes: Vec<std::thread::JoinHandle<bool>> = COMMON
        .iter()
        .map(|&port| {
            std::thread::spawn(move || {
                let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
                std::net::TcpStream::connect_timeout(
                    &addr,
                    std::time::Duration::from_millis(120),
                )
                .is_ok()
            })
        })
        .collect();
    let open: Vec<bool> = probes
        .into_iter()
        .map(|h| h.join().unwrap_or(false))
        .collect();
    let port = COMMON
        .iter()
        .zip(&open)
        .find_map(|(&port, &is_open)| is_open.then_some(port))?;
    let addr = format!("127.0.0.1:{}", port);
    let url = format!("http://{}", addr);
    Some(DetectedProxy {
        http: Some(url.clone()),
        https: Some(url),
        socks: Some(format!("socks5://{}", addr)),
        no_proxy: None,
        source: Box::leak(format!("scan:{}", port).into_boxed_str()),
    })
}

/// Verify the detected proxy is actually reachable. Some users have leftover
/// proxy config in OS settings pointing at a dead port.
fn probe_endpoint(proxy: &DetectedProxy) -> Option<bool> {
    let url = proxy.primary_http()?;
    let addr = url.strip_prefix("http://").or_else(|| url.strip_prefix("https://"))?;
    let socket: std::net::SocketAddr = addr.parse().ok()?;
    Some(
        std::net::TcpStream::connect_timeout(&socket, std::time::Duration::from_millis(150))
            .is_ok(),
    )
}
