//! Theme selector overlay for Unterm visual styles.

use mux::termwiztermtab::TermWizTerminal;
use serde_json::json;
use termwiz::cell::CellAttributes;
use termwiz::input::{InputEvent, KeyCode, KeyEvent};
use termwiz::surface::{Change, Position};
use termwiz::terminal::Terminal;
use window::WindowOps;

#[derive(Clone)]
struct ThemePreset {
    id: &'static str,
    name: &'static str,
    scheme: &'static str,
    desc: &'static str,
}

struct ThemeState {
    active: String,
    selected: usize,
    message: Option<String>,
    presets: Vec<ThemePreset>,
}

const MANTLE: (u8, u8, u8) = (0x1a, 0x1a, 0x1a);
const CRUST: (u8, u8, u8) = (0x10, 0x10, 0x10);
const SURFACE0: (u8, u8, u8) = (0x2d, 0x2d, 0x2d);
const SURFACE1: (u8, u8, u8) = (0x3f, 0x3f, 0x3f);
const TEXT: (u8, u8, u8) = (0xe0, 0xe0, 0xe0);
const SUBTEXT0: (u8, u8, u8) = (0xbb, 0xbb, 0xbb);
const OVERLAY0: (u8, u8, u8) = (0x80, 0x80, 0x80);
const BLUE: (u8, u8, u8) = (0x61, 0xaf, 0xef);
const GREEN: (u8, u8, u8) = (0xa6, 0xe3, 0xa1);

pub fn theme_selector(term: &mut TermWizTerminal) -> anyhow::Result<()> {
    let mut state = ThemeState::load();
    state.render(term)?;
    state.run_loop(term)?;
    Ok(())
}

impl ThemeState {
    fn load() -> Self {
        let presets = theme_presets();
        let active = read_theme_id();
        let selected = presets
            .iter()
            .position(|preset| preset.id == active)
            .unwrap_or(0);
        Self {
            active,
            selected,
            message: None,
            presets,
        }
    }

    fn render(&self, term: &mut TermWizTerminal) -> termwiz::Result<()> {
        let size = term.get_screen_size()?;
        let term_w = size.cols;
        let term_h = size.rows;
        let card_w = 68usize.min(term_w.saturating_sub(4));
        let card_h = (7 + self.presets.len() * 2).min(term_h.saturating_sub(2));
        let start_x = (term_w.saturating_sub(card_w)) / 2;
        let start_y = (term_h.saturating_sub(card_h)) / 3;
        let mut changes = vec![Change::ClearScreen(termwiz::color::ColorAttribute::Default)];

        for y in 0..term_h {
            changes.push(Change::CursorPosition {
                x: Position::Absolute(0),
                y: Position::Absolute(y),
            });
            changes.push(fg_bg(" ".repeat(term_w), CRUST, CRUST));
        }

        for y in 0..card_h {
            changes.push(Change::CursorPosition {
                x: Position::Absolute(start_x),
                y: Position::Absolute(start_y + y),
            });
            changes.push(fg_bg(" ".repeat(card_w), TEXT, MANTLE));
        }

        let mut row = start_y;
        text_row(
            &mut changes,
            start_x,
            row,
            card_w,
            &crate::i18n::t("theme.title"),
            BLUE,
            MANTLE,
        );
        row += 1;
        text_row(
            &mut changes,
            start_x,
            row,
            card_w,
            &crate::i18n::t_args("theme.current", &[("name", &self.active)]),
            GREEN,
            MANTLE,
        );
        row += 1;
        separator(&mut changes, start_x, row, card_w);
        row += 1;

        for (idx, preset) in self.presets.iter().enumerate() {
            let selected = idx == self.selected;
            let bg = if selected { SURFACE0 } else { MANTLE };
            let indicator = if selected { ">" } else { " " };
            let current = if preset.id == self.active { "*" } else { " " };
            let translated_name = crate::i18n::t(&format!("theme.preset.{}.name", preset.id));
            let translated_desc = crate::i18n::t(&format!("theme.preset.{}.desc", preset.id));
            text_row(
                &mut changes,
                start_x,
                row,
                card_w,
                &format!(
                    "{indicator}{current} {} - {}",
                    translated_name, preset.scheme
                ),
                if selected { TEXT } else { SUBTEXT0 },
                bg,
            );
            row += 1;
            text_row(
                &mut changes,
                start_x,
                row,
                card_w,
                &format!("   {}", translated_desc),
                OVERLAY0,
                bg,
            );
            row += 1;
        }

        separator(&mut changes, start_x, row, card_w);
        row += 1;
        let footer = self
            .message
            .clone()
            .unwrap_or_else(|| crate::i18n::t("theme.footer.hint"));
        text_row(
            &mut changes,
            start_x,
            row,
            card_w,
            &footer,
            OVERLAY0,
            MANTLE,
        );

        changes.push(Change::AllAttributes(CellAttributes::default()));
        changes.push(Change::CursorPosition {
            x: Position::Absolute(0),
            y: Position::Absolute(0),
        });
        term.render(&changes)
    }

    fn apply_selected(&mut self) {
        if let Some(preset) = self.presets.get(self.selected) {
            if let Err(err) = apply_theme_preset(preset) {
                self.message = Some(crate::i18n::t_args(
                    "theme.apply_failed",
                    &[("err", &format!("{err}"))],
                ));
                return;
            }
            self.active = preset.id.to_string();
            let translated_name = crate::i18n::t(&format!("theme.preset.{}.name", preset.id));
            self.message = Some(crate::i18n::t_args(
                "theme.applied",
                &[("name", &translated_name)],
            ));
        }
    }

    fn run_loop(&mut self, term: &mut TermWizTerminal) -> anyhow::Result<()> {
        while let Ok(Some(event)) = term.poll_input(None) {
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
                    ..
                }) => {
                    self.selected = self.selected.saturating_sub(1);
                    self.message = None;
                    self.render(term)?;
                }
                InputEvent::Key(KeyEvent {
                    key: KeyCode::DownArrow,
                    ..
                })
                | InputEvent::Key(KeyEvent {
                    key: KeyCode::Char('j'),
                    ..
                }) => {
                    self.selected = (self.selected + 1).min(self.presets.len().saturating_sub(1));
                    self.message = None;
                    self.render(term)?;
                }
                InputEvent::Key(KeyEvent {
                    key: KeyCode::Enter,
                    ..
                }) => {
                    self.apply_selected();
                    self.render(term)?;
                }
                _ => {}
            }
        }
        Ok(())
    }
}

pub(crate) fn read_theme_id() -> String {
    let path = theme_config_path();
    std::fs::read_to_string(path)
        .ok()
        .and_then(|content| serde_json::from_str::<serde_json::Value>(&content).ok())
        .and_then(|value| {
            value
                .get("theme")
                .and_then(|theme| theme.as_str())
                .map(str::to_string)
        })
        .unwrap_or_else(|| "standard".to_string())
}

pub(crate) fn cycle_theme() -> anyhow::Result<(String, String)> {
    let presets = theme_presets();
    let active = read_theme_id();
    let current = presets
        .iter()
        .position(|preset| preset.id == active)
        .unwrap_or(0);
    let next = (current + 1) % presets.len();
    let preset = &presets[next];
    apply_theme_preset(preset)?;
    Ok((preset.name.to_string(), preset.scheme.to_string()))
}

fn apply_theme_preset(preset: &ThemePreset) -> anyhow::Result<()> {
    save_theme(preset)?;
    // Apply now rather than waiting for the file-watcher to notice theme.json
    // (macOS FSEvents lag + the watcher's settle delay made this feel slow).
    config::reload();
    apply_theme_to_gui_windows(preset.scheme)?;
    Ok(())
}

pub(crate) fn apply_theme_to_gui_windows(scheme: &str) -> anyhow::Result<()> {
    let scheme = scheme.to_string();
    let (count_tx, count_rx) = std::sync::mpsc::channel();
    let (done_tx, done_rx) = std::sync::mpsc::channel();

    promise::spawn::spawn_into_main_thread(async move {
        let windows = match crate::frontend::try_front_end() {
            Some(fe) => fe.gui_windows(),
            None => {
                let _ = count_tx.send(Err("front end is not initialized".to_string()));
                return;
            }
        };
        let count = windows.len();
        let _ = count_tx.send(Ok(count));
        for win in windows {
            let scheme = scheme.clone();
            let done_tx = done_tx.clone();
            win.window
                .notify(crate::termwindow::TermWindowNotif::Apply(Box::new(
                    move |tw| {
                        let result = tw
                            .apply_theme_palette(&scheme)
                            .map_err(|err| err.to_string());
                        let _ = done_tx.send(result);
                    },
                )));
        }
    })
    .detach();

    let count = count_rx
        .recv_timeout(std::time::Duration::from_secs(2))
        .map_err(|_| anyhow::anyhow!("timeout waiting for theme window dispatch"))?
        .map_err(anyhow::Error::msg)?;
    if count == 0 {
        return Err(anyhow::anyhow!(
            "no GUI window is registered for theme apply"
        ));
    }

    for _ in 0..count {
        done_rx
            .recv_timeout(std::time::Duration::from_secs(2))
            .map_err(|_| anyhow::anyhow!("timeout waiting for theme palette apply"))?
            .map_err(anyhow::Error::msg)?;
    }
    Ok(())
}

fn theme_presets() -> Vec<ThemePreset> {
    vec![
        ThemePreset {
            id: "standard",
            name: "Standard",
            scheme: "Unterm Dark",
            desc: "Neutral high-contrast terminal style",
        },
        ThemePreset {
            id: "midnight",
            name: "Midnight",
            scheme: "Unterm Midnight",
            desc: "Low-glare blue-black workspace",
        },
        ThemePreset {
            id: "daylight",
            name: "Daylight",
            scheme: "Unterm Daylight",
            desc: "Readable light mode for bright rooms",
        },
        ThemePreset {
            id: "classic",
            name: "Classic",
            scheme: "Classic Dark",
            desc: "Plain high-contrast terminal colors",
        },
        ThemePreset {
            id: "notion-dark",
            name: "Notion Dark",
            scheme: "Notion Dark",
            desc: "Notion-inspired warm dark",
        },
        ThemePreset {
            id: "notion-light",
            name: "Notion Light",
            scheme: "Notion Light",
            desc: "Notion-inspired clean light",
        },
    ]
}

fn save_theme(preset: &ThemePreset) -> anyhow::Result<()> {
    let path = theme_config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let value = json!({
        "theme": preset.id,
        "name": preset.name,
        "color_scheme": preset.scheme,
    });
    std::fs::write(path, serde_json::to_string_pretty(&value)?)?;
    Ok(())
}

fn theme_config_path() -> std::path::PathBuf {
    dirs_next::home_dir()
        .unwrap_or_default()
        .join(".unterm")
        .join("theme.json")
}

fn separator(changes: &mut Vec<Change>, start_x: usize, row: usize, card_w: usize) {
    changes.push(Change::CursorPosition {
        x: Position::Absolute(start_x),
        y: Position::Absolute(row),
    });
    changes.push(fg_bg("-".repeat(card_w), SURFACE1, MANTLE));
}

fn text_row(
    changes: &mut Vec<Change>,
    start_x: usize,
    row: usize,
    card_w: usize,
    text: &str,
    fg: (u8, u8, u8),
    bg: (u8, u8, u8),
) {
    let visible = truncate_chars(text, card_w.saturating_sub(2));
    let pad = card_w.saturating_sub(visible.chars().count());
    changes.push(Change::CursorPosition {
        x: Position::Absolute(start_x),
        y: Position::Absolute(row),
    });
    changes.push(fg_bg(
        format!(" {}{}", visible, " ".repeat(pad - 1)),
        fg,
        bg,
    ));
}

fn truncate_chars(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    let mut out: String = text.chars().take(max.saturating_sub(3)).collect();
    out.push_str("...");
    out
}

fn fg_bg(text: String, fg: (u8, u8, u8), bg: (u8, u8, u8)) -> Change {
    Change::Text(format!(
        "\x1b[38;2;{};{};{}m\x1b[48;2;{};{};{}m{}\x1b[0m",
        fg.0, fg.1, fg.2, bg.0, bg.1, bg.2, text
    ))
}
