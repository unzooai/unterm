//! Settings menu — opened by left-clicking the `▼` button on the tab bar.
//!
//! This is the *only* GUI menu Unterm has. The right-click gesture is direct
//! copy/paste (no menu), so the dropdown is the visible entry point for
//! configuration overlays. Items are deliberately limited to settings
//! actions; window/session operations like New Tab, Split, and Close stay
//! on keyboard shortcuts and the tab bar's `+` button.

use crate::termwindow::TermWindowNotif;
use mux::pane::PaneId;
use mux::termwiztermtab::TermWizTerminal;
use termwiz::cell::{unicode_column_width, CellAttributes};
use termwiz::input::{InputEvent, KeyCode, KeyEvent, Modifiers};
use termwiz::surface::{Change, Position};
use termwiz::terminal::Terminal;
use window::WindowOps;

/// Visible cell width — counts CJK chars as 2 cells, ascii as 1, etc. We
/// use this anywhere we're trying to compute "how many columns will this
/// string occupy in the terminal grid", because `str::chars().count()`
/// gives the wrong answer for any non-ASCII string (every Chinese / Japanese
/// / Korean / hi-IN locale broke the layout when we used chars().count()).
fn cw(s: &str) -> usize {
    unicode_column_width(s, None)
}

#[derive(Clone, Copy)]
enum Action {
    ChangeWorkingDirectory,
    OpenProjectDirectory,
    OpenFolderInSplit,
    SplitRight,
    ToggleRecording,
    ExportCurrentSession,
    OpenWebSettings,
}

struct Item {
    label: String,
    hint: String,
    action: Action,
    /// "New feature" orange dot — shown until the user clicks it once.
    new_badge: bool,
    /// Item is a section divider (rendered as a thin line, not selectable).
    is_separator: bool,
}

pub fn settings_menu(
    term: &mut TermWizTerminal,
    window: ::window::Window,
    pane_id: PaneId,
) -> anyhow::Result<()> {
    let onboarded = read_onboarded();
    let recording_on = crate::recording::recorder::current_session(pane_id).is_some();
    let recording_label = if recording_on {
        crate::i18n::t("settings.menu.recording_on")
    } else {
        crate::i18n::t("settings.menu.recording_off")
    };

    // Quick-action panel only. Anything that's "configure once and forget"
    // (themes, proxy details, sessions browser, etc.) lives in the Web
    // Settings page — building serious form UX in a terminal cell grid is a
    // dead end. The five items below are the things that genuinely need
    // current-pane context or a single button press.
    let items = vec![
        Item {
            label: crate::i18n::t("settings.menu.change_cwd"),
            hint: crate::i18n::t("settings.menu.change_cwd.hint"),
            action: Action::ChangeWorkingDirectory,
            new_badge: false,
            is_separator: false,
        },
        Item {
            label: crate::i18n::t("settings.menu.open_folder"),
            hint: crate::i18n::t("settings.menu.open_folder.hint"),
            action: Action::OpenProjectDirectory,
            new_badge: false,
            is_separator: false,
        },
        Item {
            label: crate::i18n::t("settings.menu.split_right"),
            hint: crate::i18n::t("settings.menu.split_right.hint"),
            action: Action::SplitRight,
            new_badge: false,
            is_separator: false,
        },
        Item {
            label: String::new(),
            hint: String::new(),
            action: Action::OpenWebSettings,
            new_badge: false,
            is_separator: true,
        },
        Item {
            label: recording_label,
            hint: crate::i18n::t("settings.menu.recording.hint"),
            action: Action::ToggleRecording,
            new_badge: !onboarded.session_recording,
            is_separator: false,
        },
        Item {
            label: crate::i18n::t("settings.menu.export_session"),
            hint: crate::i18n::t("settings.menu.export_session.hint"),
            action: Action::ExportCurrentSession,
            new_badge: !onboarded.session_recording,
            is_separator: false,
        },
        Item {
            label: String::new(),
            hint: String::new(),
            action: Action::OpenWebSettings,
            new_badge: false,
            is_separator: true,
        },
        Item {
            label: crate::i18n::t("settings.menu.web_settings"),
            hint: crate::i18n::t("settings.menu.web_settings.hint"),
            action: Action::OpenWebSettings,
            new_badge: true,
            is_separator: false,
        },
    ];
    let mut state = MenuState {
        items,
        active: 0,
        window,
        pane_id,
        layout: None,
    };
    state.normalize_active();
    state.render(term)?;
    state.run_loop(term)?;
    Ok(())
}

#[derive(Default, serde::Deserialize, serde::Serialize)]
struct Onboarded {
    #[serde(default)]
    session_recording: bool,
}

fn onboarded_path() -> std::path::PathBuf {
    dirs_next::home_dir()
        .unwrap_or_default()
        .join(".unterm")
        .join("onboarded.json")
}

fn read_onboarded() -> Onboarded {
    std::fs::read_to_string(onboarded_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn mark_session_recording_seen() {
    let path = onboarded_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let mut state = read_onboarded();
    state.session_recording = true;
    if let Ok(json) = serde_json::to_string_pretty(&state) {
        let _ = std::fs::write(path, json);
    }
}

struct MenuState {
    items: Vec<Item>,
    active: usize,
    window: ::window::Window,
    pane_id: PaneId,
    /// Cached layout from the last render — populated for mouse hit-testing.
    layout: Option<Layout>,
}

#[derive(Clone, Copy)]
struct Layout {
    start_x: usize,
    start_y: usize,
    card_w: usize,
    items_y_top: usize,
    close_row_y: usize,
}

const MANTLE: (u8, u8, u8) = (0x1a, 0x1a, 0x1a);
const CRUST: (u8, u8, u8) = (0x10, 0x10, 0x10);
const SURFACE0: (u8, u8, u8) = (0x2d, 0x2d, 0x2d);
const SURFACE1: (u8, u8, u8) = (0x3f, 0x3f, 0x3f);
const SURFACE2: (u8, u8, u8) = (0x55, 0x55, 0x55);
const TEXT: (u8, u8, u8) = (0xe0, 0xe0, 0xe0);
const SUBTEXT0: (u8, u8, u8) = (0xbb, 0xbb, 0xbb);
const OVERLAY0: (u8, u8, u8) = (0x80, 0x80, 0x80);
const MAUVE: (u8, u8, u8) = (0x61, 0xaf, 0xef);

impl MenuState {
    fn selectable_indices(&self) -> Vec<usize> {
        self.items
            .iter()
            .enumerate()
            .filter(|(_, item)| !item.is_separator)
            .map(|(i, _)| i)
            .collect()
    }

    fn normalize_active(&mut self) {
        let indices = self.selectable_indices();
        if !indices.contains(&self.active) {
            self.active = indices.first().copied().unwrap_or(0);
        }
    }

    fn move_up(&mut self) {
        let indices = self.selectable_indices();
        if let Some(pos) = indices.iter().position(|&i| i == self.active) {
            if pos > 0 {
                self.active = indices[pos - 1];
            }
        }
    }

    fn move_down(&mut self) {
        let indices = self.selectable_indices();
        if let Some(pos) = indices.iter().position(|&i| i == self.active) {
            if pos + 1 < indices.len() {
                self.active = indices[pos + 1];
            }
        }
    }

    fn launch(&self) {
        let item = match self.items.get(self.active) {
            Some(item) => item,
            None => return,
        };
        if item.is_separator {
            return;
        }
        let action = item.action;
        // Mark "session recording" cluster as seen the first time the user
        // engages with any of those three items.
        if matches!(
            action,
            Action::ToggleRecording | Action::ExportCurrentSession
        ) {
            mark_session_recording_seen();
        }
        let pane_id = self.pane_id;
        self.window.notify(TermWindowNotif::Apply(Box::new(
            move |termwindow| match action {
                Action::ChangeWorkingDirectory => {
                    termwindow.change_working_directory_for_pane(pane_id)
                }
                Action::OpenProjectDirectory => termwindow.open_project_directory_from_menu(),
                Action::OpenFolderInSplit => termwindow.open_folder_in_split(pane_id),
                Action::SplitRight => {
                    use config::keyassignment::{KeyAssignment, SpawnCommand};
                    if let Some(pane) = termwindow.get_active_pane_or_overlay() {
                        let _ = termwindow.perform_key_assignment(
                            &pane,
                            &KeyAssignment::SplitHorizontal(SpawnCommand::default()),
                        );
                    }
                }
                Action::ToggleRecording => termwindow.toggle_session_recording(pane_id),
                Action::ExportCurrentSession => {
                    termwindow.export_current_session(pane_id);
                }
                Action::OpenWebSettings => termwindow.open_web_settings(),
            },
        )));
    }

    /// Compute the minimum card width that fits every visible item. Each row
    /// is `[chrome 2] + "  " + label + space-pad + badge(2) + hint + " " + [chrome 1]`
    /// so we need `4 + label.len + 2 + hint.len + 2` chars (the +2 covers the
    /// gap padding between label and hint that always renders).
    fn auto_card_width(&self) -> usize {
        const MIN_GAP: usize = 2;
        const CHROME: usize = 4 + 2; // "│ " + " │" + 2 badge slot
        let widest = self
            .items
            .iter()
            .filter(|item| !item.is_separator)
            .map(|item| cw(&item.label) + cw(&item.hint) + MIN_GAP + CHROME)
            .max()
            .unwrap_or(40);
        widest.max(40)
    }

    fn render(&mut self, term: &mut TermWizTerminal) -> termwiz::Result<()> {
        let size = term.get_screen_size()?;
        let term_w = size.cols;
        let term_h = size.rows;

        let card_w = self.auto_card_width().min(term_w.saturating_sub(4));
        let card_h = self.items.len() + 5;
        let start_x = (term_w.saturating_sub(card_w)) / 2;
        let start_y = (term_h.saturating_sub(card_h)) / 3;

        let mut changes: Vec<Change> =
            vec![Change::ClearScreen(termwiz::color::ColorAttribute::Default)];

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

        let top = format!("╭{}╮", "─".repeat(card_w.saturating_sub(2)));
        changes.push(Change::CursorPosition {
            x: Position::Absolute(start_x),
            y: Position::Absolute(start_y),
        });
        changes.push(fg_bg(top, SURFACE1, MANTLE));

        let title_inner = crate::i18n::t("settings.title");
        let title = format!("  {}", title_inner);
        let right_pad = card_w.saturating_sub(cw(&title) + 5);
        changes.push(Change::CursorPosition {
            x: Position::Absolute(start_x),
            y: Position::Absolute(start_y + 1),
        });
        changes.push(fg_bg("│ ".to_string(), SURFACE1, MANTLE));
        changes.push(fg_bg("◆".to_string(), MAUVE, MANTLE));
        changes.push(fg_bg(title.clone(), TEXT, MANTLE));
        changes.push(fg_bg(
            format!("{} │", " ".repeat(right_pad)),
            SURFACE1,
            MANTLE,
        ));

        let sep = format!("├{}┤", "─".repeat(card_w.saturating_sub(2)));
        changes.push(Change::CursorPosition {
            x: Position::Absolute(start_x),
            y: Position::Absolute(start_y + 2),
        });
        changes.push(fg_bg(sep, SURFACE1, MANTLE));

        // Orange "new feature" dot.
        const ORANGE: (u8, u8, u8) = (0xfa, 0x9f, 0x4d);

        for (idx, item) in self.items.iter().enumerate() {
            let y = start_y + 3 + idx;
            changes.push(Change::CursorPosition {
                x: Position::Absolute(start_x),
                y: Position::Absolute(y),
            });

            if item.is_separator {
                let line = format!("│ {} │", "─".repeat(card_w.saturating_sub(4)));
                changes.push(fg_bg(line, SURFACE1, MANTLE));
                continue;
            }

            let is_selected = idx == self.active;
            let (row_fg, row_bg, hint_fg) = if is_selected {
                (TEXT, SURFACE0, MAUVE)
            } else {
                (SUBTEXT0, MANTLE, OVERLAY0)
            };

            if is_selected {
                changes.push(fg_bg("│".to_string(), SURFACE1, MANTLE));
                changes.push(fg_bg("▎".to_string(), MAUVE, row_bg));
            } else {
                changes.push(fg_bg("│ ".to_string(), SURFACE1, row_bg));
            }
            let left = format!("  {}", item.label);
            let right = format!("{} ", item.hint);
            let badge = if item.new_badge { "● " } else { "  " };
            let pad = card_w.saturating_sub(cw(&left) + cw(&right) + cw(badge) + 4);
            changes.push(fg_bg(left, row_fg, row_bg));
            changes.push(fg_bg(" ".repeat(pad), row_fg, row_bg));
            changes.push(fg_bg(badge.to_string(), ORANGE, row_bg));
            changes.push(fg_bg(right, hint_fg, row_bg));
            changes.push(fg_bg(" │".to_string(), SURFACE1, MANTLE));
        }

        // Close button row, sits above the bottom border so it's clearly
        // *inside* the card and clickable. ESC still works; the button is
        // for users who don't know the shortcut.
        let close_row_y = start_y + 3 + self.items.len();
        let close_label = crate::i18n::t("settings.menu.close");
        let close_pad = card_w.saturating_sub(cw(&close_label) + 4);
        let close_left_pad = close_pad / 2;
        let close_right_pad = close_pad - close_left_pad;
        changes.push(Change::CursorPosition {
            x: Position::Absolute(start_x),
            y: Position::Absolute(close_row_y),
        });
        changes.push(fg_bg("│ ".to_string(), SURFACE1, MANTLE));
        changes.push(fg_bg(" ".repeat(close_left_pad), TEXT, MANTLE));
        // Reverse-video so the button reads as a button.
        let red = (0xe0, 0x6c, 0x75);
        changes.push(fg_bg(close_label.clone(), MANTLE, red));
        changes.push(fg_bg(" ".repeat(close_right_pad), TEXT, MANTLE));
        changes.push(fg_bg(" │".to_string(), SURFACE1, MANTLE));

        let footer_y = close_row_y + 1;
        let bottom = format!("╰{}╯", "─".repeat(card_w.saturating_sub(2)));
        changes.push(Change::CursorPosition {
            x: Position::Absolute(start_x),
            y: Position::Absolute(footer_y),
        });
        changes.push(fg_bg(bottom, SURFACE1, MANTLE));

        let brand = "Unterm";
        let brand_x = start_x + (card_w.saturating_sub(brand.len())) / 2;
        changes.push(Change::CursorPosition {
            x: Position::Absolute(brand_x),
            y: Position::Absolute(footer_y + 1),
        });
        changes.push(fg_bg(brand.to_string(), SURFACE2, CRUST));

        changes.push(Change::AllAttributes(CellAttributes::default()));
        changes.push(Change::CursorPosition {
            x: Position::Absolute(0),
            y: Position::Absolute(0),
        });
        term.render(&changes)
    }

    fn run_loop(&mut self, term: &mut TermWizTerminal) -> anyhow::Result<()> {
        while let Ok(Some(event)) = term.poll_input(None) {
            match event {
                InputEvent::Key(KeyEvent {
                    key: KeyCode::UpArrow,
                    ..
                })
                | InputEvent::Key(KeyEvent {
                    key: KeyCode::Char('k'),
                    modifiers: Modifiers::NONE,
                }) => {
                    self.move_up();
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
                    self.move_down();
                    self.render(term)?;
                }
                InputEvent::Key(KeyEvent {
                    key: KeyCode::Enter,
                    ..
                }) => {
                    self.launch();
                    break;
                }
                InputEvent::Key(KeyEvent {
                    key: KeyCode::Escape,
                    ..
                }) => break,
                _ => {}
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
