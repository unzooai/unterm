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
    /// Auth modes the user can pick from. First entry is the default if
    /// the per-profile settings haven't recorded a selection yet. Empty
    /// `auth_modes` falls back to the legacy `auth.primary/fallback`
    /// shape so manifests authored before v0.18.1 still parse.
    #[serde(default)]
    pub auth_modes: Vec<AuthModeSpec>,
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

/// One user-selectable auth mode for an agent (e.g., "official subscription
/// OAuth", "bring your own API key", "custom gateway/endpoint").
///
/// The current selection is stored as the synthetic setting key
/// `_auth_mode` per (profile, agent) — see SettingsState.values. The
/// launcher filters `settings_storage.env_at_launch` entries by the
/// `only_if_auth_mode` field so that, for example, ANTHROPIC_API_KEY is
/// NEVER injected when the user has picked the "subscription" mode for
/// Claude Code (otherwise we'd silently bypass their Pro subscription
/// and bill their API key — exactly the operational footgun we want
/// to avoid).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthModeSpec {
    pub id: String,
    #[serde(default)]
    pub recommended: bool,
    #[serde(default)]
    pub label_i18n: BTreeMap<String, String>,
    #[serde(default)]
    pub description_i18n: BTreeMap<String, String>,
    /// If present, this mode requires the user to run an interactive
    /// OAuth login (typically `<agent> login`). When the user picks
    /// this mode, the SPA + CLI prompt them to run this command in a
    /// real terminal.
    #[serde(default)]
    pub oauth_trigger: Option<ShellCmd>,
    #[serde(default)]
    pub oauth_ready_marker: Option<String>,
    #[serde(default = "default_oauth_timeout")]
    pub oauth_timeout_s: u64,
    /// External console URL for getting an API key (BYO mode) or signing
    /// up for a subscription. Pure UX hint; ignored by the launcher.
    #[serde(default)]
    pub console_url: Option<String>,
    /// Setting keys to show in the UI only when this mode is active.
    /// (Inverse of `only_if_auth_mode` on env_at_launch — this is the
    /// UI side.) E.g., custom_endpoint mode reveals "base_url".
    #[serde(default)]
    pub reveals_settings: Vec<String>,
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
    /// Optional launch flags the user can pick from in Web Settings or have
    /// auto-completed in the terminal. Never applied unless the user selects
    /// them (or `default_on` for the few safe ones). `#[serde(default)]` so
    /// older envelopes without this field still parse.
    #[serde(default)]
    pub flag_catalog: Vec<FlagSpec>,
}

/// One selectable launch flag for an agent. The catalog is the single source
/// of truth consumed by the Web Settings picker, the in-terminal ghost-text
/// completion, and the launch-command composer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlagSpec {
    /// Stable id used to persist the user's selection (survives label edits).
    pub id: String,
    /// Argument template. Toggles use the literal flag ("--skip-trust").
    /// Value/choice flags use a "{value}" placeholder ("--model {value}");
    /// the composer substitutes the chosen value and splits on spaces.
    pub arg: String,
    #[serde(default)]
    pub label_i18n: BTreeMap<String, String>,
    #[serde(default)]
    pub description_i18n: BTreeMap<String, String>,
    #[serde(default)]
    pub kind: FlagKind,
    /// Allowed values when `kind == Choice`.
    #[serde(default)]
    pub choices: Vec<String>,
    #[serde(default)]
    pub risk: FlagRisk,
    /// Pre-selected when true. Only ever set for `Safe` flags.
    #[serde(default)]
    pub default_on: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FlagKind {
    /// A bare on/off flag (`--skip-trust`).
    #[default]
    Toggle,
    /// Takes a free-text value (`--model gpt-5`).
    Value,
    /// Takes one of `choices` (`--approval-mode auto_edit`).
    Choice,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FlagRisk {
    /// No destructive effect (cwd, trust-this-folder, model selection).
    #[default]
    Safe,
    /// Changes behavior but not destructive (auto-approve edits only).
    Caution,
    /// Auto-approves destructive actions (yolo / skip-permissions). Never
    /// default_on; rendered with a warning in the picker.
    Danger,
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
    /// `{ "from": "secret:<namespace>", "only_if_auth_mode": ["byo_key"] }`
    /// reads from the OS keychain. Empty or missing `only_if_auth_mode`
    /// = inject under every mode (the default).
    Secret {
        from: String,
        #[serde(default)]
        only_if_auth_mode: Vec<String>,
    },
    /// `{ "from_setting": "<key>", "skip_if_empty": true,
    ///    "only_if_auth_mode": ["custom_endpoint"] }`
    /// sources from a settings_schema entry.
    Setting {
        from_setting: String,
        #[serde(default)]
        skip_if_empty: bool,
        #[serde(default)]
        only_if_auth_mode: Vec<String>,
    },
    /// `{ "literal": "...", "only_if_auth_mode": [...] }` for things like
    /// `RUST_LOG=info`.
    Literal {
        literal: String,
        #[serde(default)]
        only_if_auth_mode: Vec<String>,
    },
}

impl EnvBinding {
    /// Returns whether this binding should be injected given the user's
    /// currently selected auth_mode. Bindings without a filter always
    /// inject; with a filter, only when the mode is listed.
    pub fn applies_to_mode(&self, mode: &str) -> bool {
        let filter = match self {
            EnvBinding::Secret {
                only_if_auth_mode, ..
            } => only_if_auth_mode,
            EnvBinding::Setting {
                only_if_auth_mode, ..
            } => only_if_auth_mode,
            EnvBinding::Literal {
                only_if_auth_mode, ..
            } => only_if_auth_mode,
        };
        filter.is_empty() || filter.iter().any(|m| m == mode)
    }
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
