//! Reading what has already been recorded.
//!
//! The recorder needs a live pane; listing and reading finished sessions needs
//! only the files under `~/.unterm/sessions/`. This is that half, so a front
//! end can show someone their recordings without owning a recorder.

use super::index::{self, IndexEntry};
use super::render::{self, RenderConfig};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[allow(dead_code)]
const ROTATE_BYTES: u64 = 5 * 1024 * 1024;
#[allow(dead_code)]
const ROTATE_BLOCKS: u64 = 1000;

/// Persistent config loaded from `~/.unterm/recording.json`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RecordingConfig {
    #[serde(default)]
    pub recording: RecordingFlags,
    #[serde(default)]
    pub redaction: RedactionFlags,
    #[serde(default = "default_idle_minutes")]
    pub idle_rotate_minutes: u64,
}

fn default_idle_minutes() -> u64 {
    5
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RecordingFlags {
    #[serde(default)]
    pub enabled: bool,
}

impl Default for RecordingFlags {
    fn default() -> Self {
        Self { enabled: false }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RedactionFlags {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub custom_patterns: Vec<String>,
}

impl Default for RedactionFlags {
    fn default() -> Self {
        Self {
            enabled: true,
            custom_patterns: Vec::new(),
        }
    }
}

fn default_true() -> bool {
    true
}

impl Default for RecordingConfig {
    fn default() -> Self {
        Self {
            recording: RecordingFlags::default(),
            redaction: RedactionFlags::default(),
            idle_rotate_minutes: default_idle_minutes(),
        }
    }
}

fn config_path() -> PathBuf {
    dirs_next::home_dir()
        .unwrap_or_default()
        .join(".unterm")
        .join("recording.json")
}

pub fn load_config() -> RecordingConfig {
    let p = config_path();
    if !p.exists() {
        return RecordingConfig::default();
    }
    match std::fs::read_to_string(&p) {
        Ok(raw) => serde_json::from_str(&raw).unwrap_or_default(),
        Err(_) => RecordingConfig::default(),
    }
}

#[allow(dead_code)]

pub fn list_sessions(project_filter: Option<&str>) -> Result<Vec<IndexEntry>> {
    let entries = index::load_index()?;
    let filtered: Vec<IndexEntry> = entries
        .into_iter()
        .filter(|e| match project_filter {
            Some(p) => {
                e.project_slug == p || e.project_path.as_deref().map(|x| x == p).unwrap_or(false)
            }
            None => true,
        })
        .collect();
    Ok(filtered)
}

/// Render a session's markdown on demand by reading its log file.
pub fn read_session_markdown(session_id: &str) -> Result<String> {
    let entry = index::find_entry(session_id)?
        .ok_or_else(|| anyhow!("Unknown session_id {}", session_id))?;
    let log_path = Path::new(&entry.log_path);
    let cfg = load_config();
    let render_cfg = RenderConfig {
        redaction_enabled: cfg.redaction.enabled,
        custom_patterns: cfg.redaction.custom_patterns.clone(),
    };
    let out = render::render_log(log_path, &entry, &render_cfg)?;
    Ok(out.markdown)
}
