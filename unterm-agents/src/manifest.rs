//! AgentManifest schema — the data the signed envelope carries.
//!
//! The envelope's `manifests` field is `Vec<AgentManifest>`. Each manifest
//! describes everything Unterm needs to install, authenticate, configure,
//! and launch one AI CLI agent. The schema is intentionally agent-agnostic:
//! `claude-code`, `codex-cli`, `aider`, etc. all serialize into the same
//! shape, with the differences captured in the various typed sub-records.
//!
//! Forward-compatibility: every struct uses `#[serde(default)]` /
//! `Option<>` for fields that newer manifests may carry, so an older Unterm
//! binary can still parse a newer envelope (it just ignores unknown fields
//! and misses out on new features). Removing a field is a breaking change
//! and requires bumping `envelope_version`.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Top-level signed envelope returned by `GET /api/agents/manifests`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Envelope {
    pub envelope_version: u32,
    pub issued_at: String,
    pub expires_at: String,
    pub min_unterm_version: String,
    pub manifests: Vec<AgentManifest>,
    pub signature: Signature,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signature {
    pub alg: String, // "ed25519"
    pub key_id: String,
    /// base64 (standard, padded).
    pub sig: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentManifest {
    pub id: String,
    pub version: u32,
    pub name: String,
    pub vendor: String,
    #[serde(default)]
    pub category: String, // "first-party" | "community" | "experimental"
    #[serde(default)]
    pub popularity_rank: u32,
    #[serde(default)]
    pub icon_url: Option<String>,
    #[serde(default)]
    pub homepage: Option<String>,
    #[serde(default)]
    pub tagline_i18n: BTreeMap<String, String>,
    #[serde(default)]
    pub description_i18n: BTreeMap<String, String>,
    #[serde(default)]
    pub license: Option<String>,
    pub platforms: Vec<String>, // "macos" | "linux" | "windows"

    pub detect: DetectSpec,
    pub install: InstallSpec,
    pub auth: AuthSpec,
    pub launch: LaunchSpec,

    #[serde(default)]
    pub mcp: Option<McpSpec>,
    #[serde(default)]
    pub profile_defaults: ProfileDefaults,
    #[serde(default)]
    pub settings_schema: Vec<SettingSpec>,
    #[serde(default)]
    pub settings_storage: Option<SettingsStorage>,
    #[serde(default)]
    pub telemetry_notice: Option<TelemetryNotice>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectSpec {
    pub command: String,
    #[serde(default)]
    pub version_args: Vec<String>,
    #[serde(default)]
    pub version_regex: Option<String>,
    #[serde(default)]
    pub min_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InstallSpec {
    #[serde(default)]
    pub macos: Option<PlatformInstall>,
    #[serde(default)]
    pub linux: Option<PlatformInstall>,
    #[serde(default)]
    pub windows: Option<PlatformInstall>,
    #[serde(default)]
    pub requires: Vec<BinaryRequirement>,
    #[serde(default)]
    pub update: Option<ShellCmd>,
    #[serde(default)]
    pub uninstall: Option<ShellCmd>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformInstall {
    pub steps: Vec<InstallStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InstallStep {
    Shell {
        cmd: Vec<String>,
    },
    Download {
        url: String,
        sha256: String,
        dest: String, // template, expanded with {{HOME}} etc.
        #[serde(default)]
        chmod: Option<String>, // e.g. "0755"
    },
    ScriptText {
        interpreter: String, // "bash" | "sh"
        text: String,
        sha256: String, // sha256 of `text`, double-check
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinaryRequirement {
    pub binary: String,
    #[serde(default)]
    pub min: Option<String>,
    #[serde(default)]
    pub install_hint_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellCmd {
    pub cmd: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthSpec {
    pub primary: AuthMethod,
    #[serde(default)]
    pub fallback: Option<AuthMethod>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AuthMethod {
    /// Run a command that opens the browser; poll the command's stdout for
    /// `ready_marker`; if not seen within `timeout_s`, declare failure.
    OauthBrowser {
        trigger: ShellCmd,
        #[serde(default)]
        ready_marker: Option<String>,
        #[serde(default = "default_oauth_timeout")]
        timeout_s: u64,
    },
    /// Prompt the user for an API key, validate it (optional), then store
    /// in the OS keychain under (profile, agent, env_var).
    ApiKeyEnv {
        env_var: String,
        #[serde(default)]
        key_console_url: Option<String>,
        #[serde(default)]
        validate: Option<ShellCmd>,
    },
}

fn default_oauth_timeout() -> u64 {
    180
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaunchSpec {
    pub exec: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub args_when_cwd_set: Vec<String>,
    #[serde(default)]
    pub respects_unterm_split: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpSpec {
    #[serde(default)]
    pub client_supports_mcp: bool,
    #[serde(default)]
    pub auto_register_unterm: Option<McpAutoRegister>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpAutoRegister {
    pub config_path: String, // template
    pub format: String,      // "claude_code_v1" | "codex_v1" | "opencode_v1" | "generic_mcp_json"
    pub server_name: String,
    pub transport: String, // "stdio" | "tcp" | "sse"
    #[serde(default)]
    pub scopes_default: Vec<String>,
    #[serde(default)]
    pub scopes_optional: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileDefaults {
    #[serde(default = "default_true")]
    pub isolate_env: bool,
    #[serde(default = "default_inherit_env")]
    pub inherit_env: Vec<String>,
    #[serde(default)]
    pub shell: Option<String>, // None => user_default
    #[serde(default)]
    pub history_file: Option<String>,
    #[serde(default)]
    pub starting_cwd: Option<String>,
    #[serde(default = "default_audit_on")]
    pub audit_default: String, // "on" | "off"
}

// Custom Default keeps isolate_env=true + audit_default="on" when the
// outer `manifest.profile_defaults` field is missing entirely. Derived
// Default would give isolate_env=false (bool default), which would
// silently flip the sandbox off — exactly the wrong direction to fail.
impl Default for ProfileDefaults {
    fn default() -> Self {
        Self {
            isolate_env: true,
            inherit_env: default_inherit_env(),
            shell: None,
            history_file: None,
            starting_cwd: None,
            audit_default: default_audit_on(),
        }
    }
}

fn default_true() -> bool {
    true
}
fn default_audit_on() -> String {
    "on".into()
}
fn default_inherit_env() -> Vec<String> {
    vec![
        "PATH".into(),
        "HOME".into(),
        "LANG".into(),
        "TERM".into(),
        "SHELL".into(),
    ]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingSpec {
    pub key: String,
    #[serde(rename = "type")]
    pub kind: SettingKind,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub default: Option<serde_json::Value>,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub sensitive: bool,
    #[serde(default)]
    pub label_i18n: BTreeMap<String, String>,
    #[serde(default)]
    pub description_i18n: BTreeMap<String, String>,
    #[serde(default)]
    pub placeholder: Option<String>,
    #[serde(default)]
    pub restart_required: bool,
    #[serde(default)]
    pub values: Vec<EnumValue>,
    #[serde(default)]
    pub min: Option<f64>,
    #[serde(default)]
    pub max: Option<f64>,
    #[serde(default)]
    pub secret_namespace: Option<String>,
    #[serde(default)]
    pub since_manifest_version: Option<u32>,
    #[serde(default)]
    pub depends_on: Option<SettingDependency>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SettingKind {
    Bool,
    Int,
    Float,
    String,
    TextLong,
    Enum,
    MultiEnum,
    Path,
    Secret,
    KeyValueList,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumValue {
    pub value: serde_json::Value,
    #[serde(default)]
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingDependency {
    pub key: String,
    pub equals: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsStorage {
    #[serde(default)]
    pub files: Vec<StorageFile>,
    #[serde(default)]
    pub env_at_launch: BTreeMap<String, EnvBinding>,
    #[serde(default)]
    pub live_reload: bool,
    #[serde(default)]
    pub validate_after_write: Option<ValidateCmd>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageFile {
    pub path: String,
    pub format: String, // "json" | "toml" | "yaml" | "env" | "ini" | "raw_text"
    #[serde(default = "default_merge")]
    pub merge: String, // "preserve_unknown_keys" | "overwrite"
    #[serde(default = "default_true")]
    pub atomic_write: bool,
    #[serde(default)]
    pub key_map: BTreeMap<String, String>, // setting_key → jq-path-in-file
    #[serde(default)]
    pub single_key: Option<String>, // for raw_text format
}

fn default_merge() -> String {
    "preserve_unknown_keys".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum EnvBinding {
    /// `{ "from": "secret:<namespace>" }` reads from the OS keychain.
    Secret { from: String },
    /// `{ "from_setting": "<key>", "skip_if_empty": true }` sources from
    /// a settings_schema entry; supports `skip_if_empty` to drop the env
    /// var entirely when the setting is unset / empty string.
    Setting {
        from_setting: String,
        #[serde(default)]
        skip_if_empty: bool,
    },
    /// `{ "literal": "..." }` for things like `RUST_LOG=info`.
    Literal { literal: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateCmd {
    pub cmd: Vec<String>,
    #[serde(default)]
    pub expect_exit: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryNotice {
    #[serde(default)]
    pub vendor_dials_home: bool,
    #[serde(default)]
    pub privacy_url: Option<String>,
    #[serde(default)]
    pub opt_out_env: Option<String>,
}

impl AgentManifest {
    /// Picks the right [`PlatformInstall`] based on the current host OS.
    pub fn platform_install(&self) -> Option<&PlatformInstall> {
        match std::env::consts::OS {
            "macos" => self.install.macos.as_ref(),
            "linux" => self.install.linux.as_ref(),
            "windows" => self.install.windows.as_ref(),
            _ => None,
        }
    }

    /// Is this agent advertised as runnable on the current host?
    pub fn supports_current_platform(&self) -> bool {
        let host = std::env::consts::OS;
        self.platforms.iter().any(|p| p == host)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn manifest_round_trips() {
        let m = json!({
            "id": "x",
            "version": 1,
            "name": "X",
            "vendor": "Y",
            "platforms": ["macos"],
            "detect": { "command": "x" },
            "install": {},
            "auth": { "primary": { "kind": "api_key_env", "env_var": "X_KEY" } },
            "launch": { "exec": "x" }
        });
        let parsed: AgentManifest = serde_json::from_value(m.clone()).unwrap();
        assert_eq!(parsed.id, "x");
        assert!(parsed.profile_defaults.isolate_env);
    }
}
