//! Window session state persistence for Unterm.
//!
//! Saves window geometry (position + size) and tab CWDs on close,
//! and restores them on the next launch.
//!
//! State file: `~/.unterm/last_session.json`

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Persisted state for a single tab.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabState {
    /// Working directory of the active pane in this tab, if known.
    pub cwd: Option<String>,
    /// Tab title.
    pub title: String,
}

/// Persisted window geometry and tab info.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionState {
    /// Window position – x coordinate of client area (screen pixels).
    pub x: i32,
    /// Window position – y coordinate of client area (screen pixels).
    pub y: i32,
    /// Client area width in pixels.
    pub width: usize,
    /// Client area height in pixels.
    pub height: usize,
    /// DPI at the time of saving.
    pub dpi: usize,
    /// Tabs that were open.
    pub tabs: Vec<TabState>,
    /// Timestamp (RFC 3339).
    pub saved_at: String,
}

fn state_path() -> PathBuf {
    dirs_next::home_dir()
        .unwrap_or_default()
        .join(".unterm")
        .join("last_session.json")
}

/// Save session state to disk.
pub fn save_session_state(state: &SessionState) -> Result<()> {
    let path = state_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(state)?;
    std::fs::write(&path, json)?;
    log::info!("Session state saved to {}", path.display());
    Ok(())
}

/// Load session state from disk, if it exists. Sizes that are clearly bogus
/// (too small to be usable) are dropped — this prevents a single accidental
/// resize from locking the user into a tiny window across restarts.
pub fn load_session_state() -> Option<SessionState> {
    /// Minimum acceptable persisted window size in physical pixels. Anything
    /// smaller almost certainly came from a misclick / runaway resize and
    /// should fall back to `default_initial_cols/rows` instead.
    const MIN_W: usize = 800;
    const MIN_H: usize = 480;

    let path = state_path();
    if !path.exists() {
        return None;
    }
    let data = match std::fs::read_to_string(&path) {
        Ok(data) => data,
        Err(e) => {
            log::warn!("Failed to read session state: {}", e);
            return None;
        }
    };
    let state: SessionState = match serde_json::from_str(&data) {
        Ok(state) => state,
        Err(e) => {
            log::warn!("Failed to parse session state: {}", e);
            return None;
        }
    };
    if state.width < MIN_W || state.height < MIN_H {
        log::info!(
            "Ignoring tiny saved window ({}x{}); falling back to default size",
            state.width,
            state.height,
        );
        return None;
    }
    log::info!("Session state loaded from {}", path.display());
    Some(state)
}
