//! Append-only index of recorded sessions stored at
//! `~/.unterm/sessions/index.json`. The file is a single JSON array; we
//! load the whole thing, mutate, and rewrite. The file is small (one
//! entry per session), so this is fine.

use anyhow::{Context, Result};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::OnceLock;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IndexEntry {
    pub unterm_session_id: String,
    pub tab_id: u64,
    pub project_path: Option<String>,
    pub project_slug: String,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub block_count: u64,
    pub total_lines: u64,
    pub bytes_raw: u64,
    pub log_path: String,
    pub md_path: String,
    pub exit_reason: Option<String>,
    pub parent_session_id: Option<String>,
    pub osc133_active: bool,
    pub redaction_active: bool,
    pub redaction_count: u64,
    pub trace_ids: Vec<String>,
}

fn index_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

pub fn sessions_root() -> PathBuf {
    dirs_next::home_dir()
        .unwrap_or_default()
        .join(".unterm")
        .join("sessions")
}

pub fn index_path() -> PathBuf {
    sessions_root().join("index.json")
}

pub fn load_index() -> Result<Vec<IndexEntry>> {
    let _g = index_lock().lock();
    let path = index_path();
    if !path.exists() {
        return Ok(Vec::new());
    }
    let raw = std::fs::read_to_string(&path)
        .with_context(|| format!("read {}", path.display()))?;
    if raw.trim().is_empty() {
        return Ok(Vec::new());
    }
    let parsed: Vec<IndexEntry> = serde_json::from_str(&raw)
        .with_context(|| format!("parse {}", path.display()))?;
    Ok(parsed)
}

#[allow(dead_code)]
pub fn append_entry(entry: IndexEntry) -> Result<()> {
    let _g = index_lock().lock();
    let path = index_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).context("create sessions dir")?;
    }
    let mut current: Vec<IndexEntry> = if path.exists() {
        let raw = std::fs::read_to_string(&path).unwrap_or_default();
        if raw.trim().is_empty() {
            Vec::new()
        } else {
            serde_json::from_str(&raw).unwrap_or_default()
        }
    } else {
        Vec::new()
    };
    current.push(entry);
    let serialized = serde_json::to_string_pretty(&current)?;
    std::fs::write(&path, serialized).with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

/// Update an existing entry in-place by `unterm_session_id`. If the entry
/// is missing the function appends it.
pub fn upsert_entry(entry: IndexEntry) -> Result<()> {
    let _g = index_lock().lock();
    let path = index_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).context("create sessions dir")?;
    }
    let mut current: Vec<IndexEntry> = if path.exists() {
        let raw = std::fs::read_to_string(&path).unwrap_or_default();
        if raw.trim().is_empty() {
            Vec::new()
        } else {
            serde_json::from_str(&raw).unwrap_or_default()
        }
    } else {
        Vec::new()
    };

    if let Some(existing) = current
        .iter_mut()
        .find(|e| e.unterm_session_id == entry.unterm_session_id)
    {
        *existing = entry;
    } else {
        current.push(entry);
    }
    let serialized = serde_json::to_string_pretty(&current)?;
    std::fs::write(&path, serialized).with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

pub fn find_entry(session_id: &str) -> Result<Option<IndexEntry>> {
    let entries = load_index()?;
    Ok(entries.into_iter().find(|e| e.unterm_session_id == session_id))
}
