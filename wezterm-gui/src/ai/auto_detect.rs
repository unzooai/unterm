//! Auto error detection — monitors terminal output for errors and triggers AI analysis.

use super::client;
use super::models::{ai_state, InsightCard, InsightType, ModelProvider};
use mux::pane::PaneId;
use mux::Mux;
use std::sync::Mutex;
use std::time::Instant;
use termwiz::surface::line::Line;

/// Minimum interval between auto-detection checks (seconds).
const DEBOUNCE_SECS: u64 = 3;

static LAST_CHECK: Mutex<Option<Instant>> = Mutex::new(None);
static LAST_ERROR_HASH: Mutex<u64> = Mutex::new(0);

/// Minimum interval between flow recommendation checks (seconds).
const FLOW_DEBOUNCE_SECS: u64 = 8;
static LAST_FLOW_CHECK: Mutex<Option<Instant>> = Mutex::new(None);
static LAST_SCREEN_HASH: Mutex<u64> = Mutex::new(0);

const ERROR_PATTERNS: &[&str] = &[
    "error:",
    "Error:",
    "ERROR:",
    "error[",
    "fatal:",
    "Fatal:",
    "FATAL:",
    "panic:",
    "PANIC:",
    "command not found",
    "Permission denied",
    "permission denied",
    "No such file or directory",
    "FAILED",
    "Traceback",
    "Exception:",
    "Segmentation fault",
];

/// Called from mux_pane_output_event. Debounces and checks for errors.
pub fn on_pane_output(pane_id: PaneId) {
    // Only check if AI panel is visible or auto-detect is enabled
    let config = config::configuration();
    if !ai_state().panel_visible() && !config.ai_insights_panel {
        return;
    }

    // Check API key availability
    let model_name = ai_state().active_model();
    let provider = ModelProvider::from_name(&model_name);
    let api_key = match &provider {
        ModelProvider::Claude => config.ai_claude_api_key.clone(),
        ModelProvider::OpenAI => config.ai_openai_api_key.clone(),
        ModelProvider::Gemini => config.ai_gemini_api_key.clone(),
        ModelProvider::Custom => String::new(),
    };
    if api_key.is_empty() {
        return;
    }

    // Debounce
    {
        let mut last = LAST_CHECK.lock().unwrap();
        if let Some(t) = *last {
            if t.elapsed().as_secs() < DEBOUNCE_SECS {
                return;
            }
        }
        *last = Some(Instant::now());
    }

    // Get screen content
    let mux = match Mux::try_get() {
        Some(mux) => mux,
        None => return,
    };
    let pane = match mux.get_pane(pane_id) {
        Some(p) => p,
        None => return,
    };

    let dims = pane.get_dimensions();
    let first_row = dims.physical_top;
    let last_row = first_row + dims.viewport_rows as isize;
    let (_first, lines) = pane.get_lines(first_row..last_row);

    // Check for error patterns
    let mut error_lines = Vec::new();
    for line in &lines {
        let text = line.as_str();
        let trimmed = text.trim();
        if trimmed.is_empty() {
            continue;
        }
        for pattern in ERROR_PATTERNS {
            if trimmed.contains(pattern) {
                error_lines.push(trimmed.to_string());
                break;
            }
        }
    }

    if error_lines.is_empty() {
        // No errors — check for flow recommendation (command completed successfully)
        maybe_suggest_next_step(pane_id, &lines, &provider, &api_key, &config);
        return;
    }

    // Simple hash to avoid re-analyzing the same errors
    let hash = {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        error_lines.hash(&mut hasher);
        hasher.finish()
    };

    {
        let mut last_hash = LAST_ERROR_HASH.lock().unwrap();
        if *last_hash == hash {
            return;
        }
        *last_hash = hash;
    }

    // Build screen context
    let screen_context: String = lines
        .iter()
        .map(|line| line.as_str().trim_end().to_string())
        .collect::<Vec<_>>()
        .join("\n");

    // Detect shell type
    let shell_type = detect_shell_type(&pane);

    // Get AI config
    let model_id = match &provider {
        ModelProvider::Claude => config.ai_claude_model.clone(),
        ModelProvider::OpenAI => config.ai_openai_model.clone(),
        ModelProvider::Gemini => config.ai_gemini_model.clone(),
        ModelProvider::Custom => String::new(),
    };

    // Spawn AI analysis in background thread (not on main thread)
    std::thread::spawn(move || {
        match client::analyze_error(&provider, &api_key, &model_id, &shell_type, &screen_context) {
            Ok((explanation, fix_command)) => {
                ai_state().set_insight(InsightCard {
                    title: "Error Detected".to_string(),
                    content: explanation,
                    card_type: InsightType::Error,
                    command: fix_command,
                });
                log::info!("Auto error detection: AI analysis posted to insights panel");
            }
            Err(e) => {
                log::warn!("Auto error detection: AI analysis failed: {}", e);
            }
        }
    });
}

/// Prompt markers that indicate a command just completed and shell is ready.
const PROMPT_MARKERS: &[&str] = &["$ ", "> ", "% ", "# ", "PS ", "❯ "];

/// Check if the last non-empty line looks like a fresh prompt (command completed).
/// If so, call suggest_next_step() in a background thread.
fn maybe_suggest_next_step(
    pane_id: PaneId,
    lines: &[Line],
    provider: &ModelProvider,
    api_key: &str,
    config: &config::ConfigHandle,
) {
    // Only suggest if AI panel is visible
    if !ai_state().panel_visible() {
        return;
    }

    // Debounce flow recommendations independently (longer interval)
    {
        let last = LAST_FLOW_CHECK.lock().unwrap();
        if let Some(t) = *last {
            if t.elapsed().as_secs() < FLOW_DEBOUNCE_SECS {
                return;
            }
        }
    }

    // Build screen text and find the last non-empty line
    let screen_lines: Vec<String> = lines
        .iter()
        .map(|l| l.as_str().trim_end().to_string())
        .collect();

    // Find last non-empty line — should be a prompt
    let last_line = match screen_lines.iter().rev().find(|l| !l.is_empty()) {
        Some(l) => l.clone(),
        None => return,
    };

    // Check if the last line looks like a prompt
    let trimmed = last_line.trim();
    let is_prompt = PROMPT_MARKERS.iter().any(|m| trimmed.contains(m));
    if !is_prompt {
        return;
    }

    // Need at least some output above the prompt (not just a fresh shell)
    let non_empty_count = screen_lines.iter().filter(|l| !l.is_empty()).count();
    if non_empty_count < 3 {
        return;
    }

    // Hash check — don't re-suggest for the same screen
    let screen_hash = {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        screen_lines.hash(&mut hasher);
        hasher.finish()
    };
    {
        let mut last_hash = LAST_SCREEN_HASH.lock().unwrap();
        if *last_hash == screen_hash {
            return;
        }
        *last_hash = screen_hash;
    }

    // Update debounce timer
    {
        let mut last = LAST_FLOW_CHECK.lock().unwrap();
        *last = Some(Instant::now());
    }

    let mux = match Mux::try_get() {
        Some(m) => m,
        None => return,
    };
    let pane = match mux.get_pane(pane_id) {
        Some(p) => p,
        None => return,
    };

    let shell_type = detect_shell_type(&pane);
    let screen_context = screen_lines.join("\n");
    let model_id = match provider {
        ModelProvider::Claude => config.ai_claude_model.clone(),
        ModelProvider::OpenAI => config.ai_openai_model.clone(),
        ModelProvider::Gemini => config.ai_gemini_model.clone(),
        ModelProvider::Custom => String::new(),
    };
    let provider = provider.clone();
    let api_key = api_key.to_string();

    std::thread::spawn(move || {
        match client::suggest_next_step(
            &provider,
            &api_key,
            &model_id,
            &shell_type,
            &screen_context,
        ) {
            Ok((suggestion, command)) => {
                ai_state().set_insight(InsightCard {
                    title: "Next Step".to_string(),
                    content: suggestion,
                    card_type: InsightType::Suggestion,
                    command,
                });
                log::info!("Flow recommendation posted to insights panel");
            }
            Err(e) => {
                log::warn!("Flow recommendation failed: {}", e);
            }
        }
    });
}

fn detect_shell_type(pane: &std::sync::Arc<dyn mux::pane::Pane>) -> String {
    if let Some(name) = pane.get_foreground_process_name(mux::pane::CachePolicy::AllowStale) {
        let lower = name.to_lowercase();
        if lower.contains("pwsh") {
            "PowerShell 7".to_string()
        } else if lower.contains("powershell") {
            "PowerShell 5".to_string()
        } else if lower.contains("cmd") {
            "CMD".to_string()
        } else if lower.contains("nu") {
            "Nushell".to_string()
        } else if lower.contains("bash") {
            // Check if running under WSL
            if is_wsl_process(pane) {
                "Bash (WSL)".to_string()
            } else {
                "Bash".to_string()
            }
        } else if lower.contains("zsh") {
            if is_wsl_process(pane) {
                "Zsh (WSL)".to_string()
            } else {
                "Zsh".to_string()
            }
        } else if lower.contains("fish") {
            if is_wsl_process(pane) {
                "Fish (WSL)".to_string()
            } else {
                "Fish".to_string()
            }
        } else if lower.contains("wsl") {
            "WSL".to_string()
        } else {
            "unknown".to_string()
        }
    } else {
        "unknown".to_string()
    }
}

/// Check if a pane's process is running under WSL by inspecting the process path.
fn is_wsl_process(pane: &std::sync::Arc<dyn mux::pane::Pane>) -> bool {
    if let Some(name) = pane.get_foreground_process_name(mux::pane::CachePolicy::AllowStale) {
        let lower = name.to_lowercase();
        // WSL processes typically have paths like /usr/bin/bash or run via wsl.exe
        lower.contains("wsl") || lower.starts_with("/")
    } else {
        false
    }
}
