//! `.netrc` sniffer.
//!
//! `~/.netrc` is the venerable format curl / git / fetchmail use for
//! per-host login + password. Each `machine` block becomes one
//! Candidate. The format is whitespace-separated tokens (no
//! quoting) — we parse it as a stream of tokens since multiple
//! `machine` entries can sit on one line (rare but valid).
//!
//! Vibe coders' netrc typically looks like:
//!
//! ```text
//! machine api.openai.com login token password sk-xxx
//! machine github.com login alex password ghp_xxx
//! ```

use anyhow::Result;
use std::path::PathBuf;

use super::{Candidate, CandidateSource};

fn netrc_path() -> Option<PathBuf> {
    dirs_next::home_dir().map(|h| h.join(".netrc"))
}

pub fn sniff() -> Result<Vec<Candidate>> {
    let Some(path) = netrc_path() else {
        return Ok(Vec::new());
    };
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Ok(Vec::new());
    };
    Ok(parse(&text, &path))
}

fn parse(text: &str, path: &PathBuf) -> Vec<Candidate> {
    let mut out = Vec::new();
    let tokens: Vec<&str> = text
        .lines()
        .filter(|l| !l.trim_start().starts_with('#'))
        .flat_map(|l| l.split_whitespace())
        .collect();

    let mut i = 0;
    while i < tokens.len() {
        match tokens[i] {
            "machine" if i + 1 < tokens.len() => {
                let host = tokens[i + 1].to_string();
                i += 2;
                let mut login: Option<String> = None;
                let mut password: Option<String> = None;
                // Consume tokens until we hit the next `machine` /
                // `default` / `macdef` keyword.
                while i < tokens.len() {
                    match tokens[i] {
                        "machine" | "default" | "macdef" => break,
                        "login" if i + 1 < tokens.len() => {
                            login = Some(tokens[i + 1].to_string());
                            i += 2;
                        }
                        "password" | "account" if i + 1 < tokens.len() => {
                            // Treat `password` and `account` the same —
                            // some tools use account for the secret.
                            password = Some(tokens[i + 1].to_string());
                            i += 2;
                        }
                        _ => i += 1,
                    }
                }
                if let Some(pw) = password {
                    out.push(Candidate {
                        source: CandidateSource::Netrc,
                        label: match &login {
                            Some(l) => format!("{l} on {host}"),
                            None => host.clone(),
                        },
                        suggested_env_name: env_name_for(&host),
                        suggested_value: Some(pw),
                        host: Some(host),
                        user: login,
                        origin: path.clone(),
                    });
                }
            }
            "default" => {
                // `default` block — rarely useful for our purposes
                // and easy to mis-attribute to the wrong host. Skip.
                i += 1;
                while i < tokens.len() && !matches!(tokens[i], "machine" | "macdef") {
                    i += 1;
                }
            }
            "macdef" => {
                // `macdef <name>` introduces a macro until a blank
                // line. Skip everything.
                i += 1;
                while i < tokens.len() && !matches!(tokens[i], "machine" | "default" | "macdef") {
                    i += 1;
                }
            }
            _ => i += 1,
        }
    }
    out
}

fn env_name_for(host: &str) -> String {
    // Heuristic mapping host → typical env-var name. Vibe coders'
    // `.netrc` is most often used for github.com / openai / anthropic
    // these days, so we recognize those explicitly.
    match host {
        "github.com" => "GITHUB_TOKEN".to_string(),
        "api.openai.com" => "OPENAI_API_KEY".to_string(),
        "api.anthropic.com" => "ANTHROPIC_API_KEY".to_string(),
        other => format!(
            "{}_TOKEN",
            other
                .replace(|c: char| !c.is_alphanumeric(), "_")
                .to_uppercase()
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_two_machines() {
        let text = "
machine github.com login alex password ghp_abc
machine api.openai.com login token password sk-xyz
# trailing comment
";
        let cs = parse(text, &PathBuf::from("/dev/null/.netrc"));
        assert_eq!(cs.len(), 2);
        let gh = cs
            .iter()
            .find(|c| c.host.as_deref() == Some("github.com"))
            .unwrap();
        assert_eq!(gh.user.as_deref(), Some("alex"));
        assert_eq!(gh.suggested_value.as_deref(), Some("ghp_abc"));
        assert_eq!(gh.suggested_env_name, "GITHUB_TOKEN");

        let oa = cs
            .iter()
            .find(|c| c.host.as_deref() == Some("api.openai.com"))
            .unwrap();
        assert_eq!(oa.suggested_env_name, "OPENAI_API_KEY");
    }

    #[test]
    fn skips_default_and_macdef_blocks() {
        let text = "
default login anonymous password none
machine github.com login alex password ghp_abc
macdef init
    cd /home/something
machine api.anthropic.com login user password sk-ant-xxx
";
        let cs = parse(text, &PathBuf::from("/dev/null/.netrc"));
        assert!(cs.iter().any(|c| c.host.as_deref() == Some("github.com")));
        assert!(cs
            .iter()
            .any(|c| c.host.as_deref() == Some("api.anthropic.com")));
        assert!(!cs.iter().any(|c| c.host.is_none()));
    }
}
