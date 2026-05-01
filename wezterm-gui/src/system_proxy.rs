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

/// Try, in order:
///   1. The OS's own configured proxy (`scutil --proxy` on macOS,
///      `gsettings`/env on Linux, registry on Windows).
///   2. The current process's `HTTPS_PROXY`/`HTTP_PROXY`/`ALL_PROXY` env.
///   3. A short scan of the most common local proxy ports.
/// Return None when nothing is reachable.
pub fn detect() -> Option<DetectedProxy> {
    if let Some(found) = detect_os() {
        if probe_endpoint(&found).unwrap_or(false) {
            return Some(found);
        }
    }
    if let Some(found) = detect_env() {
        if probe_endpoint(&found).unwrap_or(false) {
            return Some(found);
        }
    }
    if let Some(found) = scan_common_ports() {
        return Some(found);
    }
    None
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
fn scan_common_ports() -> Option<DetectedProxy> {
    const COMMON: &[u16] = &[7897, 7890, 1087, 7070, 8118, 8888, 8080, 1080];
    for port in COMMON {
        let addr = format!("127.0.0.1:{}", port);
        if std::net::TcpStream::connect_timeout(
            &addr.parse().ok()?,
            std::time::Duration::from_millis(120),
        )
        .is_ok()
        {
            let url = format!("http://{}", addr);
            return Some(DetectedProxy {
                http: Some(url.clone()),
                https: Some(url),
                socks: Some(format!("socks5://{}", addr)),
                no_proxy: None,
                source: Box::leak(format!("scan:{}", port).into_boxed_str()),
            });
        }
    }
    None
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
