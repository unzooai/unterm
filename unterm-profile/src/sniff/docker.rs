//! Docker registry auth sniffer.
//!
//! Reads `~/.docker/config.json` and pulls out the `auths` map. Each
//! entry has shape:
//!
//! ```json
//! {
//!   "auths": {
//!     "https://index.docker.io/v1/": {
//!       "auth": "<base64(user:pass)>"
//!     }
//!   }
//! }
//! ```
//!
//! Modern Docker (Desktop) uses `credsStore: "desktop"` and stores
//! the actual secrets in the OS keychain — `auths.<host>.auth` is
//! empty in that case. We surface the *host* as a candidate even
//! when the value is missing, so the wizard can prompt the user for
//! their `DOCKER_HUB_TOKEN` (or whatever) and stash it in our own
//! Keychain entry.

use anyhow::Result;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;

use super::{Candidate, CandidateSource};

#[derive(Debug, Deserialize)]
struct DockerConfig {
    #[serde(default)]
    auths: HashMap<String, AuthEntry>,
}

#[derive(Debug, Deserialize)]
struct AuthEntry {
    #[serde(default)]
    auth: Option<String>,
    #[serde(default, rename = "identityToken")]
    identity_token: Option<String>,
}

fn config_path() -> Option<PathBuf> {
    dirs_next::home_dir().map(|h| h.join(".docker").join("config.json"))
}

pub fn sniff() -> Result<Vec<Candidate>> {
    let Some(path) = config_path() else {
        return Ok(Vec::new());
    };
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Ok(Vec::new());
    };
    let Ok(cfg): std::result::Result<DockerConfig, _> = serde_json::from_str(&text) else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for (registry, entry) in cfg.auths {
        let host = host_of(&registry);
        // Prefer the modern identityToken; fall back to base64(user:pass).
        // If both are absent (Desktop's credsStore mode), we still
        // surface the host so the wizard can prompt for it.
        let value = entry
            .identity_token
            .or_else(|| entry.auth.and_then(|a| decode_auth(&a)));
        let suggested_env = env_name_for(&host);
        out.push(Candidate {
            source: CandidateSource::Docker,
            label: host.clone(),
            suggested_env_name: suggested_env,
            suggested_value: value,
            host: Some(host),
            user: None,
            origin: path.clone(),
        });
    }
    Ok(out)
}

/// `auth` is base64-encoded `user:pass`. For our purposes the
/// password half is the credential — most registries also accept
/// the whole `user:pass` token verbatim so we can hand back either.
/// We return just the password to avoid leaking the username into
/// the suggested value preview.
fn decode_auth(b64: &str) -> Option<String> {
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;
    let bytes = STANDARD.decode(b64.trim()).ok()?;
    let text = String::from_utf8(bytes).ok()?;
    let (_, pass) = text.split_once(':')?;
    if pass.is_empty() {
        None
    } else {
        Some(pass.to_string())
    }
}

fn host_of(registry: &str) -> String {
    registry
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_end_matches('/')
        .split('/')
        .next()
        .unwrap_or(registry)
        .to_string()
}

fn env_name_for(host: &str) -> String {
    // Vibe coder mental model: `DOCKER_TOKEN` for Docker Hub, and a
    // host-suffixed name for private registries. GHCR is common
    // enough to deserve its own constant.
    match host {
        "index.docker.io" | "registry-1.docker.io" => "DOCKER_TOKEN".to_string(),
        "ghcr.io" => "GHCR_TOKEN".to_string(),
        other => format!(
            "DOCKER_TOKEN_{}",
            other.replace(|c: char| !c.is_alphanumeric(), "_").to_uppercase()
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_b64_decode_returns_password_half() {
        // base64("alex:hunter2") = "YWxleDpodW50ZXIy"
        assert_eq!(decode_auth("YWxleDpodW50ZXIy"), Some("hunter2".to_string()));
    }

    #[test]
    fn host_of_strips_scheme_and_path() {
        assert_eq!(host_of("https://index.docker.io/v1/"), "index.docker.io");
        assert_eq!(host_of("ghcr.io"), "ghcr.io");
    }

    #[test]
    fn env_name_specialcases_dockerhub_and_ghcr() {
        assert_eq!(env_name_for("index.docker.io"), "DOCKER_TOKEN");
        assert_eq!(env_name_for("ghcr.io"), "GHCR_TOKEN");
        assert_eq!(env_name_for("registry.acme.com"), "DOCKER_TOKEN_REGISTRY_ACME_COM");
    }
}
