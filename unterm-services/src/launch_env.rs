//! The environment a command is launched with.
//!
//! Split from spawning itself, which needs a mux and a window. This half needs
//! neither: it decides what profile variables, proxy settings and encoding
//! switches a shell should start with, and a second front end needs exactly
//! the same answers.
//!
//! The Windows encoding rewrite here is the reason a shell on a Chinese
//! Windows shows its text rather than a row of boxes -- `cmd.exe` and
//! PowerShell both emit the console codepage unless told otherwise, and what
//! a terminal receives then is not UTF-8 at all.

use config::keyassignment::SpawnCommand;
use portable_pty::CommandBuilder;

pub fn apply_unterm_profile_env(cmd_builder: &mut Option<CommandBuilder>) {
    let info = crate::server_info::read_current();
    let Some(profile_id) = info.profile.as_deref() else {
        return;
    };
    if profile_id.is_empty() {
        return;
    }

    let registry = match unterm_profile::ProfileRegistry::load() {
        Ok(r) => r,
        Err(e) => {
            log::warn!("apply_unterm_profile_env: registry load failed: {e:#}");
            return;
        }
    };
    let store = match unterm_profile::default_store() {
        Ok(s) => s,
        Err(e) => {
            // Backend unavailable on this platform — expected on Linux
            // headless / Windows before backend lands. Debug level: this
            // is not an error, just "feature not active here".
            log::debug!("apply_unterm_profile_env: no secret store: {e:#}");
            return;
        }
    };
    let env = match registry.resolve_env(store.as_ref(), profile_id) {
        Ok(e) => e,
        Err(e) => {
            log::warn!("apply_unterm_profile_env: resolve_env({profile_id}) failed: {e:#}");
            return;
        }
    };
    if env.is_empty() {
        return;
    }

    // Mirror `apply_unterm_proxy_env`: if the spawn didn't customize
    // anything, we still need a CommandBuilder so we have somewhere
    // to attach env. Default prog matches what wezterm would have
    // launched anyway.
    let builder = cmd_builder.get_or_insert_with(CommandBuilder::new_default_prog);
    for (k, v) in env {
        builder.env(k, v);
    }
}

pub fn apply_unterm_proxy_env(cmd_builder: &mut Option<CommandBuilder>) {
    let Some(proxy) = read_unterm_proxy_env() else {
        return;
    };
    let builder = cmd_builder.get_or_insert_with(CommandBuilder::new_default_prog);
    for (key, value) in proxy {
        builder.env(key, value);
    }
}

pub fn apply_unterm_proxy_to_spawn(spawn: &mut SpawnCommand) {
    let Some(proxy) = read_unterm_proxy_env() else {
        return;
    };
    for (key, value) in proxy {
        spawn.set_environment_variables.insert(key, value);
    }
}

pub fn apply_unterm_proxy_to_process_env() {
    let Some(proxy) = read_unterm_proxy_env() else {
        return;
    };
    for (key, value) in proxy {
        std::env::set_var(key, value);
    }
}

/// On Windows, force UTF-8 in spawned shells so that PowerShell / cmd.exe
/// running on a zh-CN (CP936/GBK), zh-TW (CP950/Big5), ja-JP (CP932/SJIS),
/// or any other non-UTF-8 system locale still emit UTF-8 bytes — which is
/// what every other tool in our pipeline (font rendering, MCP transcripts,
/// recording redaction, agent integrations) expects.
///
/// Why we do this even though Windows Terminal mostly "just works": WT
/// relies on the user's PowerShell `$PROFILE` having
/// `[Console]::OutputEncoding = [Text.UTF8Encoding]::new()` already, or
/// on `pwsh` (PowerShell 7+) which defaults to UTF-8. Our positioning is
/// "the terminal AI agents can drive" — agents won't fix the user's
/// profile, and we can't assume PS7 either. So we wrap at spawn time.
///
/// Strategy:
///   - args == ["powershell.exe"] / ["pwsh.exe"] / etc. (just the
///     binary, nothing else): replace with the binary plus
///     `-NoLogo -NoExit -Command "<UTF-8 setup>; load $PROFILE"`.
///     The -Command runs first, then $PROFILE loads (so user
///     customizations still apply, but on top of UTF-8 defaults).
///   - args == ["cmd.exe"]: wrap with
///     `cmd.exe /D /K "chcp 65001 > nul"` — sets the console codepage
///     before the prompt appears, no command echoed.
///   - Anything else (user passed extra args, or non-shell exe): leave
///     untouched. They're customizing; we don't second-guess.
///
/// If no args are present, leave the builder alone. The mux/domain layer
/// still needs to resolve the configured default_prog; forcing it to cmd.exe
/// here makes default split panes ignore the PowerShell default on Windows.
#[cfg(windows)]
pub fn apply_unterm_windows_utf8(cmd_builder: &mut Option<CommandBuilder>) {
    let Some(builder) = cmd_builder.as_mut() else {
        return;
    };

    if builder.is_default_prog() {
        return;
    }

    let args = builder.get_argv().clone();
    if args.len() != 1 {
        return; // user passed args — don't second-guess.
    }
    let exe_str = args[0].to_string_lossy().to_lowercase();
    let basename = exe_str
        .rsplit(['\\', '/'])
        .next()
        .unwrap_or(exe_str.as_str());

    let argv = builder.get_argv_mut();
    if basename.starts_with("powershell") || basename.starts_with("pwsh") {
        // Four encoding knobs on Windows PowerShell. Missing any one
        // causes mojibake somewhere in the I/O chain. In particular,
        // omitting InputEncoding is what causes UTF-8 Chinese filenames
        // typed at the prompt to be re-decoded as CP936/GBK by PowerShell
        // before they reach cmdlets — at which point Move-Item / Get-Item
        // can't find the file and reports a corrupted name back. So we
        // set all four every time, on the same line so a profile that
        // overrides one of them only overrides that one.
        let setup = "[Console]::OutputEncoding=[Text.UTF8Encoding]::new($false);\
                     [Console]::InputEncoding=[Text.UTF8Encoding]::new($false);\
                     $OutputEncoding=[Console]::OutputEncoding;\
                     $PSDefaultParameterValues['*:Encoding']='utf8';\
                     chcp 65001>$null;\
                     if(Test-Path $PROFILE){. $PROFILE}";
        let exe = argv[0].clone();
        argv.clear();
        argv.push(exe);
        argv.push("-NoLogo".into());
        argv.push("-NoExit".into());
        argv.push("-Command".into());
        argv.push(setup.into());
    } else if basename == "cmd.exe" || basename == "cmd" {
        let exe = argv[0].clone();
        argv.clear();
        argv.push(exe);
        argv.push("/D".into());
        argv.push("/K".into());
        argv.push("chcp 65001 > nul".into());
    }
}

#[cfg(not(windows))]
pub fn apply_unterm_windows_utf8(_cmd_builder: &mut Option<CommandBuilder>) {}

/// SpawnCommand-shaped variant — same logic as `apply_unterm_windows_utf8`
/// but operating on `config::keyassignment::SpawnCommand` (used for the
/// MCP / Lua-driven spawn paths). Mutates `spawn.args` in place.
#[cfg(windows)]
pub fn apply_unterm_windows_utf8_to_spawn(spawn: &mut config::keyassignment::SpawnCommand) {
    let args = match spawn.args.as_ref() {
        None => return, // None = use default prog; the CommandBuilder path handles that.
        Some(a) if a.len() == 1 => a.clone(),
        Some(_) => return, // user passed extra args — don't wrap.
    };
    let exe_str = args[0].to_lowercase();
    let basename = exe_str
        .rsplit(['\\', '/'])
        .next()
        .unwrap_or(exe_str.as_str());

    if basename.starts_with("powershell") || basename.starts_with("pwsh") {
        // Four encoding knobs on Windows PowerShell. Missing any one
        // causes mojibake somewhere in the I/O chain. In particular,
        // omitting InputEncoding is what causes UTF-8 Chinese filenames
        // typed at the prompt to be re-decoded as CP936/GBK by PowerShell
        // before they reach cmdlets — at which point Move-Item / Get-Item
        // can't find the file and reports a corrupted name back. So we
        // set all four every time, on the same line so a profile that
        // overrides one of them only overrides that one.
        let setup = "[Console]::OutputEncoding=[Text.UTF8Encoding]::new($false);\
                     [Console]::InputEncoding=[Text.UTF8Encoding]::new($false);\
                     $OutputEncoding=[Console]::OutputEncoding;\
                     $PSDefaultParameterValues['*:Encoding']='utf8';\
                     chcp 65001>$null;\
                     if(Test-Path $PROFILE){. $PROFILE}";
        spawn.args = Some(vec![
            args[0].clone(),
            "-NoLogo".into(),
            "-NoExit".into(),
            "-Command".into(),
            setup.into(),
        ]);
    } else if basename == "cmd.exe" || basename == "cmd" {
        spawn.args = Some(vec![
            args[0].clone(),
            "/D".into(),
            "/K".into(),
            "chcp 65001 > nul".into(),
        ]);
    }
}

#[cfg(not(windows))]
pub fn apply_unterm_windows_utf8_to_spawn(_spawn: &mut config::keyassignment::SpawnCommand) {
}

pub fn read_unterm_proxy_env() -> Option<Vec<(String, String)>> {
    // Reads ~/.unterm/proxy.json (managed by the ▼ menu / Web Settings).
    // Schema: { enabled, mode: "auto" | "manual", http_proxy, socks_proxy, no_proxy }.
    // In auto mode (default), system_proxy::detect() runs at every spawn and
    // overlays whatever stale URLs are on disk — mirroring what the UI does in
    // mcp/handler.rs::load_proxy_settings() so the ▼ menu, Web Settings, and
    // spawned shells never disagree on the active proxy URL.
    let path = dirs_next::home_dir()
        .unwrap_or_default()
        .join(".unterm")
        .join("proxy.json");
    let value: serde_json::Value = std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| serde_json::json!({"enabled": false}));
    if !value
        .get("enabled")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
    {
        return None;
    }

    // Mode determines whether on-disk URLs are authoritative.
    //   - "manual" (or any explicit non-auto value): trust the URLs in
    //     proxy.json exactly. This is the user saying "I know what I want."
    //   - anything else (including missing field): treat as auto. The
    //     URLs on disk are stale state from a previous detect() and must
    //     be overlaid with a fresh detect() call. Without this, a user
    //     who changed their Clash listener from 7890 to 7897 keeps getting
    //     7890 piped into every spawned shell — which is exactly the bug
    //     that bit the v0.5.4 Windows release.
    //
    // Mirrors mcp/handler.rs::load_proxy_settings(), which has the same
    // overlay semantics for the UI/MCP side. The two paths must agree or
    // ▼ menu and Web Settings will show 7897 while spawned shells get 7890.
    let mode = value
        .get("mode")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("auto");
    let is_auto = mode != "manual";

    let manual_http = value
        .get("http_proxy")
        .and_then(serde_json::Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    let manual_socks = value
        .get("socks_proxy")
        .and_then(serde_json::Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    let manual_no = value
        .get("no_proxy")
        .and_then(serde_json::Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_string);

    // In auto mode: always run detect() and let it win over stale on-disk URLs.
    // In manual mode: only fall back to detect() when both URL fields are blank.
    let detected = if is_auto || (manual_http.is_none() && manual_socks.is_none()) {
        crate::system_proxy::detect()
    } else {
        None
    };

    let detected_http = detected
        .as_ref()
        .and_then(|d| d.primary_http().map(str::to_string));
    let detected_socks = detected.as_ref().and_then(|d| d.socks.clone());
    let detected_no = detected.as_ref().and_then(|d| d.no_proxy.clone());

    let (http, socks, no_proxy) = if is_auto {
        // Auto mode: detected wins; manual values only fill blanks.
        let http = detected_http.or(manual_http);
        let socks = detected_socks.or(manual_socks);
        let no_proxy = detected_no
            .or(manual_no)
            .unwrap_or_else(|| "localhost,127.0.0.1,::1".to_string());
        (http, socks, no_proxy)
    } else {
        // Manual mode: user URLs win; detected only fills blanks.
        let http = manual_http.or(detected_http);
        let socks = manual_socks.or(detected_socks);
        let no_proxy = manual_no
            .or(detected_no)
            .unwrap_or_else(|| "localhost,127.0.0.1,::1".to_string());
        (http, socks, no_proxy)
    };

    // Safety guard: never inject a dead proxy. A proxy URL piped into a shell
    // that points at a port nothing is listening on turns every command in
    // that terminal into a connection failure — which reads to the user as
    // "Unterm broke my VPN". So we TCP-probe each endpoint and drop any that
    // isn't accepting connections right now. If nothing is reachable we inject
    // nothing, and the shell talks to the network directly, exactly as if the
    // proxy toggle were off. (Auto mode still re-detects on the next spawn, so
    // the proxy reattaches automatically once the software is back up.)
    let http = http.filter(|u| proxy_endpoint_reachable(u));
    let socks = socks.filter(|u| proxy_endpoint_reachable(u));
    if http.is_none() && socks.is_none() {
        log::warn!(
            "Unterm proxy is enabled but no configured/detected proxy endpoint is reachable; \
             leaving this shell on a direct connection so it isn't cut off from the network"
        );
        return None;
    }

    let mut env = Vec::new();
    if let Some(http) = &http {
        env.push(("HTTP_PROXY".to_string(), http.clone()));
        env.push(("HTTPS_PROXY".to_string(), http.clone()));
        env.push(("http_proxy".to_string(), http.clone()));
        env.push(("https_proxy".to_string(), http.clone()));
    }
    if let Some(socks) = &socks {
        env.push(("ALL_PROXY".to_string(), socks.clone()));
        env.push(("all_proxy".to_string(), socks.clone()));
    }
    if !no_proxy.is_empty() {
        env.push(("NO_PROXY".to_string(), no_proxy.clone()));
        env.push(("no_proxy".to_string(), no_proxy));
    }

    if env.is_empty() {
        log::warn!(
            "Unterm proxy is enabled but no proxy could be detected from the OS or scan; \
             set explicit URLs in ~/.unterm/proxy.json if needed"
        );
        None
    } else {
        Some(env)
    }
}

/// TCP-probe a proxy endpoint so we never inject a dead proxy into a shell.
/// Returns false fast (250 ms) if nothing is listening on `host:port` — the
/// caller then skips injecting that URL, leaving the shell on a direct
/// connection instead of routing every command into a black hole.
pub fn proxy_endpoint_reachable(url: &str) -> bool {
    use std::net::{TcpStream, ToSocketAddrs};
    use std::time::Duration;
    // Strip scheme (http://, socks5://) and any trailing path, leaving host:port.
    let host_port = url
        .split("://")
        .nth(1)
        .unwrap_or(url)
        .split('/')
        .next()
        .unwrap_or(url);
    // Without an explicit port we can't probe; don't block injection in that case.
    if !host_port.contains(':') {
        return true;
    }
    match host_port.to_socket_addrs() {
        Ok(addrs) => addrs
            .into_iter()
            .any(|addr| TcpStream::connect_timeout(&addr, Duration::from_millis(250)).is_ok()),
        Err(_) => false,
    }
}

