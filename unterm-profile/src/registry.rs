//! `ProfileRegistry` — in-memory view of `~/.unterm/profiles/`.
//!
//! Loads every `<id>.toml` from the profiles directory plus the
//! `index.toml` ordering file, exposes lookup by ID *or* display name
//! (with case-insensitive unique-prefix fallback), and persists writes
//! atomically. Used by:
//!
//! - **wezterm-gui** at spawn time (read profile, resolve env vars,
//!   inject into the shell)
//! - **unterm-cli** for `profile list/create/spawn/...` subcommands
//! - **MCP `profile.*`** handlers
//!
//! Concurrency: callers re-load the registry rather than sharing a
//! mutable instance. Cross-process races (CLI writes while GUI reads)
//! are handled at the file level — TOML writes are tempfile+rename,
//! so a concurrent reader either sees the old file or the new one,
//! never a half-written mid-rename state.

use anyhow::{Context, Result};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use crate::config::{IndexFile, ProfileFile};
use crate::slugify::{disambiguate, slugify};
use crate::store::{SecretKey, SecretStore};
use crate::{index_path, profile_path, profiles_dir};

pub struct ProfileRegistry {
    /// Profile ID → loaded TOML. The ID is the file stem (`work-acme`
    /// from `work-acme.toml`) or the explicit `id` field if the user
    /// overrode it in the TOML.
    profiles: HashMap<String, ProfileFile>,
    index: IndexFile,
}

impl ProfileRegistry {
    /// Read every `*.toml` in `~/.unterm/profiles/` plus `index.toml`.
    /// Missing directory is treated as "no profiles yet" — first run.
    /// Unreadable or unparseable files are logged + skipped, so one
    /// corrupted file doesn't sink the whole load.
    pub fn load() -> Result<Self> {
        let dir = profiles_dir();
        let mut profiles = HashMap::new();
        let mut index = IndexFile::default();

        let read = match std::fs::read_dir(&dir) {
            Ok(r) => r,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self { profiles, index });
            }
            Err(e) => {
                return Err(e).with_context(|| format!("read profiles dir {}", dir.display()))
            }
        };

        for entry in read.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("toml") {
                continue;
            }
            let Some(stem) = path
                .file_stem()
                .and_then(|s| s.to_str())
                .map(str::to_string)
            else {
                continue;
            };
            if stem == "index" {
                match IndexFile::load(&path) {
                    Ok(idx) => index = idx,
                    Err(e) => log::warn!("skip unreadable index file {}: {e:#}", path.display()),
                }
                continue;
            }
            match ProfileFile::load(&path) {
                Ok(p) => {
                    // Honor an explicit `id` override; otherwise the
                    // filename stem is authoritative.
                    let id = p.id.clone().unwrap_or_else(|| stem.clone());
                    profiles.insert(id, p);
                }
                Err(e) => log::warn!("skip unreadable profile {}: {e:#}", path.display()),
            }
        }

        Ok(Self { profiles, index })
    }

    /// Empty registry — useful for tests that don't want to touch the
    /// real `~/.unterm/profiles/` directory.
    pub fn empty() -> Self {
        Self {
            profiles: HashMap::new(),
            index: IndexFile::default(),
        }
    }

    // ---- test-only helpers ----
    //
    // The fields of `ProfileRegistry` are private so the public API
    // stays tight, but submodule tests (e.g. `ssh::tests`) need to
    // build a registry from synthetic profiles without writing them
    // to disk. These helpers are exposed for that purpose. The
    // `#[doc(hidden)]` keeps them out of generated documentation;
    // production code paths should go through `load` / `create` etc.

    #[doc(hidden)]
    pub fn empty_for_tests() -> Self {
        Self::empty()
    }

    #[doc(hidden)]
    pub fn insert_for_tests(&mut self, id: String, profile: ProfileFile) {
        self.profiles.insert(id, profile);
    }

    #[doc(hidden)]
    pub fn set_order_for_tests(&mut self, order: Vec<String>) {
        self.index.order = order;
    }

    /// Return profiles in chip-display order: `index.order` first, then
    /// any profile not mentioned in the index appended alphabetically.
    /// Items removed from disk but still in `index.order` are dropped.
    pub fn iter_ordered(&self) -> Vec<(&str, &ProfileFile)> {
        let mut seen = HashSet::<&str>::new();
        let mut out: Vec<(&str, &ProfileFile)> = Vec::new();
        for id in &self.index.order {
            if let Some(p) = self.profiles.get(id) {
                seen.insert(id.as_str());
                out.push((id.as_str(), p));
            }
        }
        let mut leftover: Vec<_> = self
            .profiles
            .iter()
            .filter(|(id, _)| !seen.contains(id.as_str()))
            .map(|(id, p)| (id.as_str(), p))
            .collect();
        leftover.sort_by_key(|(id, _)| *id);
        out.extend(leftover);
        out
    }

    /// Lookup by ID-or-display-name. Match order:
    ///   1. exact ID
    ///   2. exact display_name
    ///   3. unique case-insensitive prefix on display_name OR ID
    /// Returns `None` if the prefix is ambiguous or matches nothing.
    pub fn resolve(&self, name_or_id: &str) -> Option<(&str, &ProfileFile)> {
        // 1. Exact ID. Use `get_key_value` to return a reference owned
        // by the registry (lifetime of `&self`) rather than the input
        // parameter — the returned `(&str, &ProfileFile)` tuple needs
        // both halves to live as long as the registry, not as long as
        // the caller's string slice.
        if let Some((id, p)) = self.profiles.get_key_value(name_or_id) {
            return Some((id.as_str(), p));
        }
        // 2. Exact display_name.
        for (id, p) in &self.profiles {
            if p.display_name == name_or_id {
                return Some((id.as_str(), p));
            }
        }
        // 3. Unique case-insensitive prefix.
        let lower = name_or_id.to_lowercase();
        let mut candidates = self.profiles.iter().filter(|(id, p)| {
            id.to_lowercase().starts_with(&lower)
                || p.display_name.to_lowercase().starts_with(&lower)
        });
        let first = candidates.next()?;
        if candidates.next().is_some() {
            return None; // ambiguous
        }
        Some((first.0.as_str(), first.1))
    }

    /// Create a new profile from a free-text display name. Generates a
    /// non-colliding ID via [`slugify`] + [`disambiguate`], persists
    /// the TOML, and appends the ID to `index.order`. Returns the new
    /// profile's ID.
    pub fn create(&mut self, display_name: &str, accent_color: Option<&str>) -> Result<String> {
        let base = slugify(display_name)
            .with_context(|| format!("display_name {display_name:?} slugifies to empty"))?;
        let taken: Vec<String> = self.profiles.keys().cloned().collect();
        let id = disambiguate(&base, &taken);
        // `..Default::default()` gives accent_color = "" because Default
        // is derived; the serde-side `default_accent_color()` function
        // only fires during deserialization. So fill in emerald-500
        // explicitly on the create path; that way every profile written
        // by us has a usable color, and only profiles that someone hand-
        // edited to remove the field would ever appear "uncolored".
        let profile = ProfileFile {
            display_name: display_name.to_string(),
            accent_color: accent_color.unwrap_or("#10b981").to_string(),
            ..Default::default()
        };
        profile.save(&profile_path(&id))?;
        self.profiles.insert(id.clone(), profile);
        if !self.index.order.iter().any(|x| x == &id) {
            self.index.order.push(id.clone());
            self.index.save(&index_path())?;
        }
        // Keep SSH routing in sync. Don't propagate a failure here:
        // the profile was created successfully, and a missing
        // config.unterm regen is a soft warning rather than a hard
        // error — the user can rerun `profile edit <id>` later or
        // restart Unterm to re-sync.
        if let Err(e) = self.sync_ssh_config() {
            log::warn!("sync_ssh_config after create({id}) failed: {e:#}");
        }
        Ok(id)
    }

    /// Persist a modified profile back to disk and update the in-memory cache.
    pub fn save_profile(&mut self, id: &str, profile: ProfileFile) -> Result<()> {
        profile.save(&profile_path(id))?;
        self.profiles.insert(id.to_string(), profile);
        if let Err(e) = self.sync_ssh_config() {
            log::warn!("sync_ssh_config after save_profile({id}) failed: {e:#}");
        }
        Ok(())
    }

    /// Store a secret value in the OS keychain and update the
    /// profile's `[secrets]` table to reference it. The TOML never
    /// receives the raw value.
    pub fn set_secret(
        &mut self,
        store: &dyn SecretStore,
        id: &str,
        env_name: &str,
        value: &str,
    ) -> Result<()> {
        let key = SecretKey::new(id, env_name);
        store.set(&key, value)?;
        let profile = self
            .profiles
            .get_mut(id)
            .with_context(|| format!("profile not found: {id}"))?;
        profile.secrets.insert(env_name.to_string(), key.to_url());
        profile.save(&profile_path(id))?;
        Ok(())
    }

    /// Build the env map for a profile: static `[env]` + resolved
    /// `[secrets]` from the keychain + `GIT_AUTHOR_*` / `GIT_COMMITTER_*`
    /// derived from `[git]` + `UNTERM_PROFILE` itself.
    ///
    /// A secret that fails to resolve (e.g. user revoked Keychain
    /// access, entry was deleted out-of-band) is *logged and skipped*
    /// rather than aborting the spawn — a shell with most of its env
    /// is more useful than no shell. The chip will surface the missing
    /// secret via `profile.audit` so the user notices.
    pub fn resolve_env(
        &self,
        store: &dyn SecretStore,
        id: &str,
    ) -> Result<HashMap<String, String>> {
        let profile = self
            .profiles
            .get(id)
            .with_context(|| format!("profile not found: {id}"))?;

        let mut env = HashMap::new();
        env.insert("UNTERM_PROFILE".to_string(), id.to_string());

        for (k, v) in &profile.env {
            env.insert(k.clone(), v.clone());
        }

        // Git identity → process env (so spawned `git` picks it up
        // without us editing `~/.gitconfig`). Both AUTHOR and COMMITTER
        // get set so commits, merges, and rebases all attribute right.
        if !profile.git.user_name.is_empty() {
            env.insert("GIT_AUTHOR_NAME".to_string(), profile.git.user_name.clone());
            env.insert(
                "GIT_COMMITTER_NAME".to_string(),
                profile.git.user_name.clone(),
            );
        }
        if !profile.git.user_email.is_empty() {
            env.insert(
                "GIT_AUTHOR_EMAIL".to_string(),
                profile.git.user_email.clone(),
            );
            env.insert(
                "GIT_COMMITTER_EMAIL".to_string(),
                profile.git.user_email.clone(),
            );
        }

        for (env_name, url) in &profile.secrets {
            let Some(key) = SecretKey::from_url(url) else {
                log::warn!("profile {id}: secret {env_name} has malformed reference {url:?}");
                continue;
            };
            match store.get(&key) {
                Ok(val) => {
                    env.insert(env_name.clone(), val);
                }
                Err(e) => {
                    log::warn!("profile {id}: secret {env_name} unresolved: {e:#}");
                }
            }
        }

        Ok(env)
    }

    pub fn get(&self, id: &str) -> Option<&ProfileFile> {
        self.profiles.get(id)
    }

    pub fn default_id(&self) -> Option<&str> {
        self.index.default.as_deref()
    }

    pub fn set_default(&mut self, id: Option<String>) -> Result<()> {
        self.index.default = id;
        self.index.save(&index_path())?;
        Ok(())
    }

    /// Regenerate `~/.unterm/ssh/config.unterm` from the current state.
    /// Cheap — a few hundred bytes of formatting + atomic write. Call
    /// after any registry mutation so SSH key routing stays in sync.
    /// Failures are non-fatal: SSH routing breaks until next regen,
    /// but profile-level operations succeed.
    pub fn sync_ssh_config(&self) -> Result<()> {
        crate::ssh::write_for_registry(self)
    }

    pub fn len(&self) -> usize {
        self.profiles.len()
    }

    pub fn is_empty(&self) -> bool {
        self.profiles.is_empty()
    }

    /// Files this registry would write to. Surfaced so tests can pick
    /// alternate paths via env override (see [`with_dir_override`]
    /// helpers in tests).
    pub fn paths(&self) -> RegistryPaths {
        RegistryPaths {
            profiles_dir: profiles_dir(),
            index: index_path(),
        }
    }
}

pub struct RegistryPaths {
    pub profiles_dir: PathBuf,
    pub index: PathBuf,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fake in-memory secret store for resolve_env unit tests.
    #[derive(Default)]
    struct MemStore {
        inner: std::sync::Mutex<HashMap<(String, String), String>>,
    }
    impl SecretStore for MemStore {
        fn get(&self, k: &SecretKey) -> Result<String> {
            self.inner
                .lock()
                .unwrap()
                .get(&(k.profile_id.clone(), k.env_name.clone()))
                .cloned()
                .ok_or_else(|| crate::store::SecretError::NotFound(k.clone()).into())
        }
        fn set(&self, k: &SecretKey, v: &str) -> Result<()> {
            self.inner
                .lock()
                .unwrap()
                .insert((k.profile_id.clone(), k.env_name.clone()), v.to_string());
            Ok(())
        }
        fn delete(&self, k: &SecretKey) -> Result<()> {
            self.inner
                .lock()
                .unwrap()
                .remove(&(k.profile_id.clone(), k.env_name.clone()));
            Ok(())
        }
        fn list_for_profile(&self, _id: &str) -> Result<Vec<SecretKey>> {
            Ok(Vec::new())
        }
    }

    fn make_registry() -> ProfileRegistry {
        let mut r = ProfileRegistry::empty();
        let mut work = ProfileFile {
            display_name: "Work — Acme".to_string(),
            ..Default::default()
        };
        work.git.user_name = "Alex Lee".to_string();
        work.git.user_email = "alex@acme.example".to_string();
        work.env
            .insert("NODE_ENV".to_string(), "production".to_string());
        work.secrets.insert(
            "GITHUB_TOKEN".to_string(),
            "keychain://unterm/work-acme/GITHUB_TOKEN".to_string(),
        );
        r.profiles.insert("work-acme".to_string(), work);

        r.profiles.insert(
            "personal".to_string(),
            ProfileFile {
                display_name: "Personal".to_string(),
                ..Default::default()
            },
        );
        r.profiles.insert(
            "side-project".to_string(),
            ProfileFile {
                display_name: "Side Project".to_string(),
                ..Default::default()
            },
        );
        r.index.order = vec!["personal".to_string(), "work-acme".to_string()];
        r
    }

    #[test]
    fn resolve_by_exact_id() {
        let r = make_registry();
        assert_eq!(r.resolve("work-acme").map(|(id, _)| id), Some("work-acme"));
    }

    #[test]
    fn resolve_by_exact_display_name() {
        let r = make_registry();
        assert_eq!(
            r.resolve("Work — Acme").map(|(id, _)| id),
            Some("work-acme")
        );
    }

    #[test]
    fn resolve_by_unique_prefix() {
        let r = make_registry();
        // "work" is a unique prefix of "work-acme"
        assert_eq!(r.resolve("work").map(|(id, _)| id), Some("work-acme"));
        // "personal" matches itself uniquely
        assert_eq!(r.resolve("Pers").map(|(id, _)| id), Some("personal"));
    }

    #[test]
    fn resolve_ambiguous_returns_none() {
        // "s" is a prefix of "Side Project" (display) AND "side-project" (id)
        // — but those resolve to the same profile, so it's NOT ambiguous.
        // "p" is a prefix of both "personal" and "Personal" (same), so OK.
        // To force ambiguity we add another profile.
        let mut r = make_registry();
        r.profiles.insert(
            "work-2".to_string(),
            ProfileFile {
                display_name: "Work 2".to_string(),
                ..Default::default()
            },
        );
        // "wo" now prefixes both "work-acme" and "work-2"
        assert!(r.resolve("wo").is_none());
    }

    #[test]
    fn iter_ordered_follows_index_then_alphabetical() {
        let r = make_registry();
        let ordered: Vec<&str> = r.iter_ordered().into_iter().map(|(id, _)| id).collect();
        // index says personal, work-acme; leftover side-project goes after, alphabetically
        assert_eq!(ordered, vec!["personal", "work-acme", "side-project"]);
    }

    #[test]
    fn resolve_env_assembles_static_plus_git_plus_secrets() {
        let r = make_registry();
        let store = MemStore::default();
        store
            .set(
                &SecretKey::new("work-acme", "GITHUB_TOKEN"),
                "ghp_super_secret_token",
            )
            .unwrap();

        let env = r.resolve_env(&store, "work-acme").unwrap();
        assert_eq!(env.get("UNTERM_PROFILE").unwrap(), "work-acme");
        assert_eq!(env.get("NODE_ENV").unwrap(), "production");
        assert_eq!(env.get("GIT_AUTHOR_NAME").unwrap(), "Alex Lee");
        assert_eq!(env.get("GIT_AUTHOR_EMAIL").unwrap(), "alex@acme.example");
        assert_eq!(env.get("GIT_COMMITTER_NAME").unwrap(), "Alex Lee");
        assert_eq!(env.get("GIT_COMMITTER_EMAIL").unwrap(), "alex@acme.example");
        assert_eq!(env.get("GITHUB_TOKEN").unwrap(), "ghp_super_secret_token");
    }

    #[test]
    fn resolve_env_skips_unresolvable_secret_without_aborting() {
        let r = make_registry();
        let store = MemStore::default();
        // Don't set the GITHUB_TOKEN — the spawn should still get
        // UNTERM_PROFILE, NODE_ENV, GIT_* etc.
        let env = r.resolve_env(&store, "work-acme").unwrap();
        assert_eq!(env.get("UNTERM_PROFILE").unwrap(), "work-acme");
        assert!(env.get("GITHUB_TOKEN").is_none());
        assert_eq!(env.get("NODE_ENV").unwrap(), "production");
    }
}
