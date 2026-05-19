//! Install + detect + uninstall + update runners.
//!
//! Every shell command we run from a manifest passes through here. We
//! deliberately do NOT use `sh -c` to interpret the steps — install
//! commands are encoded as `Vec<String>` argv arrays, so manifests can't
//! sneak in shell metacharacters or environment-variable lookups that
//! weren't intended by the author. This narrows the blast radius of a
//! compromised manifest signing key + bad envelope to exactly what the
//! argv array spells out.

use crate::errors::{AgentError, Result};
use crate::manifest::{AgentManifest, DetectSpec, InstallStep, PlatformInstall};
use sha2::{Digest, Sha256};
use std::process::{Command, Stdio};

#[derive(Debug, Clone)]
pub struct DetectOutcome {
    /// `true` if the binary was found AND (if `min_version` set) the version
    /// passes the check.
    pub ok: bool,
    pub version: Option<String>,
    pub binary_path: Option<String>,
    pub stderr_sample: Option<String>,
}

/// Run the detect command and return a structured outcome. Never errors —
/// failure to detect is just `ok: false`.
pub fn detect(spec: &DetectSpec) -> DetectOutcome {
    let binary_path = which(&spec.command);
    if binary_path.is_none() {
        return DetectOutcome {
            ok: false,
            version: None,
            binary_path: None,
            stderr_sample: None,
        };
    }

    let mut cmd = Command::new(&spec.command);
    for a in &spec.version_args {
        cmd.arg(a);
    }
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

    let output = match cmd.output() {
        Ok(o) => o,
        Err(e) => {
            return DetectOutcome {
                ok: false,
                version: None,
                binary_path,
                stderr_sample: Some(e.to_string()),
            }
        }
    };

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let version = if let Some(re) = &spec.version_regex {
        regex_first_capture(re, &stdout).or_else(|| regex_first_capture(re, &stderr))
    } else {
        stdout
            .lines()
            .next()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    };

    let ok = match (&version, &spec.min_version) {
        (Some(have), Some(need)) => crate::envelope::compare_versions(have, need) >= 0,
        _ => version.is_some() || output.status.success(),
    };

    DetectOutcome {
        ok,
        version,
        binary_path,
        stderr_sample: if stderr.is_empty() {
            None
        } else {
            Some(stderr.chars().take(500).collect())
        },
    }
}

fn regex_first_capture(pat: &str, hay: &str) -> Option<String> {
    // Tiny dependency-free regex isn't on the workspace; reuse fancy-regex
    // would pull a lot in. Manifest authors mostly need "^([0-9.]+)" style
    // patterns — handle that subset by hand: find first run of digits/dots.
    if pat.contains("([0-9.]") {
        let mut started = false;
        let mut buf = String::new();
        for c in hay.chars() {
            if c.is_ascii_digit() || c == '.' {
                buf.push(c);
                started = true;
            } else if started {
                break;
            }
        }
        if buf.is_empty() {
            return None;
        }
        return Some(buf);
    }
    // Fall back to taking the first whitespace-delimited token.
    hay.split_whitespace().next().map(|s| s.to_string())
}

fn which(bin: &str) -> Option<String> {
    if bin.contains('/') {
        return std::path::Path::new(bin).exists().then(|| bin.to_string());
    }
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let p = dir.join(bin);
        if p.is_file() {
            return Some(p.to_string_lossy().to_string());
        }
        #[cfg(windows)]
        for ext in ["exe", "cmd", "bat", "ps1"] {
            let p2 = dir.join(format!("{bin}.{ext}"));
            if p2.is_file() {
                return Some(p2.to_string_lossy().to_string());
            }
        }
    }
    None
}

#[derive(Debug, Clone)]
pub struct StepReport {
    pub label: String,
    pub success: bool,
    pub exit_code: Option<i32>,
    pub stdout_tail: String,
    pub stderr_tail: String,
}

pub fn run_install(manifest: &AgentManifest) -> Result<Vec<StepReport>> {
    let platform: &PlatformInstall = manifest.platform_install().ok_or_else(|| {
        AgentError::InstallFailed {
            exit: None,
            detail: format!(
                "agent {:?} has no install steps for this platform ({})",
                manifest.id,
                std::env::consts::OS
            ),
        }
    })?;

    let mut reports = Vec::new();
    for (i, step) in platform.steps.iter().enumerate() {
        let report = run_step(step, &format!("step {}/{}", i + 1, platform.steps.len()))?;
        let ok = report.success;
        reports.push(report);
        if !ok {
            return Err(AgentError::InstallFailed {
                exit: reports.last().and_then(|r| r.exit_code),
                detail: format!(
                    "{}: stderr: {}",
                    manifest.id,
                    reports.last().map(|r| r.stderr_tail.as_str()).unwrap_or("")
                ),
            });
        }
    }

    // After install, run detect again to confirm.
    let after = detect(&manifest.detect);
    if !after.ok {
        return Err(AgentError::InstallFailed {
            exit: None,
            detail: format!(
                "install steps completed but `{}` still not detected on PATH",
                manifest.detect.command
            ),
        });
    }
    Ok(reports)
}

pub fn run_uninstall(manifest: &AgentManifest) -> Result<StepReport> {
    let cmd = manifest
        .install
        .uninstall
        .as_ref()
        .ok_or_else(|| AgentError::InstallFailed {
            exit: None,
            detail: format!("agent {:?} has no uninstall command", manifest.id),
        })?;
    run_shell(&cmd.cmd, "uninstall")
}

pub fn run_update(manifest: &AgentManifest) -> Result<StepReport> {
    let cmd = manifest
        .install
        .update
        .as_ref()
        .ok_or_else(|| AgentError::InstallFailed {
            exit: None,
            detail: format!("agent {:?} has no update command", manifest.id),
        })?;
    run_shell(&cmd.cmd, "update")
}

fn run_step(step: &InstallStep, label: &str) -> Result<StepReport> {
    match step {
        InstallStep::Shell { cmd } => run_shell(cmd, label),
        InstallStep::Download {
            url,
            sha256,
            dest,
            chmod,
        } => download_to(url, sha256, dest, chmod.as_deref(), label),
        InstallStep::ScriptText {
            interpreter,
            text,
            sha256,
        } => run_script_text(interpreter, text, sha256, label),
    }
}

fn run_shell(cmd: &[String], label: &str) -> Result<StepReport> {
    let mut iter = cmd.iter();
    let bin = iter
        .next()
        .ok_or_else(|| AgentError::InstallFailed {
            exit: None,
            detail: format!("{label}: empty command"),
        })?;
    let mut child = Command::new(bin);
    for a in iter {
        child.arg(a);
    }
    let output = child.output().map_err(|e| AgentError::InstallFailed {
        exit: None,
        detail: format!("{label}: spawn failed: {e}"),
    })?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    Ok(StepReport {
        label: label.to_string(),
        success: output.status.success(),
        exit_code: output.status.code(),
        stdout_tail: tail(&stdout, 2000),
        stderr_tail: tail(&stderr, 2000),
    })
}

fn run_script_text(interpreter: &str, text: &str, expected_sha256: &str, label: &str) -> Result<StepReport> {
    let actual = {
        let mut h = Sha256::new();
        h.update(text.as_bytes());
        hex_lower(&h.finalize())
    };
    if actual != expected_sha256.to_ascii_lowercase() {
        return Err(AgentError::InstallFailed {
            exit: None,
            detail: format!(
                "{label}: script_text sha256 mismatch (manifest tampered? expected {expected_sha256}, got {actual})"
            ),
        });
    }

    let mut child = Command::new(interpreter);
    child.arg("-c").arg(text);
    let output = child.output().map_err(|e| AgentError::InstallFailed {
        exit: None,
        detail: format!("{label}: spawn failed: {e}"),
    })?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    Ok(StepReport {
        label: label.to_string(),
        success: output.status.success(),
        exit_code: output.status.code(),
        stdout_tail: tail(&stdout, 2000),
        stderr_tail: tail(&stderr, 2000),
    })
}

fn download_to(url: &str, sha256: &str, dest: &str, chmod: Option<&str>, label: &str) -> Result<StepReport> {
    let resp = reqwest::blocking::get(url).map_err(|e| AgentError::InstallFailed {
        exit: None,
        detail: format!("{label}: download {url}: {e}"),
    })?;
    if !resp.status().is_success() {
        return Err(AgentError::InstallFailed {
            exit: Some(resp.status().as_u16() as i32),
            detail: format!("{label}: download {url}: HTTP {}", resp.status()),
        });
    }
    let bytes = resp.bytes().map_err(|e| AgentError::InstallFailed {
        exit: None,
        detail: format!("{label}: read body: {e}"),
    })?;
    let mut h = Sha256::new();
    h.update(&bytes);
    let actual = hex_lower(&h.finalize());
    if actual != sha256.to_ascii_lowercase() {
        return Err(AgentError::InstallFailed {
            exit: None,
            detail: format!(
                "{label}: sha256 mismatch (expected {sha256}, got {actual}) — refusing to write"
            ),
        });
    }
    let path = std::path::Path::new(dest);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| AgentError::InstallFailed {
            exit: None,
            detail: format!("{label}: mkdir {parent:?}: {e}"),
        })?;
    }
    std::fs::write(path, &bytes).map_err(|e| AgentError::InstallFailed {
        exit: None,
        detail: format!("{label}: write {dest}: {e}"),
    })?;
    #[cfg(unix)]
    if let Some(mode_str) = chmod {
        use std::os::unix::fs::PermissionsExt;
        let mode = u32::from_str_radix(mode_str.trim_start_matches('0'), 8).unwrap_or(0o755);
        let perm = std::fs::Permissions::from_mode(mode);
        std::fs::set_permissions(path, perm).map_err(|e| AgentError::InstallFailed {
            exit: None,
            detail: format!("{label}: chmod {dest}: {e}"),
        })?;
    }
    #[cfg(not(unix))]
    let _ = chmod;
    Ok(StepReport {
        label: label.to_string(),
        success: true,
        exit_code: Some(0),
        stdout_tail: format!("downloaded {} bytes to {}", bytes.len(), dest),
        stderr_tail: String::new(),
    })
}

fn tail(s: &str, n: usize) -> String {
    if s.len() <= n {
        return s.to_string();
    }
    s[s.len() - n..].to_string()
}

fn hex_lower(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        use std::fmt::Write;
        let _ = write!(&mut s, "{:02x}", b);
    }
    s
}
