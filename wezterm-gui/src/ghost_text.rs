//! Ghost Text — AI-generated inline suggestions rendered at cursor position.

use mux::pane::PaneId;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct GhostText {
    pub pane_id: PaneId,
    pub text: String,
    /// Cursor x position when the suggestion was set
    pub cursor_x: usize,
    /// Cursor y (stable row) when the suggestion was set
    pub cursor_y: i64,
}

/// Thread-safe shared ghost text state.
/// Accessed from the MCP server thread (to set/clear) and the render thread (to display).
#[derive(Clone)]
pub struct GhostTextState {
    inner: Arc<Mutex<Option<GhostText>>>,
    /// Generation counter — incremented on every clear/set. Used for debouncing auto-trigger.
    generation: Arc<AtomicU64>,
}

impl GhostTextState {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(None)),
            generation: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn set(&self, ghost: GhostText) {
        self.generation.fetch_add(1, Ordering::SeqCst);
        *self.inner.lock().unwrap() = Some(ghost);
    }

    pub fn clear(&self) {
        self.generation.fetch_add(1, Ordering::SeqCst);
        *self.inner.lock().unwrap() = None;
    }

    pub fn get(&self) -> Option<GhostText> {
        self.inner.lock().unwrap().clone()
    }

    /// Take the ghost text (clear and return it), used when accepting the suggestion.
    pub fn take(&self) -> Option<GhostText> {
        self.generation.fetch_add(1, Ordering::SeqCst);
        self.inner.lock().unwrap().take()
    }

    /// Current generation value.
    pub fn generation(&self) -> u64 {
        self.generation.load(Ordering::SeqCst)
    }
}

lazy_static::lazy_static! {
    static ref GHOST_TEXT: GhostTextState = GhostTextState::new();
}

pub fn ghost_text_state() -> &'static GhostTextState {
    &GHOST_TEXT
}

/// Schedule an auto ghost text suggestion after a delay.
/// If another key is pressed before the delay, the generation changes and we skip.
pub fn schedule_auto_suggest(pane_id: PaneId) {
    use crate::ai::client;
    use crate::ai::models::{ai_state, ModelProvider};

    let config = config::configuration();

    // Check if AI is configured
    let model_name = ai_state().active_model();
    let provider = ModelProvider::from_name(&model_name);
    let api_key = match &provider {
        ModelProvider::Claude => config.ai_claude_api_key.clone(),
        ModelProvider::OpenAI => config.ai_openai_api_key.clone(),
        ModelProvider::Gemini => config.ai_gemini_api_key.clone(),
        ModelProvider::Custom => return,
    };
    if api_key.is_empty() {
        return;
    }

    let model_id = match &provider {
        ModelProvider::Claude => config.ai_claude_model.clone(),
        ModelProvider::OpenAI => config.ai_openai_model.clone(),
        ModelProvider::Gemini => config.ai_gemini_model.clone(),
        ModelProvider::Custom => return,
    };

    let gen = ghost_text_state().generation();

    std::thread::spawn(move || {
        // Wait 600ms, then check if generation is still the same
        std::thread::sleep(std::time::Duration::from_millis(600));

        if ghost_text_state().generation() != gen {
            return; // User typed more, skip
        }

        // Get current pane state
        let mux = match mux::Mux::try_get() {
            Some(mux) => mux,
            None => return,
        };
        let pane = match mux.get_pane(pane_id) {
            Some(p) => p,
            None => return,
        };

        let cursor = pane.get_cursor_position();
        let dims = pane.get_dimensions();

        // Get current line text as partial input
        let first_row = dims.physical_top;
        let last_row = first_row + dims.viewport_rows as isize;
        let (_first, lines) = pane.get_lines(first_row..last_row);

        // Get cursor line
        let cursor_row_offset = (cursor.y as isize - first_row) as usize;
        let partial_input = if cursor_row_offset < lines.len() {
            let line_text = lines[cursor_row_offset].as_str().to_string();
            let trimmed = line_text.trim_end();
            // Extract just the command part (after prompt markers)
            let input = if let Some(pos) = trimmed.rfind("$ ") {
                &trimmed[pos + 2..]
            } else if let Some(pos) = trimmed.rfind("> ") {
                &trimmed[pos + 2..]
            } else if let Some(pos) = trimmed.rfind("% ") {
                &trimmed[pos + 2..]
            } else {
                trimmed
            };
            input.to_string()
        } else {
            return;
        };

        // Don't suggest for empty or very short input
        if partial_input.trim().len() < 2 {
            return;
        }

        // Build screen context (last 20 lines)
        let context_start = if lines.len() > 20 {
            lines.len() - 20
        } else {
            0
        };
        let screen_context: String = lines[context_start..]
            .iter()
            .map(|line| line.as_str().trim_end().to_string())
            .collect::<Vec<_>>()
            .join("\n");

        // Detect shell
        let shell_type = pane
            .get_foreground_process_name(mux::pane::CachePolicy::AllowStale)
            .map(|name| {
                let lower = name.to_lowercase();
                if lower.contains("pwsh") || lower.contains("powershell") {
                    "PowerShell"
                } else if lower.contains("bash") {
                    "Bash"
                } else if lower.contains("zsh") {
                    "Zsh"
                } else if lower.contains("fish") {
                    "Fish"
                } else {
                    "unknown"
                }
            })
            .unwrap_or("unknown");

        let cwd = pane
            .get_current_working_dir(mux::pane::CachePolicy::AllowStale)
            .and_then(|url| {
                let path = url.path().to_string();
                if path.is_empty() { None } else { Some(path) }
            })
            .unwrap_or_else(|| "~".to_string());
        let cwd = cwd.as_str();

        match client::ghost_text_complete(
            &provider,
            &api_key,
            &model_id,
            shell_type,
            cwd,
            &screen_context,
            &partial_input,
        ) {
            Ok(suggestion) => {
                let suggestion = suggestion.trim().to_string();
                if !suggestion.is_empty() && ghost_text_state().generation() == gen {
                    ghost_text_state().set(GhostText {
                        pane_id,
                        text: suggestion,
                        cursor_x: cursor.x,
                        cursor_y: cursor.y as i64,
                    });
                }
            }
            Err(e) => {
                log::debug!("Ghost text auto-suggest failed: {}", e);
            }
        }
    });
}
