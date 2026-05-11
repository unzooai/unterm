//! Abstract secret-store backend for Unterm profiles.
//!
//! Profile secrets (GitHub PATs, AWS access keys, npm tokens, OpenAI
//! API keys, …) are stored in the OS-native credential vault: macOS
//! Keychain on macOS, Credential Manager on Windows, Secret Service /
//! libsecret on Linux. Unterm's own config directory only contains
//! `keychain://unterm/<id>/<env>` reference strings — raw token bytes
//! never touch disk under `~/.unterm/`.
//!
//! See design doc §9 for the per-platform backend mapping.

use anyhow::Result;
use thiserror::Error;

/// Identifier for one secret in the vault.
///
/// `profile_id` is the slugified profile ID (`work-acme`, `工作`, …).
/// `env_name` is the env var the secret materializes as inside spawned
/// shells (`GITHUB_TOKEN`, `AWS_SECRET_ACCESS_KEY`, …).
///
/// Each platform composes these two fields into its native primary
/// key — service+account on macOS, target name on Windows, attributes
/// on Linux Secret Service — but the abstraction here is the same.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct SecretKey {
    pub profile_id: String,
    pub env_name: String,
}

impl SecretKey {
    pub fn new(profile_id: impl Into<String>, env_name: impl Into<String>) -> Self {
        Self {
            profile_id: profile_id.into(),
            env_name: env_name.into(),
        }
    }

    /// `keychain://unterm/<profile_id>/<env_name>` — the canonical
    /// reference string written into profile TOML.
    pub fn to_url(&self) -> String {
        format!("keychain://unterm/{}/{}", self.profile_id, self.env_name)
    }

    /// Inverse of [`Self::to_url`]. Returns `None` if the input doesn't
    /// match the expected shape.
    pub fn from_url(url: &str) -> Option<Self> {
        let rest = url.strip_prefix("keychain://unterm/")?;
        let (profile_id, env_name) = rest.split_once('/')?;
        if profile_id.is_empty() || env_name.is_empty() {
            return None;
        }
        Some(Self {
            profile_id: profile_id.to_string(),
            env_name: env_name.to_string(),
        })
    }
}

/// Errors common across all backends. Backends translate their native
/// error types into one of these variants so the caller can decide
/// whether to retry, prompt the user, or surface a friendly message.
#[derive(Debug, Error)]
pub enum SecretError {
    /// Key isn't present in the vault.
    #[error("secret not found in vault: {0:?}")]
    NotFound(SecretKey),

    /// The platform vault isn't available in this environment. E.g.
    /// SSH'd into a headless Linux server with no D-Bus session;
    /// macOS Keychain locked and prompt denied; Windows Credential
    /// Manager service stopped. Callers should fall back gracefully:
    /// hide the profile chip, disable secret-backed env injection, or
    /// suggest the user run in a desktop session.
    #[error("secret backend unavailable: {0}")]
    BackendUnavailable(String),

    /// Backend returned an error we don't have a specific variant for.
    /// Use this sparingly; prefer adding a typed variant when a class
    /// of errors becomes common.
    #[error("secret backend error: {0}")]
    Other(String),
}

/// Backend-agnostic secret CRUD.
///
/// Implementations live in `store_macos.rs`, `store_windows.rs`,
/// `store_linux.rs`. They are cfg-gated and surfaced through
/// [`default_store`].
pub trait SecretStore: Send + Sync {
    /// Retrieve a secret. Returns [`SecretError::NotFound`] if the key
    /// isn't present — callers should distinguish that from a vault
    /// failure to render the right error message.
    fn get(&self, key: &SecretKey) -> Result<String>;

    /// Write or overwrite a secret. Idempotent.
    fn set(&self, key: &SecretKey, value: &str) -> Result<()>;

    /// Remove a secret. No-op if it doesn't exist.
    fn delete(&self, key: &SecretKey) -> Result<()>;

    /// Enumerate keys belonging to a profile. Used by the settings
    /// panel ("6 keys stored") and by `unterm-cli profile delete <id>`
    /// to clean up the vault.
    fn list_for_profile(&self, profile_id: &str) -> Result<Vec<SecretKey>>;
}

/// Returns the platform-default secret store.
///
/// Selection is cfg-driven:
/// - macOS → [`crate::store_macos`] using `security-framework`'s
///   generic-password API.
/// - Windows → `store_windows` (lands with task #18; until then this
///   returns [`SecretError::BackendUnavailable`]).
/// - Linux → `store_linux` (lands with task #19; same fallback).
pub fn default_store() -> Result<Box<dyn SecretStore>> {
    #[cfg(target_os = "macos")]
    {
        return crate::store_macos::open();
    }

    #[cfg(not(target_os = "macos"))]
    {
        Err(SecretError::BackendUnavailable(
            "no SecretStore backend compiled in for this platform yet \
             (Windows + Linux backends land in tasks #18/#19)"
                .to_string(),
        )
        .into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_round_trip() {
        let k = SecretKey::new("work-acme", "GITHUB_TOKEN");
        let url = k.to_url();
        assert_eq!(url, "keychain://unterm/work-acme/GITHUB_TOKEN");
        let parsed = SecretKey::from_url(&url).unwrap();
        assert_eq!(parsed, k);
    }

    #[test]
    fn url_cjk_round_trip() {
        let k = SecretKey::new("工作", "ANTHROPIC_API_KEY");
        let url = k.to_url();
        assert_eq!(url, "keychain://unterm/工作/ANTHROPIC_API_KEY");
        let parsed = SecretKey::from_url(&url).unwrap();
        assert_eq!(parsed, k);
    }

    #[test]
    fn url_rejects_malformed() {
        assert!(SecretKey::from_url("keychain://other/work/x").is_none());
        assert!(SecretKey::from_url("not-a-url").is_none());
        assert!(SecretKey::from_url("keychain://unterm/").is_none());
        assert!(SecretKey::from_url("keychain://unterm/work").is_none());
        assert!(SecretKey::from_url("keychain://unterm/work/").is_none());
    }
}
