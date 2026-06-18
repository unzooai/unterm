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
        // Generous gap between label and hint, plus side insets so text never
        // sits flush against the card borders.
        const MIN_GAP: usize = 5;
        const CHROME: usize = 4 + 2 + 4; // borders(4) + badge(2) + side insets(4)
        let widest = self
            .items
            .iter()
            .filter(|item| !item.is_separator)
            .map(|item| cw(&item.label) + cw(&item.hint) + MIN_GAP + CHROME)
            .max()
            .unwrap_or(46);
        widest.max(46)
    }

    fn render(&mut self, term: &mut TermWizTerminal) -> termwiz::Result<()> {
        let size = term.get_screen_size()?;
        let term_w = size.cols;
        let term_h = size.rows;

        let card_w = self.auto_card_width().min(term_w.saturating_sub(4));

        // Build the card as an explicit list of rows so spacing is obvious and
        // the y-offset math can't drift. A blank Spacer row sits between every
        // pair of items in the same group so each line has room to breathe;
        // group separators and the close button get their own padding too.
        enum Row {
            TopBorder,
            Title,
            TitleSep,
            Item(usize),
            GroupSep,
            Close,
            BottomBorder,
            Blank,
        }
        let mut rows = vec![Row::TopBorder, Row::Title, Row::TitleSep, Row::Blank];
        let item_count = self.items.len();
        for (idx, item) in self.items.iter().enumerate() {
            if item.is_separator {
                rows.push(Row::Blank);
                rows.push(Row::GroupSep);
                rows.push(Row::Blank);
                continue;
            }
            rows.push(Row::Item(idx));
            // Spacer between consecutive items in the same group. Skip it before
            // a group separator (which brings its own padding) and after the
            // last item (the pre-close blank handles that).
            let next_is_sep = self.items.get(idx + 1).map_or(false, |x| x.is_separator);
            let is_last = idx + 1 == item_count;
            if !is_last && !next_is_sep {
                rows.push(Row::Blank);
            }
        }
        // Two blank rows above the close button so it sits lower, clearly
        // separated from the last menu item.
        rows.push(Row::Blank);
        rows.push(Row::Blank);
        rows.push(Row::Close);
        rows.push(Row::Blank);
        rows.push(Row::BottomBorder);

        let card_h = rows.len();
        let start_x = (term_w.saturating_sub(card_w)) / 2;
        let start_y = (term_h.saturating_sub(card_h)) / 3;

        let mut changes: Vec<Change> =
            vec![Change::ClearScreen(termwiz::color::ColorAttribute::Default)];

        // Dim backdrop behind the card.
        for y in 0..term_h {
            changes.push(Change::CursorPosition {
                x: Position::Absolute(0),
                y: Position::Absolute(y),
            });
            changes.push(fg_bg(" ".repeat(term_w), CRUST, CRUST));
        }
        // Card background fill.
        for y in 0..card_h {
            changes.push(Change::CursorPosition {
                x: Position::Absolute(start_x),
                y: Position::Absolute(start_y + y),
            });
            changes.push(fg_bg(" ".repeat(card_w), TEXT, MANTLE));
        }

        // Orange "new feature" dot; red close-button pill.
        const ORANGE: (u8, u8, u8) = (0xfa, 0x9f, 0x4d);
        let red = (0xe0, 0x6c, 0x75);

        for (i, row) in rows.iter().enumerate() {
            let y = start_y + i;
            changes.push(Change::CursorPosition {
                x: Position::Absolute(start_x),
                y: Position::Absolute(y),
            });
            match row {
                Row::TopBorder => {
                    changes.push(fg_bg(
                        format!("╭{}╮", "─".repeat(card_w.saturating_sub(2))),
                        SURFACE1,
                        MANTLE,
                    ));
                }
                Row::BottomBorder => {
                    changes.push(fg_bg(
                        format!("╰{}╯", "─".repeat(card_w.saturating_sub(2))),
                        SURFACE1,
                        MANTLE,
                    ));
                }
                Row::TitleSep => {
                    changes.push(fg_bg(
                        format!("├{}┤", "─".repeat(card_w.saturating_sub(2))),
                        SURFACE1,
                        MANTLE,
                    ));
                }
                Row::Blank => {
                    changes.push(fg_bg("│".to_string(), SURFACE1, MANTLE));
                    changes.push(fg_bg(" ".repeat(card_w.saturating_sub(2)), TEXT, MANTLE));
                    changes.push(fg_bg("│".to_string(), SURFACE1, MANTLE));
                }
                Row::Title => {
                    let title = format!("  {}", crate::i18n::t("settings.title"));
                    let right_pad = card_w.saturating_sub(cw(&title) + 5);
                    changes.push(fg_bg("│ ".to_string(), SURFACE1, MANTLE));
                    changes.push(fg_bg("◆".to_string(), MAUVE, MANTLE));
                    changes.push(fg_bg(title, TEXT, MANTLE));
                    changes.push(fg_bg(
                        format!("{} │", " ".repeat(right_pad)),
                        SURFACE1,
                        MANTLE,
                    ));
                }
                Row::GroupSep => {
                    // Inset divider — reads as a soft group break, not a hard rule.
                    let inset = 3;
                    let rule = card_w.saturating_sub(4 + inset * 2);
                    changes.push(fg_bg("│ ".to_string(), SURFACE1, MANTLE));
                    changes.push(fg_bg(" ".repeat(inset), TEXT, MANTLE));
                    changes.push(fg_bg("─".repeat(rule), SURFACE1, MANTLE));
                    changes.push(fg_bg(" ".repeat(inset), TEXT, MANTLE));
                    changes.push(fg_bg(" │".to_string(), SURFACE1, MANTLE));
                }
                Row::Item(idx) => {
                    let idx = *idx;
                    let item = &self.items[idx];
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
                    // 3-space left inset + 2-space tail so text sits off the
                    // borders, with a wide gap auto-filled between label & hint.
                    let left = format!("   {}", item.label);
                    let right = format!("{}  ", item.hint);
                    let badge = if item.new_badge { "● " } else { "  " };
                    let pad = card_w.saturating_sub(cw(&left) + cw(&right) + cw(badge) + 4);
                    changes.push(fg_bg(left, row_fg, row_bg));
                    changes.push(fg_bg(" ".repeat(pad), row_fg, row_bg));
                    changes.push(fg_bg(badge.to_string(), ORANGE, row_bg));
                    changes.push(fg_bg(right, hint_fg, row_bg));
                    changes.push(fg_bg(" │".to_string(), SURFACE1, MANTLE));
                }
                Row::Close => {
                    // Interior spaces give the red pill horizontal breathing room.
                    let close_pill =
                        format!("   {}   ", crate::i18n::t("settings.menu.close").trim());
                    let close_pad = card_w.saturating_sub(cw(&close_pill) + 4);
                    let lp = close_pad / 2;
                    let rp = close_pad - lp;
                    changes.push(fg_bg("│".to_string(), SURFACE1, MANTLE));
                    changes.push(fg_bg(" ".repeat(lp), TEXT, MANTLE));
                    changes.push(fg_bg(close_pill, MANTLE, red));
                    changes.push(fg_bg(" ".repeat(rp), TEXT, MANTLE));
                    changes.push(fg_bg("│".to_string(), SURFACE1, MANTLE));
                }
            }
        }

        // Brand + version, centered one row below the card on the backdrop.
        let version = config::wezterm_version();
        let upgrade_pending = check_update_flag();
        let suffix = if upgrade_pending { " ●" } else { "" };
        let line = format!("Unterm  v{}{}", version, suffix);
        let line_w = cw(&line);
        let line_x = start_x + (card_w.saturating_sub(line_w)) / 2;
        changes.push(Change::CursorPosition {
            x: Position::Absolute(line_x),
            y: Position::Absolute(start_y + card_h),
        });
        changes.push(fg_bg("Unterm  ".to_string(), SURFACE2, CRUST));
        changes.push(fg_bg(format!("v{}", version), OVERLAY0, CRUST));
        if upgrade_pending {
            // Catppuccin green — signals "good thing available".
            const GREEN: (u8, u8, u8) = (0xa6, 0xe3, 0xa1);
            changes.push(fg_bg(suffix.to_string(), GREEN, CRUST));
        }

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

/// True iff the background updater has recorded that a newer release is
/// available. Reads `~/.unterm/update_check.json` written by
/// `crate::update_check`. Quiet on error — better to under-report
/// than to render an upgrade dot when there isn't really one.
fn check_update_flag() -> bool {
    let Some(home) = dirs_next::home_dir() else {
        return false;
    };
    let path = home.join(".unterm").join("update_check.json");
    let Ok(content) = std::fs::read_to_string(&path) else {
        return false;
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&content) else {
        return false;
    };
    value
        .get("upgrade_available")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}
