//! The last window, brought back.
//!
//! Closing writes what mattered — the window's size, whether it was
//! maximised, and where each tab was — and the next plain launch reads it,
//! so a terminal opens where its owner left off instead of at the factory
//! defaults. A launch that names a directory or a command on the line is
//! asking for something specific and is left alone.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LastSession {
    /// Physical pixels, exactly as the window reported them.
    pub width: u32,
    pub height: u32,
    pub maximized: bool,
    /// One entry per tab, in the strip's order.
    pub cwds: Vec<String>,
}

fn path() -> Option<std::path::PathBuf> {
    // `UNTERM_STATE_DIR` replaces ~/.unterm wholesale -- the same
    // contract the instance registry, the bridge records, the config
    // and the Core's discovery all honor. Without it here, a test or
    // a headless run reopened the real user's tabs and then wrote its
    // own over them.
    if let Some(dir) = std::env::var_os("UNTERM_STATE_DIR") {
        if !dir.is_empty() {
            return Some(std::path::PathBuf::from(dir).join("last_session.json"));
        }
    }
    dirs_next::home_dir().map(|home| home.join(".unterm").join("last_session.json"))
}

pub fn save(state: &LastSession) {
    let Some(path) = path() else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(text) = serde_json::to_string_pretty(state) {
        let _ = std::fs::write(path, text);
    }
}

pub fn load() -> Option<LastSession> {
    let text = std::fs::read_to_string(path()?).ok()?;
    let state: LastSession = serde_json::from_str(&text).ok()?;
    // A window smaller than a postage stamp is stored corruption, not a
    // preference.
    (state.width >= 200 && state.height >= 150).then_some(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_round_trip_keeps_every_field() {
        let state = LastSession {
            width: 1600,
            height: 900,
            maximized: true,
            cwds: vec!["D:/code".into(), "D:/code/unterm".into()],
        };
        let text = serde_json::to_string(&state).unwrap();
        let back: LastSession = serde_json::from_str(&text).unwrap();
        assert_eq!(back, state);
    }
}
