//! AI model definitions and runtime state.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq)]
pub enum ModelProvider {
    Claude,
    OpenAI,
    Gemini,
    Custom,
}

impl ModelProvider {
    pub fn from_name(name: &str) -> Self {
        match name.to_lowercase().as_str() {
            "claude" => Self::Claude,
            "openai" | "gpt" => Self::OpenAI,
            "gemini" => Self::Gemini,
            _ => Self::Custom,
        }
    }

    pub fn display_icon(&self) -> &'static str {
        match self {
            Self::Claude => "◆",
            Self::OpenAI => "○",
            Self::Gemini => "◇",
            Self::Custom => "●",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Claude => "Claude",
            Self::OpenAI => "GPT",
            Self::Gemini => "Gemini",
            Self::Custom => "Custom",
        }
    }
}

/// Runtime AI state shared across threads.
#[derive(Clone)]
pub struct AiState {
    inner: Arc<Mutex<AiStateInner>>,
    /// Set by MCP/background threads when state changes; cleared by render loop.
    dirty: Arc<AtomicBool>,
    /// Callback to invalidate the window when state changes from a background thread.
    invalidate_fn: Arc<Mutex<Option<Box<dyn Fn() + Send>>>>,
}

struct AiStateInner {
    active_model: String,
    provider: ModelProvider,
    /// Last AI response text (for display in insights panel)
    pub last_insight: Option<InsightCard>,
    /// Whether the insights panel is visible
    pub panel_visible: bool,
    /// Whether the chat input is focused
    pub chat_focused: bool,
    /// Current chat input text
    pub chat_input: String,
    /// Chat history (user message, AI response pairs)
    pub chat_history: Vec<ChatMessage>,
    /// Scroll offset for the panel (0 = bottom/latest)
    pub scroll_offset: usize,
}

#[derive(Debug, Clone)]
pub struct InsightCard {
    pub title: String,
    pub content: String,
    pub card_type: InsightType,
    /// Optional command to execute
    pub command: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ChatRole {
    User,
    Assistant,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InsightType {
    Error,
    Suggestion,
    Info,
    Chat,
}

impl AiState {
    pub fn new(default_model: &str) -> Self {
        let provider = ModelProvider::from_name(default_model);
        Self {
            dirty: Arc::new(AtomicBool::new(false)),
            invalidate_fn: Arc::new(Mutex::new(None)),
            inner: Arc::new(Mutex::new(AiStateInner {
                active_model: default_model.to_string(),
                provider,
                last_insight: None,
                panel_visible: false,
                chat_focused: false,
                chat_input: String::new(),
                chat_history: Vec::new(),
                scroll_offset: 0,
            })),
        }
    }

    /// Set the window invalidate callback. Called once during window initialization.
    pub fn set_invalidate_fn(&self, f: Box<dyn Fn() + Send>) {
        *self.invalidate_fn.lock().unwrap() = Some(f);
    }

    /// Mark state as dirty (needs repaint). Called by MCP/background threads.
    /// Also triggers window invalidation if a callback is registered.
    pub fn mark_dirty(&self) {
        self.dirty.store(true, Ordering::Release);
        if let Some(f) = self.invalidate_fn.lock().unwrap().as_ref() {
            f();
        }
    }

    /// Check dirty flag without clearing. Used to decide whether to schedule timer.
    pub fn is_dirty(&self) -> bool {
        self.dirty.load(Ordering::Acquire)
    }

    /// Check and clear dirty flag. Called by render loop.
    pub fn take_dirty(&self) -> bool {
        self.dirty.swap(false, Ordering::AcqRel)
    }

    pub fn active_model(&self) -> String {
        self.inner.lock().unwrap().active_model.clone()
    }

    pub fn provider(&self) -> ModelProvider {
        self.inner.lock().unwrap().provider.clone()
    }

    pub fn set_model(&self, model: &str) {
        let mut inner = self.inner.lock().unwrap();
        inner.active_model = model.to_string();
        inner.provider = ModelProvider::from_name(model);
    }

    /// Cycle through available providers: Claude → OpenAI → Gemini → Claude
    pub fn cycle_provider(&self) {
        let config = config::configuration();
        let mut inner = self.inner.lock().unwrap();
        let (next_provider, next_model) = match inner.provider {
            ModelProvider::Claude => (ModelProvider::OpenAI, config.ai_openai_model.clone()),
            ModelProvider::OpenAI => (ModelProvider::Gemini, config.ai_gemini_model.clone()),
            ModelProvider::Gemini | ModelProvider::Custom => {
                (ModelProvider::Claude, config.ai_claude_model.clone())
            }
        };
        inner.provider = next_provider.clone();
        inner.active_model = if next_model.is_empty() {
            next_provider.display_name().to_string()
        } else {
            next_model
        };
        drop(inner);
        self.mark_dirty();
    }

    pub fn toggle_panel(&self) -> bool {
        let mut inner = self.inner.lock().unwrap();
        inner.panel_visible = !inner.panel_visible;
        self.mark_dirty();
        inner.panel_visible
    }

    pub fn panel_visible(&self) -> bool {
        self.inner.lock().unwrap().panel_visible
    }

    pub fn set_panel_visible(&self, visible: bool) {
        self.inner.lock().unwrap().panel_visible = visible;
        self.mark_dirty();
    }

    pub fn set_insight(&self, card: InsightCard) {
        self.inner.lock().unwrap().last_insight = Some(card);
        self.mark_dirty();
    }

    pub fn get_insight(&self) -> Option<InsightCard> {
        self.inner.lock().unwrap().last_insight.clone()
    }

    pub fn clear_insight(&self) {
        self.inner.lock().unwrap().last_insight = None;
    }

    pub fn chat_focused(&self) -> bool {
        self.inner.lock().unwrap().chat_focused
    }

    pub fn set_chat_focused(&self, focused: bool) {
        self.inner.lock().unwrap().chat_focused = focused;
        self.mark_dirty();
    }

    pub fn toggle_chat_focus(&self) -> bool {
        let mut inner = self.inner.lock().unwrap();
        inner.chat_focused = !inner.chat_focused;
        inner.chat_focused
    }

    pub fn chat_input(&self) -> String {
        self.inner.lock().unwrap().chat_input.clone()
    }

    pub fn chat_input_push(&self, c: char) {
        self.inner.lock().unwrap().chat_input.push(c);
        self.mark_dirty();
    }

    pub fn chat_input_pop(&self) {
        self.inner.lock().unwrap().chat_input.pop();
    }

    pub fn chat_input_take(&self) -> String {
        let mut inner = self.inner.lock().unwrap();
        std::mem::take(&mut inner.chat_input)
    }

    pub fn add_chat_message(&self, msg: ChatMessage) {
        self.inner.lock().unwrap().chat_history.push(msg);
        self.mark_dirty();
    }

    pub fn chat_history(&self) -> Vec<ChatMessage> {
        self.inner.lock().unwrap().chat_history.clone()
    }

    pub fn scroll_offset(&self) -> usize {
        self.inner.lock().unwrap().scroll_offset
    }

    pub fn scroll_up(&self, lines: usize) {
        let mut inner = self.inner.lock().unwrap();
        inner.scroll_offset = inner.scroll_offset.saturating_add(lines);
    }

    pub fn scroll_down(&self, lines: usize) {
        let mut inner = self.inner.lock().unwrap();
        inner.scroll_offset = inner.scroll_offset.saturating_sub(lines);
    }

    pub fn reset_scroll(&self) {
        self.inner.lock().unwrap().scroll_offset = 0;
    }
}

lazy_static::lazy_static! {
    static ref AI_STATE: AiState = {
        let config = config::configuration();
        AiState::new(&config.ai_default_model)
    };
}

pub fn ai_state() -> &'static AiState {
    &AI_STATE
}
