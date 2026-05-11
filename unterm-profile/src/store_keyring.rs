//! Cross-platform [`SecretStore`] backed by the `keyring` crate.
//!
//! Backend selection is compile-time via the `apple-native` /
//! `windows-native` / `linux-native` keyring features wired up in the
//! workspace `Cargo.toml`:
//!
//! - **macOS** → Keychain (SecItem generic-password)
//! - **Windows** → Credential Manager (CredRead/CredWrite/CredDelete)
//! - **Linux** → Secret Service (D-Bus to gnome-keyring / KWallet /
//!   libsecret)
//!
//! All three backends store one secret per `(service, account)` pair.
//! We compose them as service = `"unterm"` and
//! account = `"<profile_id>/<env_name>"`, mirroring what was already
//! shipping in the (now-retired) `store_macos.rs`. Users who open
//! Keychain Access (macOS) / Credential Manager (Windows) /
//! seahorse (Linux) and filter on service = "unterm" will see every
//! Unterm-managed secret in one list.
//!
//! Errors are normalized into the typed [`SecretError`] enum so
//! callers can react meaningfully (e.g. fall back to `BackendUnavailable`
//! when running on Linux without a D-Bus session) instead of leaking
//! platform-specific OS error codes.

use anyhow::{Context, Result};
use keyring::{Entry, Error as KrError};

use crate::store::{SecretError, SecretKey, SecretStore};

/// Service name used for all Unterm profile secrets. Filtering on
/// this in the OS-native vault UI shows every entry we own.
const SERVICE: &str = "unterm";

fn account_for(key: &SecretKey) -> String {
    format!("{}/{}", key.profile_id, key.env_name)
}

fn entry_for(key: &SecretKey) -> Result<Entry> {
    let account = account_for(key);
    Entry::new(SERVICE, &account).with_context(|| {
        format!("construct keyring entry for {SERVICE}/{account}")
    })
}

/// Translate a `keyring::Error` into our typed [`SecretError`].
///
/// `NoEntry` becomes `NotFound` (callers can decide whether to prompt
/// the user to add the missing secret). `NoStorageAccess` / platform
/// failures become `BackendUnavailable` so the caller can degrade
/// gracefully (hide the chip on a headless Linux session, etc.). The
/// rest fall through to `Other` with the formatted error preserved.
fn translate_err(key: &SecretKey, e: KrError) -> SecretError {
    match e {
        KrError::NoEntry => SecretError::NotFound(key.clone()),
        KrError::PlatformFailure(inner) => {
            SecretError::BackendUnavailable(format!("platform failure: {inner}"))
        }
        KrError::NoStorageAccess(inner) => {
            SecretError::BackendUnavailable(format!("no vault access: {inner}"))
        }
        other => SecretError::Other(format!("keyring: {other}")),
    }
}

pub struct KeyringStore;

impl KeyringStore {
    pub fn new() -> Self {
        Self
    }
}

impl Default for KeyringStore {
    fn default() -> Self {
        Self::new()
    }
}

impl SecretStore for KeyringStore {
    fn get(&self, key: &SecretKey) -> Result<String> {
        let entry = entry_for(key)?;
        match entry.get_password() {
            Ok(s) => Ok(s),
            Err(e) => Err(translate_err(key, e).into()),
        }
    }

    fn set(&self, key: &SecretKey, value: &str) -> Result<()> {
        let entry = entry_for(key)?;
        entry
            .set_password(value)
            .map_err(|e| translate_err(key, e).into())
    }

    fn delete(&self, key: &SecretKey) -> Result<()> {
        let entry = entry_for(key)?;
        match entry.delete_password() {
            Ok(()) => Ok(()),
            // Deleting an absent entry is treated as success — callers
            // (profile-delete CLI, cleanup-on-error paths) can call this
            // idempotently without precondition checks.
            Err(KrError::NoEntry) => Ok(()),
            Err(e) => Err(translate_err(key, e).into()),
        }
    }

    fn list_for_profile(&self, _profile_id: &str) -> Result<Vec<SecretKey>> {
        // The keyring crate doesn't expose enumeration — each platform
        // has different semantics for "list all entries by service".
        // The two real callers of this both have alternatives:
        //   - profile-delete CLI iterates [secrets] from the TOML
        //   - settings panel "X keys" counts from the TOML
        // Returning an empty Vec here means "no orphans found"; if a
        // hand-edit ever leaves stray Keychain entries, the user can
        // clear them from the OS vault UI directly.
        Ok(Vec::new())
    }
}

pub fn open() -> Result<Box<dyn SecretStore>> {
    Ok(Box::new(KeyringStore::new()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Real-vault round-trip. Gated behind `UNTERM_TEST_KEYCHAIN=1` so
    /// it doesn't touch the user's keychain on every `cargo test`. On
    /// signed builds Keychain Access prompts the first time the test
    /// binary asks for an item it created; the user clicks "Always
    /// Allow" once and subsequent runs are quiet.
    ///
    /// ```sh
    /// UNTERM_TEST_KEYCHAIN=1 cargo test -p unterm-profile keychain
    /// ```
    #[test]
    fn round_trip_real_vault() {
        if std::env::var("UNTERM_TEST_KEYCHAIN").is_err() {
            eprintln!(
                "skipping real-vault round-trip — set UNTERM_TEST_KEYCHAIN=1 to enable"
            );
            return;
        }

        let store = KeyringStore::new();
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let key = SecretKey::new(format!("test-{unique}"), "FAKE_TOKEN");

        // Defensive cleanup in case a previous crashed run left an entry.
        let _ = store.delete(&key);

        store.set(&key, "hunter2").expect("set");
        assert_eq!(store.get(&key).expect("get"), "hunter2");

        store.set(&key, "rotated").expect("rotate");
        assert_eq!(store.get(&key).expect("get after rotate"), "rotated");

        store.delete(&key).expect("delete");
        let err = store
            .get(&key)
            .expect_err("expected NotFound after delete");
        let err = err.downcast::<SecretError>().expect("typed error");
        assert!(matches!(err, SecretError::NotFound(_)));

        store.delete(&key).expect("delete is idempotent");
    }
}
