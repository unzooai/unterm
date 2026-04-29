//! AI Settings overlay — centered card for configuring AI provider, API key, and model.

use crate::ai::models::{ai_state, ModelProvider};
use mux::termwiztermtab::TermWizTerminal;
use termwiz::cell::CellAttributes;
use termwiz::input::{InputEvent, KeyCode, KeyEvent, Modifiers};
use termwiz::surface::{Change, Position};
use termwiz::terminal::Terminal;

struct SettingsState {
    providers: Vec<ProviderEntry>,
    active_idx: usize,
    editing_key: bool,
    key_buffer: String,
    saved_message: Option<String>,
}

struct ProviderEntry {
    provider: ModelProvider,
    icon: &'static str,
    label: &'static str,
    model: String,
    api_key: String,
}

// Neutral dark palette
const MANTLE: (u8, u8, u8) = (0x1a, 0x1a, 0x1a);
const CRUST: (u8, u8, u8) = (0x10, 0x10, 0x10);
const SURFACE0: (u8, u8, u8) = (0x2d, 0x2d, 0x2d);
const SURFACE1: (u8, u8, u8) = (0x3f, 0x3f, 0x3f);
const TEXT: (u8, u8, u8) = (0xe0, 0xe0, 0xe0);
const SUBTEXT0: (u8, u8, u8) = (0xbb, 0xbb, 0xbb);
const OVERLAY0: (u8, u8, u8) = (0x80, 0x80, 0x80);
const MAUVE: (u8, u8, u8) = (0x61, 0xaf, 0xef);
const GREEN: (u8, u8, u8) = (0xa6, 0xe3, 0xa1);
const RED: (u8, u8, u8) = (0xf3, 0x8b, 0xa8);

pub fn ai_settings(term: &mut TermWizTerminal) -> anyhow::Result<()> {
    let config = config::configuration();

    let providers = vec![
        ProviderEntry {
            provider: ModelProvider::Claude,
            icon: "◆",
            label: "Claude (Anthropic)",
            model: config.ai_claude_model.clone(),
            api_key: config.ai_claude_api_key.clone(),
        },
        ProviderEntry {
            provider: ModelProvider::OpenAI,
            icon: "○",
            label: "GPT (OpenAI)",
            model: config.ai_openai_model.clone(),
            api_key: config.ai_openai_api_key.clone(),
        },
        ProviderEntry {
            provider: ModelProvider::Gemini,
            icon: "◇",
            label: "Gemini (Google)",
            model: config.ai_gemini_model.clone(),
            api_key: config.ai_gemini_api_key.clone(),
        },
    ];

    let current = ai_state().provider();
    let active_idx = providers
        .iter()
        .position(|p| std::mem::discriminant(&p.provider) == std::mem::discriminant(&current))
        .unwrap_or(0);

    let mut state = SettingsState {
        providers,
        active_idx,
        editing_key: false,
        key_buffer: String::new(),
        saved_message: None,
    };

    state.render(term)?;
    state.run_loop(term)?;
    Ok(())
}

impl SettingsState {
    fn render(&self, term: &mut TermWizTerminal) -> termwiz::Result<()> {
        let size = term.get_screen_size()?;
        let term_w = size.cols;
        let term_h = size.rows;

        let card_w = 60usize.min(term_w.saturating_sub(4));
        let card_h = 4 + self.providers.len() * 3 + 5; // header + entries + footer
        let start_x = (term_w.saturating_sub(card_w)) / 2;
        let start_y = (term_h.saturating_sub(card_h)) / 3;

        let mut changes: Vec<Change> =
            vec![Change::ClearScreen(termwiz::color::ColorAttribute::Default)];

        // Fill screen with dim background
        for y in 0..term_h {
            changes.push(Change::CursorPosition {
                x: Position::Absolute(0),
                y: Position::Absolute(y),
            });
            changes.push(fg_bg(" ".repeat(term_w), CRUST, CRUST));
        }

        // Card background
        for y in 0..card_h {
            changes.push(Change::CursorPosition {
                x: Position::Absolute(start_x),
                y: Position::Absolute(start_y + y),
            });
            changes.push(fg_bg(" ".repeat(card_w), TEXT, MANTLE));
        }

        let mut row = start_y;

        // Top border
        let top = format!("╭{}╮", "─".repeat(card_w.saturating_sub(2)));
        changes.push(Change::CursorPosition {
            x: Position::Absolute(start_x),
            y: Position::Absolute(row),
        });
        changes.push(fg_bg(top, SURFACE1, MANTLE));
        row += 1;

        // Title
        let title = "  AI Settings";
        let right_pad = card_w.saturating_sub(title.chars().count() + 5);
        changes.push(Change::CursorPosition {
            x: Position::Absolute(start_x),
            y: Position::Absolute(row),
        });
        changes.push(fg_bg("│ ".to_string(), SURFACE1, MANTLE));
        changes.push(fg_bg("◆".to_string(), MAUVE, MANTLE));
        changes.push(fg_bg(title.to_string(), TEXT, MANTLE));
        changes.push(fg_bg(
            format!("{} │", " ".repeat(right_pad)),
            SURFACE1,
            MANTLE,
        ));
        row += 1;

        // Subtitle
        let subtitle = "  Select provider and configure API key";
        let right_pad = card_w.saturating_sub(subtitle.chars().count() + 4);
        changes.push(Change::CursorPosition {
            x: Position::Absolute(start_x),
            y: Position::Absolute(row),
        });
        changes.push(fg_bg("│ ".to_string(), SURFACE1, MANTLE));
        changes.push(fg_bg(subtitle.to_string(), SUBTEXT0, MANTLE));
        changes.push(fg_bg(
            format!("{} │", " ".repeat(right_pad)),
            SURFACE1,
            MANTLE,
        ));
        row += 1;

        // Separator
        let sep = format!("├{}┤", "─".repeat(card_w.saturating_sub(2)));
        changes.push(Change::CursorPosition {
            x: Position::Absolute(start_x),
            y: Position::Absolute(row),
        });
        changes.push(fg_bg(sep, SURFACE1, MANTLE));
        row += 1;

        // Provider entries
        for (idx, entry) in self.providers.iter().enumerate() {
            let is_selected = idx == self.active_idx;
            let has_key = !entry.api_key.is_empty();

            // Row 1: Provider name + status
            let indicator = if is_selected { "▸" } else { " " };
            let status = if has_key {
                "✓ configured"
            } else {
                "✗ no key"
            };
            let status_color = if has_key { GREEN } else { RED };

            let left_part = format!("  {} {} {}", indicator, entry.icon, entry.label);
            let right_part = format!("{}  ", status);
            let middle_pad =
                card_w.saturating_sub(left_part.chars().count() + right_part.chars().count() + 4);

            let (row_fg, row_bg) = if is_selected {
                (TEXT, SURFACE0)
            } else {
                (SUBTEXT0, MANTLE)
            };

            changes.push(Change::CursorPosition {
                x: Position::Absolute(start_x),
                y: Position::Absolute(row),
            });

            if is_selected {
                changes.push(fg_bg("│".to_string(), SURFACE1, MANTLE));
                changes.push(fg_bg("▎".to_string(), MAUVE, row_bg));
            } else {
                changes.push(fg_bg("│ ".to_string(), SURFACE1, row_bg));
            }
            changes.push(fg_bg(left_part, row_fg, row_bg));
            changes.push(fg_bg(" ".repeat(middle_pad), row_fg, row_bg));
            changes.push(fg_bg(right_part, status_color, row_bg));
            changes.push(fg_bg(" │".to_string(), SURFACE1, MANTLE));
            row += 1;

            // Row 2: Model + masked key
            let model_info = format!("    Model: {}", entry.model);
            let key_display = if has_key {
                let k = &entry.api_key;
                if k.len() > 8 {
                    format!("{}...{}", &k[..4], &k[k.len() - 4..])
                } else {
                    "****".to_string()
                }
            } else {
                "not set".to_string()
            };
            let key_info = format!("Key: {}", key_display);
            let pad =
                card_w.saturating_sub(model_info.chars().count() + key_info.chars().count() + 6);

            changes.push(Change::CursorPosition {
                x: Position::Absolute(start_x),
                y: Position::Absolute(row),
            });
            changes.push(fg_bg("│ ".to_string(), SURFACE1, row_bg));
            changes.push(fg_bg(model_info, OVERLAY0, row_bg));
            changes.push(fg_bg(" ".repeat(pad), row_fg, row_bg));
            changes.push(fg_bg(key_info, OVERLAY0, row_bg));
            changes.push(fg_bg("   │".to_string(), SURFACE1, MANTLE));
            row += 1;

            // Row 3: Edit hint (only for selected)
            if is_selected && self.editing_key {
                let prompt = format!("    Enter API key: {}_", self.key_buffer);
                let pad2 = card_w.saturating_sub(prompt.chars().count() + 4);
                changes.push(Change::CursorPosition {
                    x: Position::Absolute(start_x),
                    y: Position::Absolute(row),
                });
                changes.push(fg_bg("│ ".to_string(), SURFACE1, row_bg));
                changes.push(fg_bg(prompt, MAUVE, row_bg));
                changes.push(fg_bg(format!("{} │", " ".repeat(pad2)), SURFACE1, MANTLE));
            } else if is_selected {
                let hint = "    [E] Edit key  [Enter] Select  [Esc] Close";
                let pad2 = card_w.saturating_sub(hint.chars().count() + 4);
                changes.push(Change::CursorPosition {
                    x: Position::Absolute(start_x),
                    y: Position::Absolute(row),
                });
                changes.push(fg_bg("│ ".to_string(), SURFACE1, row_bg));
                changes.push(fg_bg(hint.to_string(), OVERLAY0, row_bg));
                changes.push(fg_bg(format!("{} │", " ".repeat(pad2)), SURFACE1, MANTLE));
            } else {
                changes.push(Change::CursorPosition {
                    x: Position::Absolute(start_x),
                    y: Position::Absolute(row),
                });
                changes.push(fg_bg("│ ".to_string(), SURFACE1, MANTLE));
                changes.push(fg_bg(" ".repeat(card_w - 4), SUBTEXT0, MANTLE));
                changes.push(fg_bg(" │".to_string(), SURFACE1, MANTLE));
            }
            row += 1;
        }

        // Separator
        let sep = format!("├{}┤", "─".repeat(card_w.saturating_sub(2)));
        changes.push(Change::CursorPosition {
            x: Position::Absolute(start_x),
            y: Position::Absolute(row),
        });
        changes.push(fg_bg(sep, SURFACE1, MANTLE));
        row += 1;

        // Saved message or footer hints
        if let Some(msg) = &self.saved_message {
            let msg_pad = card_w.saturating_sub(msg.chars().count() + 6);
            changes.push(Change::CursorPosition {
                x: Position::Absolute(start_x),
                y: Position::Absolute(row),
            });
            changes.push(fg_bg("│  ".to_string(), SURFACE1, MANTLE));
            changes.push(fg_bg(msg.clone(), GREEN, MANTLE));
            changes.push(fg_bg(
                format!("{}  │", " ".repeat(msg_pad)),
                SURFACE1,
                MANTLE,
            ));
        } else {
            let hints = " ↑↓ Navigate   E Edit key   Enter Select   Esc Close";
            let hint_pad = card_w.saturating_sub(hints.chars().count() + 4);
            changes.push(Change::CursorPosition {
                x: Position::Absolute(start_x),
                y: Position::Absolute(row),
            });
            changes.push(fg_bg("│ ".to_string(), SURFACE1, MANTLE));
            changes.push(fg_bg(hints.to_string(), OVERLAY0, MANTLE));
            changes.push(fg_bg(
                format!("{} │", " ".repeat(hint_pad)),
                SURFACE1,
                MANTLE,
            ));
        }
        row += 1;

        // Bottom border
        let bottom = format!("╰{}╯", "─".repeat(card_w.saturating_sub(2)));
        changes.push(Change::CursorPosition {
            x: Position::Absolute(start_x),
            y: Position::Absolute(row),
        });
        changes.push(fg_bg(bottom, SURFACE1, MANTLE));

        // Reset
        changes.push(Change::AllAttributes(CellAttributes::default()));
        changes.push(Change::CursorPosition {
            x: Position::Absolute(0),
            y: Position::Absolute(0),
        });

        term.render(&changes)
    }

    fn select_provider(&mut self) {
        if let Some(entry) = self.providers.get(self.active_idx) {
            let name = entry.provider.display_name();
            ai_state().set_model(name);
            ai_state().mark_dirty();
            self.saved_message = Some(format!("Switched to {}", entry.label));
        }
    }

    fn save_api_key(&mut self) {
        if self.key_buffer.is_empty() {
            return;
        }
        if let Some(entry) = self.providers.get_mut(self.active_idx) {
            // Save key to config file
            let key_name = match entry.provider {
                ModelProvider::Claude => "ai_claude_api_key",
                ModelProvider::OpenAI => "ai_openai_api_key",
                ModelProvider::Gemini => "ai_gemini_api_key",
                ModelProvider::Custom => return,
            };

            entry.api_key = self.key_buffer.clone();

            // Apply via config override
            if let Err(e) = config::set_config_overrides(&[(
                key_name.to_string(),
                format!("\"{}\"", self.key_buffer),
            )]) {
                log::warn!("Failed to set config override: {}", e);
            }

            self.saved_message = Some(format!("API key saved for {}", entry.label));
            self.key_buffer.clear();
            self.editing_key = false;
        }
    }

    fn run_loop(&mut self, term: &mut TermWizTerminal) -> anyhow::Result<()> {
        while let Ok(Some(event)) = term.poll_input(None) {
            if self.editing_key {
                match event {
                    InputEvent::Key(KeyEvent {
                        key: KeyCode::Escape,
                        ..
                    }) => {
                        self.editing_key = false;
                        self.key_buffer.clear();
                        self.render(term)?;
                    }
                    InputEvent::Key(KeyEvent {
                        key: KeyCode::Enter,
                        ..
                    }) => {
                        self.save_api_key();
                        self.render(term)?;
                    }
                    InputEvent::Key(KeyEvent {
                        key: KeyCode::Backspace,
                        ..
                    }) => {
                        self.key_buffer.pop();
                        self.render(term)?;
                    }
                    InputEvent::Key(KeyEvent {
                        key: KeyCode::Char(c),
                        modifiers: Modifiers::NONE,
                    })
                    | InputEvent::Key(KeyEvent {
                        key: KeyCode::Char(c),
                        modifiers: Modifiers::SHIFT,
                    }) => {
                        self.key_buffer.push(c);
                        self.render(term)?;
                    }
                    _ => {}
                }
            } else {
                match event {
                    InputEvent::Key(KeyEvent {
                        key: KeyCode::Escape,
                        ..
                    }) => break,
                    InputEvent::Key(KeyEvent {
                        key: KeyCode::UpArrow,
                        ..
                    })
                    | InputEvent::Key(KeyEvent {
                        key: KeyCode::Char('k'),
                        modifiers: Modifiers::NONE,
                    }) => {
                        self.active_idx = self.active_idx.saturating_sub(1);
                        self.saved_message = None;
                        self.render(term)?;
                    }
                    InputEvent::Key(KeyEvent {
                        key: KeyCode::DownArrow,
                        ..
                    })
                    | InputEvent::Key(KeyEvent {
                        key: KeyCode::Char('j'),
                        modifiers: Modifiers::NONE,
                    }) => {
                        self.active_idx =
                            (self.active_idx + 1).min(self.providers.len().saturating_sub(1));
                        self.saved_message = None;
                        self.render(term)?;
                    }
                    InputEvent::Key(KeyEvent {
                        key: KeyCode::Enter,
                        ..
                    }) => {
                        self.select_provider();
                        self.render(term)?;
                    }
                    InputEvent::Key(KeyEvent {
                        key: KeyCode::Char('e') | KeyCode::Char('E'),
                        ..
                    }) => {
                        self.editing_key = true;
                        self.key_buffer.clear();
                        self.saved_message = None;
                        self.render(term)?;
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }
}

fn fg_bg(text: String, fg: (u8, u8, u8), bg: (u8, u8, u8)) -> Change {
    Change::Text(format!(
        "\x1b[38;2;{};{};{}m\x1b[48;2;{};{};{}m{}\x1b[0m",
        fg.0, fg.1, fg.2, bg.0, bg.1, bg.2, text
    ))
}
