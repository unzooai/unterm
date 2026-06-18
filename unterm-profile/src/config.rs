//! TOML schema for an Unterm identity profile.
//!
//! Layout (locked 2026-05-11, see design doc §2-§3):
//!
//! ```text
//! ~/.unterm/profiles/
//!     index.toml          ← display order + default profile
//!     personal.toml       ← one per profile
//!     work-acme.toml
//! ```
//!
//! Profile files contain only **references** to secrets (e.g.
//! `keychain://unterm/work-acme/github-pat`). Raw token bytes live
//! in the OS-native vault and never touch this config directory — so
//! the user can sync `~/.unterm/profiles/` into a dotfile repo without
//! leaking credentials.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// One profile, as serialized into a single TOML file.
///
/// Field ordering matters: serde+toml writes fields in declaration
/// order, and we want the user-facing fields (`display_name`,
/// `accent_color`, `description`) at the top of the file so a power
/// user opening it in `$EDITOR` sees them first.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ProfileFile {
    /// Free-text display name — the single source of truth the user
    /// types and sees. Slugified into the file's ID at create time.
    pub display_name: String,

    /// `#RRGGBB`. Used to tint the chip, the tab strip, and the window
    /// border so the user can tell identities apart at a glance.
    #[serde(default = "default_accent_color")]
    pub accent_color: String,

    /// Optional one-line note rendered in the settings panel.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,

    /// Power-user override for the derived ID. Normally `None` — Unterm
    /// generates the ID from `display_name` at create time and never
    /// changes it. Set this only if you need the ID to match an alias
    /// in some external dotfile (uncommon).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Git identity (user.name / user.email / signing key). Injected
    /// into spawned shells via `GIT_AUTHOR_*` and `GIT_COMMITTER_*` env
    /// vars rather than rewriting `~/.gitconfig` — that way the user's
    /// global git config stays untouched.
    #[serde(default, skip_serializing_if = "GitIdentity::is_empty")]
    pub git: GitIdentity,

    /// Non-secret environment variables. Examples: `NODE_ENV=production`,
    /// `AWS_REGION=us-east-1`. Values here are persisted in plaintext;
    /// anything sensitive should go in `[secrets]` instead.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub env: BTreeMap<String, String>,

    /// Secret env vars stored as keychain references. Each value is a
    /// `keychain://unterm/<id>/<env>` URL — see `SecretKey::to_url`.
    /// Never store a raw token here.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub secrets: BTreeMap<String, String>,

    /// Map of SSH host → private key path. Tilde is expanded at resolve
    /// time. Active profile writes a corresponding `Match host X exec
    /// "..."` block to `~/.unterm/ssh/config.unterm`.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub ssh: BTreeMap<String, String>,

    /// Pin the `gh` CLI to a specific account on a given host. E.g.
    /// `{"github.com" = "alex-acme"}` to use the `alex-acme` identity
    /// rather than the default one in `~/.config/gh/hosts.yml`.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub gh_host: BTreeMap<String, String>,

    /// Optional npm registry override (for scoped private packages).
    #[serde(default, skip_serializing_if = "NpmConfig::is_empty")]
    pub npm: NpmConfig,

    /// Per-secret expiration dates as `YYYY-MM-DD`. The chip surfaces a
    /// red-dot warning when ≤ 7 days remain. GitHub fine-grained PATs
    /// expose their `Expires-At`; the wizard auto-fills entries here at
    /// import time.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub expiration: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct GitIdentity {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub user_name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub user_email: String,
    /// `keychain://...` reference for a GPG signing key. Empty = no signing.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub signing_key: String,
}

impl GitIdentity {
    pub fn is_empty(&self) -> bool {
        self.user_name.is_empty() && self.user_email.is_empty() && self.signing_key.is_empty()
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct NpmConfig {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub registry: String,
}

impl NpmConfig {
    pub fn is_empty(&self) -> bool {
        self.registry.is_empty()
    }
}

fn default_accent_color() -> String {
    // Emerald-500 — same green as Unterm's primary brand color so a
    // freshly-created profile blends in until the user picks something.
    "#10b981".to_string()
}

impl ProfileFile {
    /// Read + parse a TOML profile file.
    pub fn load(path: &Path) -> Result<Self> {
        let bytes = std::fs::read_to_string(path)
            .with_context(|| format!("read profile file {}", path.display()))?;
        let parsed: ProfileFile = toml::from_str(&bytes)
            .with_context(|| format!("parse profile file {}", path.display()))?;
        Ok(parsed)
    }

    /// Serialize and atomically write the file via tempfile + rename.
    /// Atomic rename guarantees a concurrent reader sees either the old
    /// content or the new content, never a half-written file.
    pub fn save(&self, path: &Path) -> Result<()> {
        let body = toml::to_string_pretty(self).context("serialize profile to TOML")?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create dir {}", parent.display()))?;
        }
        let tmp = path.with_extension("toml.tmp");
        std::fs::write(&tmp, body).with_context(|| format!("write temp file {}", tmp.display()))?;
        std::fs::rename(&tmp, path)
            .with_context(|| format!("rename temp into {}", path.display()))?;
        Ok(())
    }
}

/// `~/.unterm/profiles/index.toml` — display order + default profile.
///
/// We keep this separate from individual profile files so that
/// reordering profiles (or changing the default) doesn't touch the
/// per-profile TOML files (which might be committed to a dotfile repo).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct IndexFile {
    /// Profile IDs in the order they appear in the chip dropdown.
    /// IDs not listed here are appended in alphabetical order at the
    /// end, so deleting `index.toml` falls back to a sensible default.
    #[serde(default)]
    pub order: Vec<String>,

    /// ID of the default profile used when a new window opens without
    /// an explicit profile choice. `None` means "show the picker".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
}

impl IndexFile {
    pub fn load(path: &Path) -> Result<Self> {
        let bytes = std::fs::read_to_string(path)
            .with_context(|| format!("read index file {}", path.display()))?;
        let parsed: IndexFile = toml::from_str(&bytes)
            .with_context(|| format!("parse index file {}", path.display()))?;
        Ok(parsed)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let body = toml::to_string_pretty(self).context("serialize index to TOML")?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create dir {}", parent.display()))?;
        }
        let tmp = path.with_extension("toml.tmp");
        std::fs::write(&tmp, body)?;
        std::fs::rename(&tmp, path)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn round_trip_profile_with_secrets() {
        let mut tmp = tempfile_path();
        let mut p = ProfileFile {
            display_name: "Work — Acme".to_string(),
            accent_color: "#3b82f6".to_string(),
            description: "Acme Corp 工作账号".to_string(),
            id: None,
            ..Default::default()
        };
        p.secrets.insert(
            "GITHUB_TOKEN".to_string(),
            "keychain://unterm/work-acme/github-pat".to_string(),
        );
        p.git.user_name = "Alex Lee".to_string();
        p.git.user_email = "alex@acme.example".to_string();
        p.env
            .insert("NODE_ENV".to_string(), "production".to_string());
        p.expiration
            .insert("GITHUB_TOKEN".to_string(), "2026-09-15".to_string());

        tmp.set_extension("toml");
        p.save(&tmp).unwrap();
        let loaded = ProfileFile::load(&tmp).unwrap();

        assert_eq!(loaded.display_name, "Work — Acme");
        assert_eq!(loaded.accent_color, "#3b82f6");
        assert_eq!(loaded.git.user_email, "alex@acme.example");
        assert_eq!(
            loaded.secrets.get("GITHUB_TOKEN").map(String::as_str),
            Some("keychain://unterm/work-acme/github-pat")
        );
        assert_eq!(
            loaded.expiration.get("GITHUB_TOKEN").map(String::as_str),
            Some("2026-09-15")
        );

        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn missing_optional_sections_default() {
        // Profiles without `[git]` or `[secrets]` etc should still parse.
        let body = r##"
display_name = "Personal"
accent_color = "#10b981"
"##;
        let mut tmp = tempfile_path();
        tmp.set_extension("toml");
        std::fs::write(&tmp, body).unwrap();
        let loaded = ProfileFile::load(&tmp).unwrap();
        assert_eq!(loaded.display_name, "Personal");
        assert!(loaded.git.is_empty());
        assert!(loaded.secrets.is_empty());
        assert!(loaded.ssh.is_empty());
        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn index_round_trip() {
        let mut tmp = tempfile_path();
        tmp.set_extension("toml");
        let idx = IndexFile {
            order: vec!["personal".to_string(), "work-acme".to_string()],
            default: Some("personal".to_string()),
        };
        idx.save(&tmp).unwrap();
        let loaded = IndexFile::load(&tmp).unwrap();
        assert_eq!(loaded.order, vec!["personal", "work-acme"]);
        assert_eq!(loaded.default.as_deref(), Some("personal"));
        std::fs::remove_file(&tmp).ok();
    }

    fn tempfile_path() -> std::path::PathBuf {
        let dir = std::env::temp_dir();
        let pid = std::process::id();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        dir.join(format!("unterm-profile-test-{pid}-{nanos}"))
    }

    // suppress unused-import warning in non-test builds
    #[allow(dead_code)]
    fn _silence_unused_io_write(mut w: impl Write) {
        let _ = w.write_all(b"");
    }
}
