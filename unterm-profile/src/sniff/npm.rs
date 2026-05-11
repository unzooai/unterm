//! npm registry token sniffer.
//!
//! Reads `~/.npmrc` (user-level only — project-level `.npmrc` files
//! belong to a specific repo and aren't useful to import into a
//! global profile). Two patterns matter:
//!
//! - `//registry.example.com/:_authToken=<token>` → registry-scoped
//!   auth token. Each becomes a candidate.
//! - `@scope:registry=https://registry.example.com/` → scope→registry
//!   mapping. Doesn't carry credentials itself; the registry's token
//!   is in a separate `//.../:_authToken=` line. We attach the scope
//!   to the resulting candidate's label so the user can tell apart
//!   "npmjs.org" from "@acme private registry".

use anyhow::Result;
use std::collections::HashMap;
use std::path::PathBuf;

use super::{Candidate, CandidateSource};

fn npmrc_path() -> Option<PathBuf> {
    dirs_next::home_dir().map(|h| h.join(".npmrc"))
}

pub fn sniff() -> Result<Vec<Candidate>> {
    let Some(path) = npmrc_path() else {
        return Ok(Vec::new());
    };
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Ok(Vec::new());
    };
    Ok(parse(&text, &path))
}

fn parse(text: &str, path: &PathBuf) -> Vec<Candidate> {
    // First pass: collect scope → registry mappings (used for labels).
    let mut scope_for_registry: HashMap<String, String> = HashMap::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
            continue;
        }
        if let Some(rest) = line.strip_prefix('@') {
            if let Some((scope, val)) = rest.split_once(':') {
                if let Some((k, registry)) = val.split_once('=') {
                    if k.trim() == "registry" {
                        // Normalize registry to host form for matching.
                        if let Some(host) = host_of(registry.trim()) {
                            scope_for_registry.insert(host, format!("@{}", scope.trim()));
                        }
                    }
                }
            }
        }
    }

    // Second pass: emit token candidates.
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("//") {
            // Shape: //<host>/:_authToken=<value>
            // (also: _password / username — npm uses several auth shapes)
            if let Some((host_with_path, kv)) = rest.split_once(':') {
                if let Some((k, v)) = kv.split_once('=') {
                    if k.trim() == "_authToken" || k.trim() == "_auth" {
                        let host = host_with_path
                            .trim_end_matches('/')
                            .split('/')
                            .next()
                            .unwrap_or(host_with_path)
                            .to_string();
                        let scope = scope_for_registry.get(&host);
                        let label = match scope {
                            Some(s) => format!("{s} ({host})"),
                            None => host.clone(),
                        };
                        out.push(Candidate {
                            source: CandidateSource::Npm,
                            label,
                            suggested_env_name: "NPM_TOKEN".to_string(),
                            suggested_value: Some(v.trim().to_string()),
                            host: Some(host),
                            user: None,
                            origin: path.clone(),
                        });
                    }
                }
            }
        }
    }
    out
}

fn host_of(url: &str) -> Option<String> {
    // Cheap parse: strip scheme then take first segment.
    let no_scheme = url
        .trim_start_matches("https://")
        .trim_start_matches("http://");
    no_scheme
        .split('/')
        .next()
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_token_with_scope_label() {
        let text = "
@acme:registry=https://npm.pkg.github.com/
//npm.pkg.github.com/:_authToken=ghp_workToken
//registry.npmjs.org/:_authToken=npm_personalToken
";
        let path = PathBuf::from("/dev/null/.npmrc");
        let cs = parse(text, &path);
        assert_eq!(cs.len(), 2);

        let acme = cs.iter().find(|c| c.label.contains("@acme")).unwrap();
        assert_eq!(acme.suggested_value.as_deref(), Some("ghp_workToken"));
        assert_eq!(acme.host.as_deref(), Some("npm.pkg.github.com"));

        let npmjs = cs.iter().find(|c| c.label == "registry.npmjs.org").unwrap();
        assert_eq!(npmjs.suggested_value.as_deref(), Some("npm_personalToken"));
    }

    #[test]
    fn ignores_commented_and_blank_lines() {
        let text = "
; commented out
# also commented
//registry.foo.com/:_authToken=tok1
";
        let path = PathBuf::from("/dev/null/.npmrc");
        let cs = parse(text, &path);
        assert_eq!(cs.len(), 1);
    }
}
