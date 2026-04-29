//! Unterm Context Menu — right-click overlay with common actions.
//! Catppuccin Mocha themed centered card, similar to shell_selector.
//!
//! Menu groups:
//!   1. Clipboard — Copy / Paste
//!   2. AI Features — Explain Selection / Fix Error / Chat (accent colored)
//!   3. Terminal Actions — New Tab / Shell Selector / Split Right / Split Down
//!   4. Tools — Screenshot / Find / Command Palette
//!   5. Settings & Close

use crate::termwindow::TermWindowNotif;
use config::keyassignment::{KeyAssignment, SpawnCommand, SpawnTabDomain};
use mux::termwiztermtab::TermWizTerminal;
use termwiz::cell::CellAttributes;
use termwiz::input::{InputEvent, KeyCode, KeyEvent, Modifiers};
use termwiz::surface::{Change, Position};
use termwiz::terminal::Terminal;
use window::WindowOps;

struct MenuItem {
    icon: &'static str,
    label: &'static str,
    shortcut_hint: &'static str,
    action: MenuAction,
    /// When true the icon is rendered in MAUVE accent color.
    accent: bool,
}

impl MenuItem {
    fn is_destructive(&self) -> bool {
        self.label.starts_with("Close") || self.label == "Quit"
    }
}

enum MenuAction {
    Assignment(KeyAssignment),
    Separator,
}

pub fn context_menu(
    term: &mut TermWizTerminal,
    window: ::window::Window,
    pane_id: mux::pane::PaneId,
) -> anyhow::Result<()> {
    let items = build_menu_items();
    let mut state = MenuState {
        items,
        active_idx: 0,
        window,
        pane_id,
    };
    state.render(term)?;
    state.run_loop(term)?;
    Ok(())
}

fn build_menu_items() -> Vec<MenuItem> {
    vec![
        // ── Group 1: Clipboard ──────────────────────────────────────
        MenuItem {
            icon: "  ",
            label: "Copy",
            shortcut_hint: "Ctrl+C",
            accent: false,
            action: MenuAction::Assignment(KeyAssignment::CopyTo(
                config::keyassignment::ClipboardCopyDestination::Clipboard,
            )),
        },
        MenuItem {
            icon: "  ",
            label: "Paste",
            shortcut_hint: "Ctrl+V",
            accent: false,
            action: MenuAction::Assignment(KeyAssignment::PasteFrom(
                config::keyassignment::ClipboardPasteSource::Clipboard,
            )),
        },
        // ── separator ───────────────────────────────────────────────
        MenuItem {
            icon: "──",
            label: "",
            shortcut_hint: "",
            accent: false,
            action: MenuAction::Separator,
        },
        // ── Group 2: AI Features ────────────────────────────────────
        MenuItem {
            icon: "  ",
            label: "AI Explain Selection",
            shortcut_hint: "Ctrl+Shift+E",
            accent: true,
            action: MenuAction::Assignment(KeyAssignment::FocusAiChat),
        },
        MenuItem {
            icon: "  ",
            label: "AI Fix Error",
            shortcut_hint: "",
            accent: true,
            action: MenuAction::Assignment(KeyAssignment::FocusAiChat),
        },
        MenuItem {
            icon: "  ",
            label: "AI Chat",
            shortcut_hint: "Ctrl+Shift+U",
            accent: true,
            action: MenuAction::Assignment(KeyAssignment::FocusAiChat),
        },
        // ── separator ───────────────────────────────────────────────
        MenuItem {
            icon: "──",
            label: "",
            shortcut_hint: "",
            accent: false,
            action: MenuAction::Separator,
        },
        // ── Group 3: Terminal Actions ───────────────────────────────
        MenuItem {
            icon: "  ",
            label: "New Tab",
            shortcut_hint: "Ctrl+T",
            accent: false,
            action: MenuAction::Assignment(KeyAssignment::SpawnTab(
                SpawnTabDomain::CurrentPaneDomain,
            )),
        },
        MenuItem {
            icon: "  ",
            label: "Shell Selector",
            shortcut_hint: "Ctrl+Shift+N",
            accent: false,
            action: MenuAction::Assignment(KeyAssignment::ShowShellSelector),
        },
        MenuItem {
            icon: "  ",
            label: "Split Right",
            shortcut_hint: "Ctrl+Shift+D",
            accent: false,
            action: MenuAction::Assignment(KeyAssignment::SplitHorizontal(SpawnCommand::default())),
        },
        MenuItem {
            icon: "  ",
            label: "Split Down",
            shortcut_hint: "",
            accent: false,
            action: MenuAction::Assignment(KeyAssignment::SplitVertical(SpawnCommand::default())),
        },
        // ── separator ───────────────────────────────────────────────
        MenuItem {
            icon: "──",
            label: "",
            shortcut_hint: "",
            accent: false,
            action: MenuAction::Separator,
        },
        // ── Group 4: Tools ──────────────────────────────────────────
        MenuItem {
            icon: "  ",
            label: "Select All",
            shortcut_hint: "",
            accent: false,
            action: MenuAction::Assignment(KeyAssignment::Multiple(vec![
                KeyAssignment::ActivateCopyMode,
            ])),
        },
        MenuItem {
            icon: "  ",
            label: "Find...",
            shortcut_hint: "Ctrl+Shift+F",
            accent: false,
            action: MenuAction::Assignment(KeyAssignment::Search(
                config::keyassignment::Pattern::CurrentSelectionOrEmptyString,
            )),
        },
        MenuItem {
            icon: "  ",
            label: "Command Palette",
            shortcut_hint: "Ctrl+Shift+P",
            accent: false,
            action: MenuAction::Assignment(KeyAssignment::ActivateCommandPalette),
        },
        // ── separator ───────────────────────────────────────────────
        MenuItem {
            icon: "──",
            label: "",
            shortcut_hint: "",
            accent: false,
            action: MenuAction::Separator,
        },
        // ── Group 5: Settings & Close ───────────────────────────────
        MenuItem {
            icon: "  ",
            label: "Settings...",
            shortcut_hint: "Ctrl+Shift+,",
            accent: false,
            action: MenuAction::Assignment(KeyAssignment::ShowAiSettings),
        },
        MenuItem {
            icon: "  ",
            label: "Close Pane",
            shortcut_hint: "Ctrl+Shift+W",
            accent: false,
            action: MenuAction::Assignment(KeyAssignment::CloseCurrentPane { confirm: true }),
        },
        MenuItem {
            icon: "  ",
            label: "Close Tab",
            shortcut_hint: "",
            accent: false,
            action: MenuAction::Assignment(KeyAssignment::CloseCurrentTab { confirm: true }),
        },
        MenuItem {
            icon: "  ",
            label: "Quit",
            shortcut_hint: "Alt+F4",
            accent: false,
            action: MenuAction::Assignment(KeyAssignment::QuitApplication),
        },
    ]
}

// Neutral dark palette
const MANTLE: (u8, u8, u8) = (0x1a, 0x1a, 0x1a);
const CRUST: (u8, u8, u8) = (0x10, 0x10, 0x10);
const SURFACE0: (u8, u8, u8) = (0x2d, 0x2d, 0x2d);
const SURFACE1: (u8, u8, u8) = (0x3f, 0x3f, 0x3f);
const SURFACE2: (u8, u8, u8) = (0x55, 0x55, 0x55);
const TEXT: (u8, u8, u8) = (0xe0, 0xe0, 0xe0);
const SUBTEXT0: (u8, u8, u8) = (0xbb, 0xbb, 0xbb);
const OVERLAY0: (u8, u8, u8) = (0x80, 0x80, 0x80);
const MAUVE: (u8, u8, u8) = (0x61, 0xaf, 0xef);
const RED: (u8, u8, u8) = (0xe0, 0x6c, 0x75);

struct MenuState {
    items: Vec<MenuItem>,
    active_idx: usize,
    window: ::window::Window,
    pane_id: mux::pane::PaneId,
}

impl MenuState {
    fn selectable_indices(&self) -> Vec<usize> {
        self.items
            .iter()
            .enumerate()
            .filter(|(_, item)| !matches!(item.action, MenuAction::Separator))
            .map(|(i, _)| i)
            .collect()
    }

    fn move_up(&mut self) {
        let indices = self.selectable_indices();
        if let Some(pos) = indices.iter().position(|&i| i == self.active_idx) {
            if pos > 0 {
                self.active_idx = indices[pos - 1];
            }
        }
    }

    fn move_down(&mut self) {
        let indices = self.selectable_indices();
        if let Some(pos) = indices.iter().position(|&i| i == self.active_idx) {
            if pos + 1 < indices.len() {
                self.active_idx = indices[pos + 1];
            }
        }
    }

    fn launch(&self) -> bool {
        if let Some(item) = self.items.get(self.active_idx) {
            if let MenuAction::Assignment(ref assignment) = item.action {
                self.window.notify(TermWindowNotif::PerformAssignment {
                    pane_id: self.pane_id,
                    assignment: assignment.clone(),
                    tx: None,
                });
                return true;
            }
        }
        false
    }

    fn render(&self, term: &mut TermWizTerminal) -> termwiz::Result<()> {
        let size = term.get_screen_size()?;
        let term_w = size.cols;
        let term_h = size.rows;

        // Count visible items (non-separator items + separator lines)
        let visible_count = self.items.len();
        let card_w = 52usize.min(term_w.saturating_sub(4));
        let card_h = visible_count + 5; // borders(2) + title(1) + sep(1) + footer(1)
        let start_x = (term_w.saturating_sub(card_w)) / 2;
        let start_y = (term_h.saturating_sub(card_h)) / 3;

        let mut changes: Vec<Change> =
            vec![Change::ClearScreen(termwiz::color::ColorAttribute::Default)];

        // Dimmed background
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

        // Top border
        let top = format!("╭{}╮", "─".repeat(card_w.saturating_sub(2)));
        changes.push(Change::CursorPosition {
            x: Position::Absolute(start_x),
            y: Position::Absolute(start_y),
        });
        changes.push(fg_bg(top, SURFACE1, MANTLE));

        // Title row
        let title = "  Actions";
        let accent = "◆";
        let right_pad = card_w.saturating_sub(title.chars().count() + 5);
        changes.push(Change::CursorPosition {
            x: Position::Absolute(start_x),
            y: Position::Absolute(start_y + 1),
        });
        changes.push(fg_bg("│ ".to_string(), SURFACE1, MANTLE));
        changes.push(fg_bg(accent.to_string(), MAUVE, MANTLE));
        changes.push(fg_bg(title.to_string(), TEXT, MANTLE));
        changes.push(fg_bg(
            format!("{} │", " ".repeat(right_pad)),
            SURFACE1,
            MANTLE,
        ));

        // Separator after title
        let sep = format!("├{}┤", "─".repeat(card_w.saturating_sub(2)));
        changes.push(Change::CursorPosition {
            x: Position::Absolute(start_x),
            y: Position::Absolute(start_y + 2),
        });
        changes.push(fg_bg(sep, SURFACE1, MANTLE));

        // Menu items
        for (idx, item) in self.items.iter().enumerate() {
            let y = start_y + 3 + idx;

            match &item.action {
                MenuAction::Separator => {
                    // Thin separator line
                    let line = format!("│ {} │", "─".repeat(card_w.saturating_sub(4)));
                    changes.push(Change::CursorPosition {
                        x: Position::Absolute(start_x),
                        y: Position::Absolute(y),
                    });
                    changes.push(fg_bg(line, SURFACE1, MANTLE));
                }
                MenuAction::Assignment(_) => {
                    let is_selected = idx == self.active_idx;

                    let left_part = format!(" {} {}", item.icon, item.label);
                    let right_part = format!("{} ", item.shortcut_hint);
                    let middle_pad = card_w
                        .saturating_sub(left_part.chars().count() + right_part.chars().count() + 4);

                    let (row_fg, row_bg, hint_fg) = if is_selected {
                        (TEXT, SURFACE0, MAUVE)
                    } else {
                        (SUBTEXT0, MANTLE, OVERLAY0)
                    };

                    // AI items get accent-colored icons, destructive items get red
                    let icon_fg = if item.is_destructive() {
                        RED
                    } else if item.accent {
                        MAUVE
                    } else {
                        row_fg
                    };
                    let row_fg = if item.is_destructive() { RED } else { row_fg };

                    changes.push(Change::CursorPosition {
                        x: Position::Absolute(start_x),
                        y: Position::Absolute(y),
                    });

                    if is_selected {
                        changes.push(fg_bg("│".to_string(), SURFACE1, MANTLE));
                        changes.push(fg_bg("▎".to_string(), MAUVE, row_bg));
                    } else {
                        changes.push(fg_bg("│ ".to_string(), SURFACE1, row_bg));
                    }

                    // Render icon in accent color for AI items
                    let icon_str = format!(" {}", item.icon);
                    changes.push(fg_bg(icon_str, icon_fg, row_bg));
                    // Render label
                    changes.push(fg_bg(item.label.to_string(), row_fg, row_bg));
                    changes.push(fg_bg(" ".repeat(middle_pad), row_fg, row_bg));
                    changes.push(fg_bg(right_part, hint_fg, row_bg));
                    changes.push(fg_bg(" │".to_string(), SURFACE1, MANTLE));
                }
            }
        }

        let footer_y = start_y + 3 + visible_count;

        // Bottom border
        let bottom = format!("╰{}╯", "─".repeat(card_w.saturating_sub(2)));
        changes.push(Change::CursorPosition {
            x: Position::Absolute(start_x),
            y: Position::Absolute(footer_y),
        });
        changes.push(fg_bg(bottom, SURFACE1, MANTLE));

        // Brand watermark
        let brand = "Unterm";
        let brand_x = start_x + (card_w.saturating_sub(brand.len())) / 2;
        changes.push(Change::CursorPosition {
            x: Position::Absolute(brand_x),
            y: Position::Absolute(footer_y + 1),
        });
        changes.push(fg_bg(brand.to_string(), SURFACE2, CRUST));

        // Reset
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
                    if self.launch() {
                        break;
                    }
                }
                InputEvent::Key(KeyEvent {
                    key: KeyCode::Escape,
                    ..
                }) => {
                    break;
                }
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
