//! AWS credentials sniffer.
//!
//! Reads `~/.aws/credentials` (the canonical static-keys file) and
//! `~/.aws/config` (for region defaults). Each `[profile_name]` section
//! becomes one Candidate per key (access-key-id + secret-access-key
//! emitted as two suggestions sharing the same `user` so the wizard
//! can keep them grouped).
//!
//! AWS SSO / IAM Roles Anywhere store credentials in `~/.aws/sso/cache/`
//! as short-lived JSON. We deliberately do NOT pick those up — they
//! expire every few hours and reproducing them under Unterm would
//! just race the user's `aws sso login`. Static keys only.

use anyhow::Result;
use std::path::PathBuf;

use super::{Candidate, CandidateSource};

fn credentials_path() -> Option<PathBuf> {
    dirs_next::home_dir().map(|h| h.join(".aws").join("credentials"))
}

pub fn sniff() -> Result<Vec<Candidate>> {
    let Some(path) = credentials_path() else {
        return Ok(Vec::new());
    };
    let Ok(text) = std::fs::read_to_string(&path) else {
        // Missing file is fine — most users won't have AWS set up.
        return Ok(Vec::new());
    };
    Ok(parse(&text, &path))
}

fn parse(text: &str, path: &PathBuf) -> Vec<Candidate> {
    let mut out = Vec::new();
    let mut current: Option<String> = None;
    let mut akid: Option<String> = None;
    let mut secret: Option<String> = None;
    let mut session: Option<String> = None;

    let flush = |current: &Option<String>,
                 akid: &mut Option<String>,
                 secret: &mut Option<String>,
                 session: &mut Option<String>,
                 out: &mut Vec<Candidate>| {
        let Some(name) = current else {
            return;
        };
        if let Some(v) = akid.take() {
            out.push(Candidate {
                source: CandidateSource::Aws,
                label: format!("{name} (aws_access_key_id)"),
                suggested_env_name: "AWS_ACCESS_KEY_ID".to_string(),
                suggested_value: Some(v),
                host: None,
                user: Some(name.clone()),
                origin: path.clone(),
            });
        }
        if let Some(v) = secret.take() {
            out.push(Candidate {
                source: CandidateSource::Aws,
                label: format!("{name} (aws_secret_access_key)"),
                suggested_env_name: "AWS_SECRET_ACCESS_KEY".to_string(),
                suggested_value: Some(v),
                host: None,
                user: Some(name.clone()),
                origin: path.clone(),
            });
        }
        if let Some(v) = session.take() {
            out.push(Candidate {
                source: CandidateSource::Aws,
                label: format!("{name} (aws_session_token)"),
                suggested_env_name: "AWS_SESSION_TOKEN".to_string(),
                suggested_value: Some(v),
                host: None,
                user: Some(name.clone()),
                origin: path.clone(),
            });
        }
    };

    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        if let Some(name) = line.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            flush(&current, &mut akid, &mut secret, &mut session, &mut out);
            current = Some(name.trim().to_string());
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            let k = k.trim().to_lowercase();
            let v = v.trim().to_string();
            match k.as_str() {
                "aws_access_key_id" => akid = Some(v),
                "aws_secret_access_key" => secret = Some(v),
                "aws_session_token" => session = Some(v),
                _ => {}
            }
        }
    }
    flush(&current, &mut akid, &mut secret, &mut session, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_two_profiles() {
        let text = "
[default]
aws_access_key_id = AKIA1111
aws_secret_access_key = secret1

# inline comment line
[work-acme]
aws_access_key_id = AKIA2222
aws_secret_access_key = secret2
aws_session_token = sess2
";
        let path = PathBuf::from("/dev/null/credentials");
        let cs = parse(text, &path);
        assert_eq!(cs.len(), 5); // 2 + 3
        assert!(cs.iter().any(|c| c.user.as_deref() == Some("default")
            && c.suggested_env_name == "AWS_ACCESS_KEY_ID"));
        assert!(cs.iter().any(|c| c.user.as_deref() == Some("work-acme")
            && c.suggested_env_name == "AWS_SESSION_TOKEN"
            && c.suggested_value.as_deref() == Some("sess2")));
    }

    #[test]
    fn empty_file_is_ok() {
        let path = PathBuf::from("/dev/null/credentials");
        assert!(parse("", &path).is_empty());
        assert!(parse("# only comments\n; nothing else\n", &path).is_empty());
    }
}
