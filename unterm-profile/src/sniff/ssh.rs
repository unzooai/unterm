//! SSH key + host sniffer.
//!
//! Two things to discover:
//!
//! 1. **Private keys** in `~/.ssh/`. We scan for filenames matching
//!    `id_*` (the conventional naming) but excluding `*.pub`, plus
//!    `*_ed25519` / `*_rsa` / `*_ecdsa` / `*_dsa` / `*_ed25519_sk`
//!    suffix patterns for users who name keys by purpose
//!    ("work_ed25519").
//! 2. **Host → IdentityFile mappings** in `~/.ssh/config`. The
//!    wizard uses these to suggest which key goes with which host
//!    when constructing a profile's `[ssh]` table.
//!
//! Unlike the other sniffers, SSH key files don't *contain* secrets
//! we'd copy into the keychain — the file itself is the secret. So
//! Candidates here have `suggested_value = None` and the wizard's
//! workflow is "include this key path in profile X's `[ssh]` map"
//! rather than "store this in keychain". The wizard treats SSH keys
//! distinctly from token-style credentials.

use anyhow::Result;
use std::path::PathBuf;

use super::{Candidate, CandidateSource};

pub fn sniff() -> Result<Vec<Candidate>> {
    let Some(ssh_dir) = dirs_next::home_dir().map(|h| h.join(".ssh")) else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    // Walk ~/.ssh and pick up private-key-looking files.
    if let Ok(entries) = std::fs::read_dir(&ssh_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = match path.file_name().and_then(|n| n.to_str()) {
                Some(n) => n.to_string(),
                None => continue,
            };
            if !looks_like_private_key(&name) {
                continue;
            }
            // Skip the matching `.pub` if it exists in the listing —
            // we explicitly want the *private* half.
            if name.ends_with(".pub") {
                continue;
            }
            out.push(Candidate {
                source: CandidateSource::Ssh,
                label: format!("{} ({})", name, path.display()),
                suggested_env_name: String::new(), // SSH keys don't materialize as env
                suggested_value: None,
                host: None,
                user: None,
                origin: path,
            });
        }
    }

    // Augment with host→IdentityFile blocks from ssh config. The
    // wizard uses these to suggest "this key is already configured
    // for github.com" so the Profile creation flow can pre-fill the
    // [ssh] map.
    let config_path = ssh_dir.join("config");
    if let Ok(text) = std::fs::read_to_string(&config_path) {
        out.extend(parse_ssh_config(&text, &config_path));
    }
    Ok(out)
}

fn looks_like_private_key(name: &str) -> bool {
    if name.ends_with(".pub") || name == "known_hosts" || name == "config" {
        return false;
    }
    name.starts_with("id_")
        || name.ends_with("_ed25519")
        || name.ends_with("_ed25519_sk")
        || name.ends_with("_rsa")
        || name.ends_with("_ecdsa")
        || name.ends_with("_dsa")
}

/// Parse `Host ... / IdentityFile ...` blocks. The format is:
///
/// ```text
/// Host github.com
///     User alex
///     IdentityFile ~/.ssh/work_ed25519
/// ```
///
/// We emit one candidate per (Host, IdentityFile) pair with the
/// host stuffed into `host` and the key path into `origin`.
fn parse_ssh_config(text: &str, path: &PathBuf) -> Vec<Candidate> {
    let mut out = Vec::new();
    let mut current_hosts: Vec<String> = Vec::new();
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.splitn(2, char::is_whitespace);
        let key = parts.next().unwrap_or("").to_lowercase();
        let val = parts.next().unwrap_or("").trim();
        match key.as_str() {
            "host" => {
                current_hosts = val.split_whitespace().map(str::to_string).collect();
            }
            "identityfile" => {
                let key_path = expand_tilde(val);
                for host in &current_hosts {
                    out.push(Candidate {
                        source: CandidateSource::Ssh,
                        label: format!("{host} → {val}"),
                        suggested_env_name: String::new(),
                        suggested_value: None,
                        host: Some(host.clone()),
                        user: None,
                        origin: PathBuf::from(&key_path),
                    });
                }
                let _ = path; // origin metadata comes from key_path; path kept for future tooltip
            }
            _ => {}
        }
    }
    out
}

fn expand_tilde(p: &str) -> String {
    if let Some(rest) = p.strip_prefix("~/") {
        if let Some(h) = dirs_next::home_dir() {
            return h.join(rest).display().to_string();
        }
    }
    p.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn private_key_detection() {
        assert!(looks_like_private_key("id_ed25519"));
        assert!(looks_like_private_key("id_rsa"));
        assert!(looks_like_private_key("work_ed25519"));
        assert!(looks_like_private_key("acme_rsa"));

        assert!(!looks_like_private_key("id_ed25519.pub"));
        assert!(!looks_like_private_key("config"));
        assert!(!looks_like_private_key("known_hosts"));
        assert!(!looks_like_private_key("authorized_keys"));
    }

    #[test]
    fn parses_host_identityfile_blocks() {
        let text = "
Host github.com
    User alex
    IdentityFile ~/.ssh/work_ed25519

Host gitlab.example
    IdentityFile ~/.ssh/personal_ed25519
";
        let cs = parse_ssh_config(text, &PathBuf::from("/dev/null/config"));
        assert_eq!(cs.len(), 2);
        assert!(cs.iter().any(|c| c.host.as_deref() == Some("github.com")));
        assert!(cs.iter().any(|c| c.host.as_deref() == Some("gitlab.example")));
    }
}
