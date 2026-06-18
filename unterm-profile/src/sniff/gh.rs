//! GitHub CLI account sniffer.
//!
//! `gh` stores accounts in `~/.config/gh/hosts.yml` (Unix) or
//! `~/Library/Application Support/gh/hosts.yml` (macOS, sometimes).
//! Modern gh (≥ 2.40) stores the OAuth token in the OS keychain
//! rather than the YAML file; older versions inlined it as
//! `oauth_token: ghp_xxx`. We try both — if the file gives us a
//! plaintext token, surface it; otherwise mark the candidate as
//! "needs user input" so the wizard prompts.
//!
//! We parse YAML by hand. Pulling in `serde_yaml` here would more
//! than double our dependency footprint for one config file with a
//! shallow shape. The format is simple enough that line-by-line
//! works without false positives in practice.

use anyhow::Result;
use std::path::PathBuf;

use super::{Candidate, CandidateSource};

fn candidate_paths() -> Vec<PathBuf> {
    let home = dirs_next::home_dir();
    let mut out = Vec::new();
    if let Some(h) = &home {
        out.push(h.join(".config").join("gh").join("hosts.yml"));
        out.push(h.join("Library/Application Support/gh/hosts.yml"));
    }
    out
}

pub fn sniff() -> Result<Vec<Candidate>> {
    for path in candidate_paths() {
        if let Ok(text) = std::fs::read_to_string(&path) {
            return Ok(parse(&text, &path));
        }
    }
    Ok(Vec::new())
}

/// Parse a `gh` hosts.yml. Expected shape:
///
/// ```yaml
/// github.com:
///     user: alice
///     oauth_token: ghp_xxx       # legacy, may be absent on new gh
///     git_protocol: https
///     users:
///         alice:
///             oauth_token: ghp_xxx
/// gh.acme.example:
///     user: alice-acme
/// ```
///
/// We emit one candidate per `(host, user)`. `oauth_token` may sit
/// at the host level (single account per host) or nested under
/// `users.<name>` (multi-account, gh ≥ 2.30).
fn parse(text: &str, path: &PathBuf) -> Vec<Candidate> {
    let mut out = Vec::new();
    let mut current_host: Option<String> = None;
    let mut current_user: Option<String> = None;
    let mut pending_token: Option<String> = None;
    let mut inside_users_block = false;
    let mut nested_user: Option<String> = None;

    // Determine the indent of nested keys by inspecting the first
    // non-empty indented line. We don't strictly need exact column
    // tracking — host-level keys are at column 0 (zero indent) and
    // everything else is indented.
    let lines: Vec<&str> = text.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        let raw = lines[i];
        let stripped = raw.trim();
        if stripped.is_empty() || stripped.starts_with('#') {
            i += 1;
            continue;
        }
        let indent = raw.len() - raw.trim_start().len();

        if indent == 0 {
            // New host. Flush any pending host's data first.
            flush(
                &current_host,
                &current_user,
                pending_token.take(),
                path,
                &mut out,
            );
            current_host = stripped.trim_end_matches(':').to_string().into();
            current_user = None;
            inside_users_block = false;
            nested_user = None;
        } else if let Some((k, v)) = stripped.split_once(':') {
            let key = k.trim();
            let val = v.trim();
            match key {
                "user" if !inside_users_block => current_user = non_empty(val),
                "oauth_token" => {
                    if inside_users_block {
                        // Belongs to nested_user
                        if let Some(u) = &nested_user {
                            out.push(Candidate {
                                source: CandidateSource::Gh,
                                label: format!(
                                    "{} on {}",
                                    u,
                                    current_host.clone().unwrap_or_default()
                                ),
                                suggested_env_name: env_name_for(
                                    current_host.as_deref().unwrap_or(""),
                                ),
                                suggested_value: non_empty(val),
                                host: current_host.clone(),
                                user: Some(u.clone()),
                                origin: path.clone(),
                            });
                        }
                    } else {
                        pending_token = non_empty(val);
                    }
                }
                "users" => {
                    inside_users_block = true;
                    nested_user = None;
                }
                _ => {
                    if inside_users_block && val.is_empty() {
                        // Looks like a sub-section header like
                        //   users:
                        //     alice:
                        //       oauth_token: ...
                        nested_user = Some(key.to_string());
                    }
                }
            }
        }
        i += 1;
    }
    flush(&current_host, &current_user, pending_token, path, &mut out);
    out
}

fn flush(
    host: &Option<String>,
    user: &Option<String>,
    token: Option<String>,
    path: &PathBuf,
    out: &mut Vec<Candidate>,
) {
    let Some(host) = host else {
        return;
    };
    // Only emit a host-level candidate if we have at least a user OR
    // a token. Hosts that only set `git_protocol: https` would
    // otherwise produce a useless entry.
    if user.is_none() && token.is_none() {
        return;
    }
    out.push(Candidate {
        source: CandidateSource::Gh,
        label: match user {
            Some(u) => format!("{u} on {host}"),
            None => host.clone(),
        },
        suggested_env_name: env_name_for(host),
        suggested_value: token,
        host: Some(host.clone()),
        user: user.clone(),
        origin: path.clone(),
    });
}

fn env_name_for(host: &str) -> String {
    // Vibe coders' GitHub PATs go into GITHUB_TOKEN. Self-hosted
    // Enterprise installations under github.acme.example get a
    // host-suffixed name so a profile can carry both without collision.
    if host == "github.com" {
        "GITHUB_TOKEN".to_string()
    } else {
        format!(
            "GH_TOKEN_{}",
            host.replace(|c: char| !c.is_alphanumeric(), "_")
                .to_uppercase()
        )
    }
}

fn non_empty(s: &str) -> Option<String> {
    if s.is_empty() || s == "null" {
        None
    } else {
        Some(s.trim_matches('"').to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_single_account_legacy_layout() {
        let text = "github.com:
    user: alice
    oauth_token: ghp_legacy
    git_protocol: https
";
        let cs = parse(text, &PathBuf::from("/dev/null/hosts.yml"));
        assert_eq!(cs.len(), 1);
        assert_eq!(cs[0].user.as_deref(), Some("alice"));
        assert_eq!(cs[0].host.as_deref(), Some("github.com"));
        assert_eq!(cs[0].suggested_value.as_deref(), Some("ghp_legacy"));
        assert_eq!(cs[0].suggested_env_name, "GITHUB_TOKEN");
    }

    #[test]
    fn extracts_multi_account_users_block() {
        let text = "github.com:
    user: alice
    users:
        alice:
            oauth_token: ghp_alice
        alex-acme:
            oauth_token: ghp_acme
";
        let cs = parse(text, &PathBuf::from("/dev/null/hosts.yml"));
        // Expect 2 nested-user candidates + 1 host-level (user "alice" present).
        // The host-level may or may not have a token; either way it
        // emits a candidate since user is set.
        assert!(cs.iter().any(|c| c.user.as_deref() == Some("alice")
            && c.suggested_value.as_deref() == Some("ghp_alice")));
        assert!(cs.iter().any(|c| c.user.as_deref() == Some("alex-acme")
            && c.suggested_value.as_deref() == Some("ghp_acme")));
    }

    #[test]
    fn enterprise_host_gets_suffixed_env_name() {
        let text = "gh.acme.example:
    user: alex
    oauth_token: gho_enterprise
";
        let cs = parse(text, &PathBuf::from("/dev/null/hosts.yml"));
        assert_eq!(cs.len(), 1);
        assert_eq!(cs[0].suggested_env_name, "GH_TOKEN_GH_ACME_EXAMPLE");
    }
}
