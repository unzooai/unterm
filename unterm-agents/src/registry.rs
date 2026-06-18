//! Runtime registry: which agents are installed, their per-profile settings,
//! and the cached envelope they were last applied against.
//!
//! Disk layout (see `paths.rs`):
//!   ~/.unterm/agents/settings/<profile_id>/<agent_id>.json
//!
//! A SettingsState file looks like:
//!   {
//!     "manifest_version": 7,
//!     "values": { "model": "claude-sonnet-4-6", "max_tokens": 8192, ... }
//!   }
//!
//! Unknown keys in `values` are preserved unchanged (forward compat); the
//! UI just won't show editors for them.

use crate::errors::{AgentError, Result};
use crate::manifest::{AgentManifest, SettingSpec};
use crate::paths;
use crate::storage::{apply_settings_to_file, read_settings_from_file};
use crate::template::{expand, TemplateCtx};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SettingsState {
    #[serde(default)]
    pub manifest_version: u32,
    #[serde(default)]
    pub values: BTreeMap<String, Value>,
}

impl SettingsState {
    pub fn load(profile_id: &str, agent_id: &str) -> Result<Self> {
        let path = paths::agent_settings_path(profile_id, agent_id)?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let bytes = std::fs::read(&path)?;
        Ok(serde_json::from_slice(&bytes)?)
    }

    pub fn save(&self, profile_id: &str, agent_id: &str) -> Result<()> {
        paths::ensure_dirs()?;
        let dir = paths::profile_settings_dir(profile_id)?;
        std::fs::create_dir_all(&dir)?;
        let path = paths::agent_settings_path(profile_id, agent_id)?;
        let bytes = serde_json::to_vec_pretty(self)?;
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, &bytes)?;
        std::fs::rename(&tmp, &path)?;
        Ok(())
    }

    pub fn merge_defaults(&mut self, schema: &[SettingSpec]) {
        for s in schema {
            if !self.values.contains_key(&s.key) {
                if let Some(d) = &s.default {
                    self.values.insert(s.key.clone(), d.clone());
                }
            }
        }
    }

    /// The user's selected launch flags, stored under the synthetic
    /// `_launch_flags` key (a `_`-prefixed value persists in this JSON state
    /// only — never written to the agent's own config). Shape:
    /// `{ "<flag_id>": true }` for toggles, `{ "<flag_id>": "value" }` for
    /// value/choice flags. Empty when the user hasn't picked any.
    pub fn launch_flags(&self) -> BTreeMap<String, Value> {
        self.values
            .get(LAUNCH_FLAGS_KEY)
            .and_then(|v| v.as_object())
            .map(|o| o.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
            .unwrap_or_default()
    }
}

/// Synthetic settings key holding the user's per-agent launch-flag selection.
pub const LAUNCH_FLAGS_KEY: &str = "_launch_flags";

/// Apply a set of pending updates onto disk: per-file write via storage
/// adapters + per-key keychain writes for `Secret`-typed updates.
///
/// `updates` may include keys not in the schema; the schema is consulted to
/// determine which entries land in keychain vs files vs nowhere.
pub fn apply_updates(
    manifest: &AgentManifest,
    profile_id: &str,
    state: &mut SettingsState,
    updates: BTreeMap<String, Value>,
) -> Result<ApplyOutcome> {
    let mut written_files = Vec::new();
    let mut written_secrets = Vec::new();
    let mut skipped_unknown = Vec::new();

    // Step 1: validate every update against schema. Underscore-prefixed
    // keys are synthetic Unterm-managed settings (e.g. `_auth_mode`) that
    // live in the per-profile JSON state but never hit the agent's own
    // config file — pass them through without schema validation.
    for (key, value) in &updates {
        if key.starts_with('_') {
            continue;
        }
        let spec = manifest.settings_schema.iter().find(|s| s.key == *key);
        match spec {
            None => skipped_unknown.push(key.clone()),
            Some(s) => validate_value(s, value)?,
        }
    }

    // Step 2: split into (file-keys) and (secret-keys). Synthetic
    // `_*` keys (e.g. `_auth_mode`) are persisted only into the
    // per-profile JSON state; they never reach a storage adapter or
    // the OS keychain.
    let mut file_updates: BTreeMap<String, Value> = BTreeMap::new();
    let mut secret_updates: BTreeMap<String, String> = BTreeMap::new();
    for (key, value) in updates.iter() {
        if key.starts_with('_') {
            continue;
        }
        let Some(spec) = manifest.settings_schema.iter().find(|s| s.key == *key) else {
            continue;
        };
        if matches!(spec.kind, crate::manifest::SettingKind::Secret) {
            let env_var = spec
                .secret_namespace
                .as_deref()
                .map(crate::secrets::env_var_for_namespace)
                .unwrap_or_else(|| key.to_uppercase());
            secret_updates.insert(env_var, value.as_str().unwrap_or("").to_string());
        } else {
            file_updates.insert(key.clone(), value.clone());
        }
    }

    // Step 3: write secrets to keychain.
    if !secret_updates.is_empty() {
        let store = crate::secrets::AgentSecretStore::open()?;
        for (env_var, val) in &secret_updates {
            store.set(profile_id, env_var, val)?;
            written_secrets.push(env_var.clone());
        }
    }

    // Step 4: route file_updates across storage.files, writing each.
    if let Some(storage) = &manifest.settings_storage {
        let ctx = TemplateCtx {
            profile_id: profile_id.into(),
            agent_id: manifest.id.clone(),
            cwd: None,
            ..Default::default()
        };
        for file_spec in &storage.files {
            // Build a view of just the updates that touch this file's key_map.
            let mut updates_for_file: BTreeMap<String, Value> = BTreeMap::new();
            for (k, v) in &file_updates {
                if file_spec.key_map.contains_key(k)
                    || file_spec.single_key.as_deref() == Some(k.as_str())
                {
                    updates_for_file.insert(k.clone(), v.clone());
                }
            }
            if updates_for_file.is_empty() {
                continue;
            }
            let resolved_path = expand(&file_spec.path, &ctx)?;
            let path = std::path::Path::new(&resolved_path).to_path_buf();
            apply_settings_to_file(file_spec, &path, &updates_for_file)?;
            written_files.push(resolved_path);
        }

        // Run validate_after_write if present.
        if let Some(v) = &storage.validate_after_write {
            if let Some((bin, args)) = v.cmd.split_first() {
                let out = std::process::Command::new(bin).args(args).output();
                match out {
                    Ok(o) if o.status.code() == Some(v.expect_exit) => {}
                    Ok(o) => {
                        return Err(AgentError::InvalidSettingValue {
                            key: "<validate_after_write>".into(),
                            reason: format!(
                                "expected exit {} from {:?}, got {:?}",
                                v.expect_exit,
                                v.cmd,
                                o.status.code()
                            ),
                        });
                    }
                    Err(e) => {
                        return Err(AgentError::InvalidSettingValue {
                            key: "<validate_after_write>".into(),
                            reason: format!("could not run validate cmd: {e}"),
                        })
                    }
                }
            }
        }
    }

    // Step 5: persist Unterm-side state file.
    state.manifest_version = manifest.version;
    for (k, v) in updates {
        state.values.insert(k, v);
    }
    state.save(profile_id, &manifest.id)?;

    Ok(ApplyOutcome {
        written_files,
        written_secrets,
        skipped_unknown,
    })
}

/// Inverse of [`apply_updates`]: scan every file in `settings_storage.files`
/// + every Secret-typed key in the schema, and report what's actually
/// present on disk / in the keychain. Used by the import-existing flow.
pub fn snapshot_existing(
    manifest: &AgentManifest,
    profile_id: &str,
) -> Result<BTreeMap<String, Value>> {
    let mut out: BTreeMap<String, Value> = BTreeMap::new();
    let ctx = TemplateCtx {
        profile_id: profile_id.into(),
        agent_id: manifest.id.clone(),
        cwd: None,
        ..Default::default()
    };
    if let Some(storage) = &manifest.settings_storage {
        for file_spec in &storage.files {
            let resolved_path = expand(&file_spec.path, &ctx)?;
            let path = std::path::Path::new(&resolved_path).to_path_buf();
            let imported = read_settings_from_file(file_spec, &path)?;
            for (k, v) in imported {
                out.insert(k, v);
            }
        }
    }

    // Secrets: we don't expose values, just presence.
    if let Ok(store) = crate::secrets::AgentSecretStore::open() {
        for s in &manifest.settings_schema {
            if matches!(s.kind, crate::manifest::SettingKind::Secret) {
                let env_var = s
                    .secret_namespace
                    .as_deref()
                    .map(crate::secrets::env_var_for_namespace)
                    .unwrap_or_else(|| s.key.to_uppercase());
                if matches!(store.get(profile_id, &env_var), Ok(Some(_))) {
                    out.insert(s.key.clone(), Value::Bool(true)); // sentinel
                }
            }
        }
    }
    Ok(out)
}

#[derive(Debug, Clone, Default)]
pub struct ApplyOutcome {
    pub written_files: Vec<String>,
    pub written_secrets: Vec<String>,
    pub skipped_unknown: Vec<String>,
}

fn validate_value(spec: &SettingSpec, value: &Value) -> Result<()> {
    use crate::manifest::SettingKind::*;
    match (&spec.kind, value) {
        (Bool, Value::Bool(_)) => Ok(()),
        (Int, Value::Number(n)) => {
            let v = n.as_i64().ok_or_else(|| AgentError::InvalidSettingValue {
                key: spec.key.clone(),
                reason: "expected integer".into(),
            })? as f64;
            check_range(spec, v)
        }
        (Float, Value::Number(n)) => {
            let v = n.as_f64().ok_or_else(|| AgentError::InvalidSettingValue {
                key: spec.key.clone(),
                reason: "expected float".into(),
            })?;
            check_range(spec, v)
        }
        (String | TextLong | Path | Secret, Value::String(_)) => Ok(()),
        (Enum, v) => {
            if spec.values.iter().any(|e| &e.value == v) {
                Ok(())
            } else {
                Err(AgentError::InvalidSettingValue {
                    key: spec.key.clone(),
                    reason: format!("not in allowed enum values"),
                })
            }
        }
        (MultiEnum, Value::Array(items)) => {
            for item in items {
                if !spec.values.iter().any(|e| &e.value == item) {
                    return Err(AgentError::InvalidSettingValue {
                        key: spec.key.clone(),
                        reason: format!("array contains non-enum value"),
                    });
                }
            }
            Ok(())
        }
        (KeyValueList, Value::Object(_)) => Ok(()),
        _ => Err(AgentError::InvalidSettingValue {
            key: spec.key.clone(),
            reason: format!("value type does not match schema kind {:?}", spec.kind),
        }),
    }
}

fn check_range(spec: &SettingSpec, v: f64) -> Result<()> {
    if let Some(min) = spec.min {
        if v < min {
            return Err(AgentError::InvalidSettingValue {
                key: spec.key.clone(),
                reason: format!("value {v} < min {min}"),
            });
        }
    }
    if let Some(max) = spec.max {
        if v > max {
            return Err(AgentError::InvalidSettingValue {
                key: spec.key.clone(),
                reason: format!("value {v} > max {max}"),
            });
        }
    }
    Ok(())
}

/// Returns whether the storage write should *invalidate* settings (require
/// the agent process to be restarted to pick up changes). Used by the UI
/// to surface the "restart agent" prompt.
pub fn requires_restart(manifest: &AgentManifest, changed_keys: &[String]) -> bool {
    if let Some(storage) = &manifest.settings_storage {
        if !storage.live_reload {
            return true;
        }
    }
    manifest
        .settings_schema
        .iter()
        .any(|s| s.restart_required && changed_keys.iter().any(|k| k == &s.key))
}
