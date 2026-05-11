//! macOS Keychain backend for [`SecretStore`].
//!
//! We store each profile secret as a generic-password Keychain item
//! with `service = "unterm"` and `account = "<profile_id>/<env_name>"`.
//! Composing both fields into one account string keeps the primary-key
//! shape symmetric across all three OS backends (macOS, Windows,
//! Linux) — each platform's native vault stores a single string per
//! entry, and we want the layout to stay simple.
//!
//! Permissions: when the Unterm app is code-signed (release builds are,
//! via Developer ID + notarytool), Keychain grants access without a
//! prompt for items it created. Unsigned debug builds and `cargo test`
//! binaries do trigger a prompt on first access; users running tests
//! locally accept once and the keychain remembers.

use anyhow::{Context, Result};
use security_framework::passwords::{
    delete_generic_password, get_generic_password, set_generic_password,
};

use crate::store::{SecretError, SecretKey, SecretStore};

/// Keychain service name shared by all Unterm profile secrets. Helpful
/// for users who want to inspect Unterm-managed entries in
/// Keychain Access: filter by Service = "unterm".
const SERVICE: &str = "unterm";

/// Compose the platform-native primary key string from a [`SecretKey`].
fn account_for(key: &SecretKey) -> String {
    format!("{}/{}", key.profile_id, key.env_name)
}

/// `errSecItemNotFound` from `<Security/SecBase.h>`. The crate exposes
/// it through `Error::code()`. We translate it into our typed
/// `SecretError::NotFound` so callers can render a friendly message
/// rather than leaking the OSStatus number.
const ERR_SEC_ITEM_NOT_FOUND: i32 = -25300;

pub struct MacosKeychainStore;

impl MacosKeychainStore {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MacosKeychainStore {
    fn default() -> Self {
        Self::new()
    }
}

impl SecretStore for MacosKeychainStore {
    fn get(&self, key: &SecretKey) -> Result<String> {
        let account = account_for(key);
        match get_generic_password(SERVICE, &account) {
            Ok(bytes) => String::from_utf8(bytes)
                .with_context(|| format!("non-UTF8 secret bytes for {SERVICE}/{account}")),
            Err(e) if e.code() == ERR_SEC_ITEM_NOT_FOUND => {
                Err(SecretError::NotFound(key.clone()).into())
            }
            Err(e) => Err(SecretError::Other(format!(
                "Keychain read failed for {SERVICE}/{account}: {e}"
            ))
            .into()),
        }
    }

    fn set(&self, key: &SecretKey, value: &str) -> Result<()> {
        let account = account_for(key);
        set_generic_password(SERVICE, &account, value.as_bytes())
            .with_context(|| format!("Keychain write failed for {SERVICE}/{account}"))?;
        Ok(())
    }

    fn delete(&self, key: &SecretKey) -> Result<()> {
        let account = account_for(key);
        match delete_generic_password(SERVICE, &account) {
            Ok(()) => Ok(()),
            // Idempotent: deleting a key that isn't there is a success.
            // Avoids "already cleaned up" callers having to distinguish.
            Err(e) if e.code() == ERR_SEC_ITEM_NOT_FOUND => Ok(()),
            Err(e) => Err(SecretError::Other(format!(
                "Keychain delete failed for {SERVICE}/{account}: {e}"
            ))
            .into()),
        }
    }

    fn list_for_profile(&self, _profile_id: &str) -> Result<Vec<SecretKey>> {
        // Enumerating Keychain entries requires SecItemCopyMatching with
        // a kSecMatchLimitAll query — possible but a stack of
        // CoreFoundation type-erased dictionaries. The two real callers
        // of this are (a) the profile-delete CLI command, which already
        // knows the expected secret keys from the profile's TOML
        // [secrets] table and uses this only to catch orphans, and (b)
        // the settings panel's "X keys stored" badge, which can also
        // count from the TOML. So we return an empty list for now; the
        // proper enumeration will land alongside the settings panel.
        Ok(Vec::new())
    }
}

/// Construct the platform default store. Selected by `default_store()`
/// in `store.rs` via cfg dispatch.
pub fn open() -> Result<Box<dyn SecretStore>> {
    Ok(Box::new(MacosKeychainStore::new()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Real-Keychain round-trip. Gated behind an env var so it does not
    /// run by default — touching the user's Keychain on every `cargo
    /// test` would (a) prompt on signed builds and (b) leave stray
    /// entries on test crash. Run explicitly with:
    ///
    /// ```sh
    /// UNTERM_TEST_KEYCHAIN=1 cargo test -p unterm-profile keychain
    /// ```
    #[test]
    fn round_trip_keychain() {
        if std::env::var("UNTERM_TEST_KEYCHAIN").is_err() {
            eprintln!(
                "skipping Keychain round-trip — set UNTERM_TEST_KEYCHAIN=1 to enable"
            );
            return;
        }

        let store = MacosKeychainStore::new();
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let key = SecretKey::new(
            format!("test-{unique}"),
            "FAKE_TOKEN".to_string(),
        );

        // Defensive cleanup in case a prior crashed run left an entry.
        let _ = store.delete(&key);

        // Set + read.
        store.set(&key, "hunter2").expect("set");
        let got = store.get(&key).expect("get");
        assert_eq!(got, "hunter2");

        // Overwrite + read.
        store.set(&key, "rotated-secret").expect("rotate");
        let got2 = store.get(&key).expect("get after rotate");
        assert_eq!(got2, "rotated-secret");

        // Delete + read → NotFound.
        store.delete(&key).expect("delete");
        let err = store.get(&key).expect_err("expected NotFound after delete");
        let downcast = err.downcast::<SecretError>().expect("typed error");
        assert!(matches!(downcast, SecretError::NotFound(_)));

        // Delete is idempotent.
        store.delete(&key).expect("delete idempotent");
    }
}
