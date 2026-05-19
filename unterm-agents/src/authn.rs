//! Auth flow runner: walks the user through `claude login` / API-key paste /
//! whatever the manifest specifies, validates, and stores the resulting
//! credentials in the OS keychain.
//!
//! Two methods today:
//!   * `oauth_browser` — spawn a vendor login command (e.g. `claude login`)
//!     and poll its combined output for `ready_marker`. The vendor opens
//!     the user's browser; we don't need to handle the OAuth dance ourselves.
//!   * `api_key_env` — caller already has the raw key (entered in CLI or
//!     Web Settings); we optionally run a validate command to confirm it
//!     works, then write into the keychain under (profile, env_var).

use crate::errors::{AgentError, Result};
use crate::manifest::{AuthMethod, AuthSpec};
use crate::secrets::AgentSecretStore;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct AuthOutcome {
    pub method_used: String,
    pub stdout_tail: String,
    pub stderr_tail: String,
}

/// Run an oauth-style auth (spawn vendor login command, wait for marker).
pub fn run_oauth_browser(auth: &AuthSpec) -> Result<AuthOutcome> {
    let AuthMethod::OauthBrowser {
        trigger,
        ready_marker,
        timeout_s,
    } = &auth.primary
    else {
        return Err(AgentError::AuthFailed(
            "primary auth is not oauth_browser".into(),
        ));
    };

    let mut iter = trigger.cmd.iter();
    let bin = iter
        .next()
        .ok_or_else(|| AgentError::AuthFailed("oauth trigger has empty command".into()))?;
    let mut child = Command::new(bin)
        .args(iter)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null())
        .spawn()
        .map_err(|e| AgentError::AuthFailed(format!("spawn {bin}: {e}")))?;

    let (tx, rx) = mpsc::channel();
    let stdout = child.stdout.take().expect("piped");
    let stderr = child.stderr.take().expect("piped");
    let marker = ready_marker.clone();
    let tx2 = tx.clone();
    thread::spawn(move || pump(BufReader::new(stdout), marker.as_deref(), tx2, false));
    let marker2 = ready_marker.clone();
    thread::spawn(move || pump(BufReader::new(stderr), marker2.as_deref(), tx, true));

    let deadline = Instant::now() + Duration::from_secs(*timeout_s);
    let mut stdout_buf = String::new();
    let mut stderr_buf = String::new();
    let mut matched = ready_marker.is_none(); // if no marker, success once child exits 0
    while Instant::now() < deadline {
        match rx.recv_timeout(Duration::from_millis(250)) {
            Ok(PumpMsg::Line { line, is_stderr }) => {
                if is_stderr {
                    stderr_buf.push_str(&line);
                    stderr_buf.push('\n');
                } else {
                    stdout_buf.push_str(&line);
                    stdout_buf.push('\n');
                }
            }
            Ok(PumpMsg::Marker) => {
                matched = true;
                break;
            }
            Err(_) => {}
        }
        if let Ok(Some(status)) = child.try_wait() {
            if !status.success() && !matched {
                let _ = child.wait();
                return Err(AgentError::AuthFailed(format!(
                    "oauth login command exited {status}; stderr: {}",
                    tail(&stderr_buf, 600)
                )));
            }
            if matched || status.success() {
                break;
            }
        }
    }
    let _ = child.kill();
    let _ = child.wait();
    if !matched {
        return Err(AgentError::AuthFailed(format!(
            "oauth login timed out after {timeout_s}s without seeing ready marker"
        )));
    }
    Ok(AuthOutcome {
        method_used: "oauth_browser".into(),
        stdout_tail: tail(&stdout_buf, 1000),
        stderr_tail: tail(&stderr_buf, 1000),
    })
}

/// Set + validate an API key. Always operates on `auth.fallback` if it's
/// an api_key_env, otherwise on `auth.primary` (some agents make api_key
/// the only auth, no oauth).
pub fn run_api_key(
    auth: &AuthSpec,
    profile_id: &str,
    raw_key: &str,
) -> Result<AuthOutcome> {
    let method = match (&auth.primary, &auth.fallback) {
        (AuthMethod::ApiKeyEnv { .. }, _) => &auth.primary,
        (_, Some(AuthMethod::ApiKeyEnv { .. })) => auth.fallback.as_ref().unwrap(),
        _ => {
            return Err(AgentError::AuthFailed(
                "no api_key_env auth method in this manifest".into(),
            ))
        }
    };
    let AuthMethod::ApiKeyEnv {
        env_var, validate, ..
    } = method
    else {
        unreachable!()
    };
    let store = AgentSecretStore::open()?;
    store.set(profile_id, env_var, raw_key)?;

    if let Some(v) = validate {
        let mut iter = v.cmd.iter();
        let bin = iter
            .next()
            .ok_or_else(|| AgentError::AuthFailed("validate cmd is empty".into()))?;
        let mut child = Command::new(bin);
        for a in iter {
            child.arg(a);
        }
        child.env(env_var, raw_key);
        let out = child
            .output()
            .map_err(|e| AgentError::AuthFailed(format!("validate spawn: {e}")))?;
        if !out.status.success() {
            return Err(AgentError::AuthFailed(format!(
                "validate exited {:?}; stderr: {}",
                out.status.code(),
                tail(&String::from_utf8_lossy(&out.stderr), 600)
            )));
        }
    }

    Ok(AuthOutcome {
        method_used: "api_key_env".into(),
        stdout_tail: String::new(),
        stderr_tail: String::new(),
    })
}

enum PumpMsg {
    Line { line: String, is_stderr: bool },
    Marker,
}

fn pump<R: BufRead>(reader: R, marker: Option<&str>, tx: mpsc::Sender<PumpMsg>, is_stderr: bool) {
    for line in reader.lines().flatten() {
        if let Some(m) = marker {
            if line.contains(m) {
                let _ = tx.send(PumpMsg::Line {
                    line: line.clone(),
                    is_stderr,
                });
                let _ = tx.send(PumpMsg::Marker);
                return;
            }
        }
        if tx
            .send(PumpMsg::Line {
                line,
                is_stderr,
            })
            .is_err()
        {
            return;
        }
    }
}

fn tail(s: &str, n: usize) -> String {
    if s.len() <= n {
        s.to_string()
    } else {
        s[s.len() - n..].to_string()
    }
}
