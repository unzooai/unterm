//! Unterm Tab Context Menu — right-click overlay for tab actions.
//! Catppuccin Mocha themed card, shows relevant tab operations.

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
    action: TabMenuAction,
}

enum TabMenuAction {
    Assignment(KeyAssignment),
    Separator,
    DuplicateTab,
}

pub fn tab_context_menu(
    term: &mut TermWizTerminal,
    window: ::window::Window,
    pane_id: mux::pane::PaneId,
    tab_idx: usize,
) -> anyhow::Result<()> {
    let items = vec![
        MenuItem {
            icon: " +",
            label: "New Tab",
            action: TabMenuAction::Assignment(KeyAssignment::SpawnTab(
                SpawnTabDomain::CurrentPaneDomain,
            )),
        },
        MenuItem {
            icon: "  ",
            label: "Duplicate Tab",
            action: TabMenuAction::DuplicateTab,
        },
        MenuItem {
            icon: "──",
            label: "",
            action: TabMenuAction::Separator,
        },
        MenuItem {
            icon: "  ",
            label: "Split Right",
            action: TabMenuAction::Assignment(KeyAssignment::SplitHorizontal(
                SpawnCommand::default(),
            )),
        },
        MenuItem {
            icon: "  ",
            label: "Split Down",
            action: TabMenuAction::Assignment(
                KeyAssignment::SplitVertical(SpawnCommand::default()),
            ),
        },
        MenuItem {
            icon: "──",
            label: "",
            action: TabMenuAction::Separator,
        },
        MenuItem {
            icon: " <",
            label: "Move Tab Left",
            action: TabMenuAction::Assignment(KeyAssignment::MoveTabRelative(-1)),
        },
        MenuItem {
            icon: " >",
            label: "Move Tab Right",
            action: TabMenuAction::Assignment(KeyAssignment::MoveTabRelative(1)),
        },
        MenuItem {
            icon: "──",
            label: "",
            action: TabMenuAction::Separator,
        },
        MenuItem {
            icon: " x",
            label: "Close Tab",
            action: TabMenuAction::Assignment(KeyAssignment::CloseCurrentTab { confirm: true }),
        },
    ];

    let mut state = TabMenuState {
        items,
        active_idx: 0,
        window,
        pane_id,
        tab_idx,
    };
    state.render(term)?;
    state.run_loop(term)?;
    Ok(())
}

// Catppuccin Mocha palette
const MANTLE: (u8, u8, u8) = (0x1a, 0x1a, 0x1a);
const CRUST: (u8, u8, u8) = (0x10, 0x10, 0x10);
const SURFACE0: (u8, u8, u8) = (0x2d, 0x2d, 0x2d);
const SURFACE1: (u8, u8, u8) = (0x3f, 0x3f, 0x3f);
const SURFACE2: (u8, u8, u8) = (0x55, 0x55, 0x55);
const TEXT: (u8, u8, u8) = (0xe0, 0xe0, 0xe0);
const SUBTEXT0: (u8, u8, u8) = (0xbb, 0xbb, 0xbb);
const MAUVE: (u8, u8, u8) = (0x61, 0xaf, 0xef);
const RED: (u8, u8, u8) = (0xe0, 0x6c, 0x75);

struct TabMenuState {
    items: Vec<MenuItem>,
    active_idx: usize,
    window: ::window::Window,
    pane_id: mux::pane::PaneId,
    tab_idx: usize,
}

impl TabMenuState {
    fn selectable_indices(&self) -> Vec<usize> {
        self.items
            .iter()
            .enumerate()
            .filter(|(_, item)| !matches!(item.action, TabMenuAction::Separator))
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
            match &item.action {
                TabMenuAction::Assignment(assignment) => {
                    self.window.notify(TermWindowNotif::PerformAssignment {
                        pane_id: self.pane_id,
                        assignment: assignment.clone(),
                        tx: None,
                    });
                    return true;
                }
                TabMenuAction::DuplicateTab => {
                    // Duplicate = spawn new tab in same domain
                    self.window.notify(TermWindowNotif::PerformAssignment {
                        pane_id: self.pane_id,
                        assignment: KeyAssignment::SpawnTab(SpawnTabDomain::CurrentPaneDomain),
                        tx: None,
                    });
                    return true;
                }
                TabMenuAction::Separator => {}
            }
        }
        false
    }

    fn render(&self, term: &mut TermWizTerminal) -> termwiz::Result<()> {
        let size = term.get_screen_size()?;
        let term_w = size.cols;
        let term_h = size.rows;

        let visible_count = self.items.len();
        let card_w = 40usize.min(term_w.saturating_sub(4));
        let card_h = visible_count + 5;
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

        // Title
        let title_text = format!("  Tab {}", self.tab_idx + 1);
        let accent = "◆";
        let right_pad = card_w.saturating_sub(title_text.chars().count() + 5);
        changes.push(Change::CursorPosition {
            x: Position::Absolute(start_x),
            y: Position::Absolute(start_y + 1),
        });
        changes.push(fg_bg("│ ".to_string(), SURFACE1, MANTLE));
        changes.push(fg_bg(accent.to_string(), MAUVE, MANTLE));
        changes.push(fg_bg(title_text, TEXT, MANTLE));
        changes.push(fg_bg(
            format!("{} │", " ".repeat(right_pad)),
            SURFACE1,
            MANTLE,
        ));

        // Separator
        let sep = format!("├{}┤", "─".repeat(card_w.saturating_sub(2)));
        changes.push(Change::CursorPosition {
            x: Position::Absolute(start_x),
            y: Position::Absolute(start_y + 2),
        });
        changes.push(fg_bg(sep, SURFACE1, MANTLE));

        // Items
        for (idx, item) in self.items.iter().enumerate() {
            let y = start_y + 3 + idx;

            match &item.action {
                TabMenuAction::Separator => {
                    let line = format!("│ {} │", "─".repeat(card_w.saturating_sub(4)));
                    changes.push(Change::CursorPosition {
                        x: Position::Absolute(start_x),
                        y: Position::Absolute(y),
                    });
                    changes.push(fg_bg(line, SURFACE1, MANTLE));
                }
                _ => {
                    let is_selected = idx == self.active_idx;
                    let is_close = item.label == "Close Tab";
                    let left_part = format!(" {} {}", item.icon, item.label);
                    let right_pad = card_w.saturating_sub(left_part.chars().count() + 4);

                    let (row_fg, row_bg) = if is_selected {
                        if is_close {
                            (RED, SURFACE0)
                        } else {
                            (TEXT, SURFACE0)
                        }
                    } else {
                        if is_close {
                            (RED, MANTLE)
                        } else {
                            (SUBTEXT0, MANTLE)
                        }
                    };

                    changes.push(Change::CursorPosition {
                        x: Position::Absolute(start_x),
                        y: Position::Absolute(y),
                    });

                    if is_selected {
                        changes.push(fg_bg("│".to_string(), SURFACE1, MANTLE));
                        let accent_color = if is_close { RED } else { MAUVE };
                        changes.push(fg_bg("▎".to_string(), accent_color, row_bg));
                    } else {
                        changes.push(fg_bg("│ ".to_string(), SURFACE1, row_bg));
                    }
                    changes.push(fg_bg(left_part, row_fg, row_bg));
                    changes.push(fg_bg(
                        format!("{} │", " ".repeat(right_pad)),
                        SURFACE1,
                        if is_selected { row_bg } else { MANTLE },
                    ));
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

        // Brand
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
