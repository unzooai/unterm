//! Build the launch command + environment for spawning an agent in a pane.
//!
//! Output is a [`LaunchPlan`]: exec + argv + env. The GUI's `spawn.rs`
//! turns this into a real `portable_pty::CommandBuilder`. The CLI's
//! `unterm agent launch <id>` exec's it directly (replacing itself), so
//! Unterm doesn't sit around as a parent process.

use crate::errors::Result;
use crate::manifest::{AgentManifest, EnvBinding};
use crate::registry::SettingsState;
use crate::secrets::AgentSecretStore;
use crate::template::{expand, expand_args, TemplateCtx};
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct LaunchPlan {
    pub exec: String,
    pub args: Vec<String>,
    pub env_set: BTreeMap<String, String>,
    /// env var names to *unset* before launch (e.g. clear inherited
    /// `AWS_PROFILE` if profile isolation is on).
    pub env_unset: Vec<String>,
    pub cwd: Option<String>,
}

pub struct LaunchInputs<'a> {
    pub manifest: &'a AgentManifest,
    pub profile_id: &'a str,
    pub settings: &'a SettingsState,
    pub cwd: Option<&'a str>,
    pub project_root: Option<&'a str>,
}

pub fn build_launch_plan(inputs: &LaunchInputs<'_>) -> Result<LaunchPlan> {
    let ctx = TemplateCtx {
        profile_id: inputs.profile_id.to_string(),
        agent_id: inputs.manifest.id.clone(),
        cwd: inputs.cwd.map(|s| s.to_string()),
    };

    let exec = expand(&inputs.manifest.launch.exec, &ctx)?;
    let mut args = expand_args(&inputs.manifest.launch.args, &ctx)?;
    if inputs.cwd.is_some() && !inputs.manifest.launch.args_when_cwd_set.is_empty() {
        args.extend(expand_args(
            &inputs.manifest.launch.args_when_cwd_set,
            &ctx,
        )?);
    }

    let mut env_set: BTreeMap<String, String> = BTreeMap::new();
    env_set.insert("UNTERM_AGENT_ID".into(), inputs.manifest.id.clone());
    env_set.insert(
        "UNTERM_AGENT_MANIFEST_VERSION".into(),
        inputs.manifest.version.to_string(),
    );
    env_set.insert("UNTERM_PROFILE".into(), inputs.profile_id.into());

    if let Some(storage) = &inputs.manifest.settings_storage {
        let store = AgentSecretStore::open().ok();
        for (env_name, binding) in &storage.env_at_launch {
            let resolved = resolve_env_binding(binding, inputs.profile_id, inputs.settings, store.as_ref())?;
            if let Some(v) = resolved {
                env_set.insert(env_name.clone(), v);
            }
        }
    }

    // Profile isolation: if isolate_env, we don't list every env to unset,
    // we just communicate to the spawn layer that the *inherited* environment
    // should be filtered through `inherit_env`. The caller (GUI spawn or
    // CLI exec) does the actual filtering.
    let mut env_unset = Vec::new();
    if inputs.manifest.profile_defaults.isolate_env {
        env_unset.push("__UNTERM_ISOLATE__".into()); // sentinel
    }

    let cwd = resolve_cwd(&inputs.manifest.profile_defaults.starting_cwd, inputs);
    Ok(LaunchPlan {
        exec,
        args,
        env_set,
        env_unset,
        cwd,
    })
}

fn resolve_env_binding(
    binding: &EnvBinding,
    profile_id: &str,
    settings: &SettingsState,
    secret_store: Option<&AgentSecretStore>,
) -> Result<Option<String>> {
    match binding {
        EnvBinding::Literal { literal } => Ok(Some(literal.clone())),
        EnvBinding::Setting {
            from_setting,
            skip_if_empty,
        } => {
            let val = settings.values.get(from_setting);
            match val {
                Some(serde_json::Value::String(s)) => {
                    if s.is_empty() && *skip_if_empty {
                        Ok(None)
                    } else {
                        Ok(Some(s.clone()))
                    }
                }
                Some(other) => Ok(Some(other.to_string())),
                None => Ok(None),
            }
        }
        EnvBinding::Secret { from } => {
            let Some(ns) = from.strip_prefix("secret:") else {
                return Ok(None);
            };
            let env_var = crate::secrets::env_var_for_namespace(ns);
            let Some(store) = secret_store else {
                return Ok(None);
            };
            store.get(profile_id, &env_var)
        }
    }
}

fn resolve_cwd(setting: &Option<String>, inputs: &LaunchInputs<'_>) -> Option<String> {
    let s = setting.as_deref().unwrap_or("project_root_or_home");
    match s {
        "current" => inputs.cwd.map(|s| s.to_string()),
        "project_root" => inputs.project_root.map(|s| s.to_string()),
        "home" => dirs_next::home_dir().map(|p| p.to_string_lossy().to_string()),
        "project_root_or_home" => inputs
            .project_root
            .map(|s| s.to_string())
            .or_else(|| dirs_next::home_dir().map(|p| p.to_string_lossy().to_string())),
        _ => inputs.cwd.map(|s| s.to_string()),
    }
}
