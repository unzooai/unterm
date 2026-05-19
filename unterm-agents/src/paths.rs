//! On-disk layout for AI agent runtime state.
//!
//! Everything Unterm knows about installed agents, their settings, and the
//! signed manifest cache lives under `~/.unterm/agents/`. We deliberately
//! keep it next to `~/.unterm/profiles/` so the existing backup story
//! (rsync the .unterm dir) covers agent state too.
//!
//! Layout:
//!
//! ```text
//! ~/.unterm/agents/
//!     manifests/
//!         cache.json       last verified Envelope (raw bytes)
//!         cache.etag       ETag from CF for conditional GET
//!         fallback.json    written on first launch from include_bytes!
//!     settings/
//!         <profile_id>/
//!             <agent_id>.json   settings + last manifest version applied
//!     state.json           per-host state (detect cache, last refresh)
//! ```
//!
//! Secrets never land here — they go straight to the OS keychain via
//! [`unterm_profile::SecretStore`].

use anyhow::{Context, Result};
use std::path::PathBuf;

pub fn unterm_dir() -> Result<PathBuf> {
    let home = dirs_next::home_dir().context("home dir not found")?;
    Ok(home.join(".unterm"))
}

pub fn agents_dir() -> Result<PathBuf> {
    Ok(unterm_dir()?.join("agents"))
}

pub fn manifests_dir() -> Result<PathBuf> {
    Ok(agents_dir()?.join("manifests"))
}

pub fn manifest_cache_path() -> Result<PathBuf> {
    Ok(manifests_dir()?.join("cache.json"))
}

pub fn manifest_etag_path() -> Result<PathBuf> {
    Ok(manifests_dir()?.join("cache.etag"))
}

pub fn manifest_fallback_path() -> Result<PathBuf> {
    Ok(manifests_dir()?.join("fallback.json"))
}

pub fn settings_dir() -> Result<PathBuf> {
    Ok(agents_dir()?.join("settings"))
}

pub fn profile_settings_dir(profile_id: &str) -> Result<PathBuf> {
    Ok(settings_dir()?.join(sanitize(profile_id)))
}

pub fn agent_settings_path(profile_id: &str, agent_id: &str) -> Result<PathBuf> {
    Ok(profile_settings_dir(profile_id)?.join(format!("{}.json", sanitize(agent_id))))
}

pub fn state_path() -> Result<PathBuf> {
    Ok(agents_dir()?.join("state.json"))
}

/// Each agent gets its own isolated $HOME under the profile dir; lets the
/// agent write its own `.config/<agent>/...` without colliding with the
/// user's real home.
pub fn agent_home(profile_id: &str, agent_id: &str) -> Result<PathBuf> {
    Ok(profile_settings_dir(profile_id)?.join(format!("{}-home", sanitize(agent_id))))
}

pub fn ensure_dirs() -> Result<()> {
    std::fs::create_dir_all(manifests_dir()?)?;
    std::fs::create_dir_all(settings_dir()?)?;
    Ok(())
}

/// Conservative file-name sanitization. We allow [A-Za-z0-9._-] only and
/// fold everything else to '_' so a malicious profile or agent id can't
/// path-traverse out of the agents dir. (e.g. "../etc/passwd" → "_._etc_passwd")
fn sanitize(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_keeps_safe_chars() {
        assert_eq!(sanitize("claude-code"), "claude-code");
        assert_eq!(sanitize("v1.2.3"), "v1.2.3");
    }

    #[test]
    fn sanitize_strips_path_traversal() {
        // `/` is folded to `_`; `.` is allowed but harmless without a slash.
        // The slash removal is what stops traversal — there's no parent dir
        // for `..` to climb into once the separator is gone.
        assert_eq!(sanitize("../etc/passwd"), ".._etc_passwd");
        assert_eq!(sanitize("foo/bar"), "foo_bar");
        assert_eq!(sanitize("a\\b"), "a_b");
    }
}
