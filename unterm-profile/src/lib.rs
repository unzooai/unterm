//! Unterm identity profile system.
//!
//! A *profile* binds together a coherent developer identity: GitHub /
//! AWS / npm / Anthropic tokens, git author name/email, SSH key routing,
//! and optional non-secret env vars. One Unterm window is bound to one
//! profile — switching identity means opening a new window, not
//! reconfiguring the current one (see design doc §1).
//!
//! Public surface:
//!
//! - [`ProfileFile`] / [`IndexFile`] — the TOML schema persisted under
//!   `~/.unterm/profiles/`.
//! - [`slugify`] — derive a stable filesystem-safe ID from a user-typed
//!   display name. Users never see the ID; it's an internal artifact.
//! - [`SecretStore`] / [`SecretKey`] — abstract backend for storing
//!   secrets in the OS-native credential vault. The TOML file only
//!   holds `keychain://unterm/<id>/<env>` references — raw token bytes
//!   never touch Unterm's config directory.
//!
//! See `project_identity_profiles_design.md` for the locked design.

pub mod config;
pub mod guard;
pub mod registry;
pub mod slugify;
pub mod sniff;
pub mod ssh;
pub mod store;
pub mod store_keyring;

pub use config::{GitIdentity, IndexFile, NpmConfig, ProfileFile};
pub use registry::ProfileRegistry;
pub use slugify::{disambiguate, slugify};
pub use store::{default_store, SecretError, SecretKey, SecretStore};

use std::path::PathBuf;

/// Root config directory for Unterm. `~/.unterm` on all platforms.
pub fn unterm_dir() -> PathBuf {
    dirs_next::home_dir().unwrap_or_default().join(".unterm")
}

/// Directory containing one TOML file per profile + `index.toml`.
pub fn profiles_dir() -> PathBuf {
    unterm_dir().join("profiles")
}

/// Path of a profile's TOML file given its ID.
pub fn profile_path(id: &str) -> PathBuf {
    profiles_dir().join(format!("{id}.toml"))
}

/// Path of the global profile index (display order + default profile).
pub fn index_path() -> PathBuf {
    profiles_dir().join("index.toml")
}
