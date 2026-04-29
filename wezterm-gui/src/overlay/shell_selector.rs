//! Unterm Shell Selector — centered card overlay inspired by Windows Terminal's
//! profile picker. Catppuccin Mocha themed with shell auto-detection.

use crate::termwindow::TermWindowNotif;
use config::keyassignment::{KeyAssignment, SpawnCommand};
use mux::termwiztermtab::TermWizTerminal;
use termwiz::cell::CellAttributes;
use termwiz::input::{InputEvent, KeyCode, KeyEvent, Modifiers};
use termwiz::surface::{Change, Position};
use termwiz::terminal::Terminal;
use window::WindowOps;

#[derive(Clone)]
struct ShellEntry {
    icon: &'static str,
    label: String,
    description: String,
    shortcut: Option<char>,
    command: SpawnCommand,
}

pub fn shell_selector(
    term: &mut TermWizTerminal,
    window: ::window::Window,
    pane_id: mux::pane::PaneId,
) -> anyhow::Result<()> {
    let entries = detect_shells();
    if entries.is_empty() {
        return Ok(());
    }

    let mut state = SelectorState {
        entries,
        active_idx: 0,
        window,
        pane_id,
    };

    state.render(term)?;
    state.run_loop(term)?;
    Ok(())
}

struct SelectorState {
    entries: Vec<ShellEntry>,
    active_idx: usize,
    window: ::window::Window,
    pane_id: mux::pane::PaneId,
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

impl SelectorState {
    fn render(&self, term: &mut TermWizTerminal) -> termwiz::Result<()> {
        let size = term.get_screen_size()?;
        let term_w = size.cols;
        let term_h = size.rows;

        // Card dimensions — centered on screen
        let card_w = 56usize.min(term_w.saturating_sub(4));
        let card_h = self.entries.len() + 8; // header(4) + entries + footer(3) + blank
        let start_x = (term_w.saturating_sub(card_w)) / 2;
        let start_y = (term_h.saturating_sub(card_h)) / 3; // upper third

        let mut changes: Vec<Change> =
            vec![Change::ClearScreen(termwiz::color::ColorAttribute::Default)];

        // Fill entire screen with dimmed background
        for y in 0..term_h {
            changes.push(Change::CursorPosition {
                x: Position::Absolute(0),
                y: Position::Absolute(y),
            });
            changes.push(fg_bg(" ".repeat(term_w), CRUST, CRUST));
        }

        // Draw card background
        for y in 0..card_h {
            changes.push(Change::CursorPosition {
                x: Position::Absolute(start_x),
                y: Position::Absolute(start_y + y),
            });
            changes.push(fg_bg(" ".repeat(card_w), TEXT, MANTLE));
        }

        // ── Row 0: Top border ──
        let top = format!("╭{}╮", "─".repeat(card_w.saturating_sub(2)));
        changes.push(Change::CursorPosition {
            x: Position::Absolute(start_x),
            y: Position::Absolute(start_y),
        });
        changes.push(fg_bg(top, SURFACE1, MANTLE));

        // ── Row 1: Title ──
        let title = "  New Tab";
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

        // ── Row 2: Subtitle ──
        let subtitle = "  Choose a shell profile";
        let right_pad = card_w.saturating_sub(subtitle.chars().count() + 4);
        changes.push(Change::CursorPosition {
            x: Position::Absolute(start_x),
            y: Position::Absolute(start_y + 2),
        });
        changes.push(fg_bg("│ ".to_string(), SURFACE1, MANTLE));
        changes.push(fg_bg(subtitle.to_string(), SUBTEXT0, MANTLE));
        changes.push(fg_bg(
            format!("{} │", " ".repeat(right_pad)),
            SURFACE1,
            MANTLE,
        ));

        // ── Row 3: Separator ──
        let sep = format!("├{}┤", "─".repeat(card_w.saturating_sub(2)));
        changes.push(Change::CursorPosition {
            x: Position::Absolute(start_x),
            y: Position::Absolute(start_y + 3),
        });
        changes.push(fg_bg(sep, SURFACE1, MANTLE));

        // ── Rows 4..4+N: Shell entries ──
        for (idx, entry) in self.entries.iter().enumerate() {
            let y = start_y + 4 + idx;
            let is_selected = idx == self.active_idx;

            let shortcut_str = entry
                .shortcut
                .map(|c| format!("{}", c))
                .unwrap_or_else(|| " ".to_string());

            // Layout: │ [▸] icon  Label                  desc  [N] │
            let indicator = if is_selected { "▸" } else { " " };
            let left_part = format!("  {} {} {}", indicator, entry.icon, entry.label);
            let right_part = format!("{}  ", shortcut_str);
            let middle_pad =
                card_w.saturating_sub(left_part.chars().count() + right_part.chars().count() + 4);

            let (row_fg, row_bg, shortcut_fg) = if is_selected {
                (TEXT, SURFACE0, MAUVE)
            } else {
                (SUBTEXT0, MANTLE, OVERLAY0)
            };

            changes.push(Change::CursorPosition {
                x: Position::Absolute(start_x),
                y: Position::Absolute(y),
            });

            if is_selected {
                // Selected row: accent bar on left
                changes.push(fg_bg("│".to_string(), SURFACE1, MANTLE));
                changes.push(fg_bg("▎".to_string(), MAUVE, row_bg));
            } else {
                changes.push(fg_bg("│ ".to_string(), SURFACE1, row_bg));
            }
            changes.push(fg_bg(left_part, row_fg, row_bg));
            changes.push(fg_bg(" ".repeat(middle_pad), row_fg, row_bg));
            changes.push(fg_bg(right_part, shortcut_fg, row_bg));
            changes.push(fg_bg(" │".to_string(), SURFACE1, MANTLE));
        }

        let footer_y = start_y + 4 + self.entries.len();

        // ── Separator before footer ──
        let sep = format!("├{}┤", "─".repeat(card_w.saturating_sub(2)));
        changes.push(Change::CursorPosition {
            x: Position::Absolute(start_x),
            y: Position::Absolute(footer_y),
        });
        changes.push(fg_bg(sep, SURFACE1, MANTLE));

        // ── Footer: key hints ──
        let hints = " ↑↓ Navigate   Enter Open   Esc Cancel   1-9 Quick";
        let hint_pad = card_w.saturating_sub(hints.chars().count() + 4);
        changes.push(Change::CursorPosition {
            x: Position::Absolute(start_x),
            y: Position::Absolute(footer_y + 1),
        });
        changes.push(fg_bg("│ ".to_string(), SURFACE1, MANTLE));
        changes.push(fg_bg(hints.to_string(), OVERLAY0, MANTLE));
        changes.push(fg_bg(
            format!("{} │", " ".repeat(hint_pad)),
            SURFACE1,
            MANTLE,
        ));

        // ── Bottom border ──
        let bottom = format!("╰{}╯", "─".repeat(card_w.saturating_sub(2)));
        changes.push(Change::CursorPosition {
            x: Position::Absolute(start_x),
            y: Position::Absolute(footer_y + 2),
        });
        changes.push(fg_bg(bottom, SURFACE1, MANTLE));

        // ── Brand watermark below card ──
        let brand = "Unterm";
        let brand_x = start_x + (card_w.saturating_sub(brand.len())) / 2;
        changes.push(Change::CursorPosition {
            x: Position::Absolute(brand_x),
            y: Position::Absolute(footer_y + 3),
        });
        changes.push(fg_bg(brand.to_string(), SURFACE2, CRUST));

        // Reset & hide cursor
        changes.push(Change::AllAttributes(CellAttributes::default()));
        changes.push(Change::CursorPosition {
            x: Position::Absolute(0),
            y: Position::Absolute(0),
        });

        term.render(&changes)
    }

    fn launch(&self) -> bool {
        if let Some(entry) = self.entries.get(self.active_idx) {
            let assignment = KeyAssignment::SpawnCommandInNewTab(entry.command.clone());
            self.window.notify(TermWindowNotif::PerformAssignment {
                pane_id: self.pane_id,
                assignment,
                tx: None,
            });
            true
        } else {
            false
        }
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
                    self.active_idx = self.active_idx.saturating_sub(1);
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
                        (self.active_idx + 1).min(self.entries.len().saturating_sub(1));
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
                // Number shortcuts
                InputEvent::Key(KeyEvent {
                    key: KeyCode::Char(c),
                    modifiers: Modifiers::NONE,
                }) if c.is_ascii_digit() => {
                    let num = c.to_digit(10).unwrap_or(0) as usize;
                    let idx = if num == 0 { 9 } else { num - 1 };
                    if idx < self.entries.len() {
                        self.active_idx = idx;
                        if self.launch() {
                            break;
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }
}

/// Build an ANSI true-color styled text Change
fn fg_bg(text: String, fg: (u8, u8, u8), bg: (u8, u8, u8)) -> Change {
    Change::Text(format!(
        "\x1b[38;2;{};{};{}m\x1b[48;2;{};{};{}m{}\x1b[0m",
        fg.0, fg.1, fg.2, bg.0, bg.1, bg.2, text
    ))
}

fn detect_shells() -> Vec<ShellEntry> {
    let mut shells = Vec::new();
    let mut shortcut = 1u8;

    let mut add = |icon: &'static str, label: &str, desc: &str, args: Vec<String>| {
        let sc = if shortcut <= 9 {
            let c = char::from(b'0' + shortcut);
            shortcut += 1;
            Some(c)
        } else {
            None
        };
        shells.push(ShellEntry {
            icon,
            label: label.to_string(),
            description: desc.to_string(),
            shortcut: sc,
            command: SpawnCommand {
                label: Some(label.to_string()),
                args: Some(args),
                ..SpawnCommand::default()
            },
        });
    };

    #[cfg(windows)]
    {
        let gui_exe = std::env::current_exe().ok();

        // PowerShell 7+
        if std::path::Path::new("C:\\Program Files\\PowerShell\\7\\pwsh.exe").exists() {
            add(
                "PS",
                "PowerShell 7",
                "Cross-platform shell",
                vec![
                    "C:\\Program Files\\PowerShell\\7\\pwsh.exe".to_string(),
                    "-NoLogo".to_string(),
                ],
            );

            if let Some(exe) = gui_exe.as_ref() {
                add(
                    "PS",
                    "PowerShell 7 (Admin)",
                    "Elevated PowerShell window",
                    elevated_unterm_args(
                        exe,
                        &["C:\\Program Files\\PowerShell\\7\\pwsh.exe", "-NoLogo"],
                    ),
                );
            }
        }

        // Windows PowerShell 5.1
        add(
            "PS",
            "Windows PowerShell",
            "Built-in (5.1)",
            vec!["powershell.exe".to_string(), "-NoLogo".to_string()],
        );
        if let Some(exe) = gui_exe.as_ref() {
            add(
                "PS",
                "Windows PowerShell (Admin)",
                "Elevated Windows PowerShell window",
                elevated_unterm_args(exe, &["powershell.exe", "-NoLogo"]),
            );
        }

        // CMD
        add(
            ">_",
            "Command Prompt",
            "cmd.exe",
            vec!["cmd.exe".to_string()],
        );

        // Git Bash
        let git_bash_paths = [
            "C:\\Program Files\\Git\\bin\\bash.exe",
            "C:\\Program Files (x86)\\Git\\bin\\bash.exe",
        ];
        for path in &git_bash_paths {
            if std::path::Path::new(path).exists() {
                add(
                    "$ ",
                    "Git Bash",
                    "Unix shell via Git",
                    vec![path.to_string(), "--login".to_string()],
                );
                break;
            }
        }

        // WSL
        if std::path::Path::new("C:\\Windows\\System32\\wsl.exe").exists() {
            add("~ ", "WSL", "Linux subsystem", vec!["wsl.exe".to_string()]);
        }

        // MSYS2
        if std::path::Path::new("C:\\msys64\\usr\\bin\\bash.exe").exists() {
            add(
                "$ ",
                "MSYS2 Bash",
                "MSYS2 environment",
                vec![
                    "C:\\msys64\\usr\\bin\\bash.exe".to_string(),
                    "--login".to_string(),
                ],
            );
        }

        // Nushell
        let nu_path = format!(
            "{}\\.cargo\\bin\\nu.exe",
            std::env::var("USERPROFILE").unwrap_or_default()
        );
        if std::path::Path::new(&nu_path).exists()
            || std::path::Path::new("C:\\Program Files\\nu\\bin\\nu.exe").exists()
        {
            let path = if std::path::Path::new(&nu_path).exists() {
                nu_path
            } else {
                "C:\\Program Files\\nu\\bin\\nu.exe".to_string()
            };
            add("nu", "Nushell", "Structured data shell", vec![path]);
        }
    }

    #[cfg(not(windows))]
    {
        add(
            "$ ",
            "Default Shell",
            "Login shell",
            vec![std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string())],
        );

        if std::path::Path::new("/bin/bash").exists() {
            add(
                "$ ",
                "Bash",
                "GNU Bourne-Again Shell",
                vec!["/bin/bash".to_string()],
            );
        }
        if std::path::Path::new("/bin/zsh").exists() {
            add("% ", "Zsh", "Z Shell", vec!["/bin/zsh".to_string()]);
        }
        if std::path::Path::new("/usr/bin/fish").exists() {
            add(
                "> ",
                "Fish",
                "Friendly Interactive Shell",
                vec!["/usr/bin/fish".to_string()],
            );
        }
    }

    shells
}

#[cfg(windows)]
fn elevated_unterm_args(gui_exe: &std::path::Path, shell_args: &[&str]) -> Vec<String> {
    let script = r#"
$exe = $args[0]
$argv = @()
if ($args.Length -gt 1) {
  $argv = $args[1..($args.Length - 1)]
}
Start-Process -Verb RunAs -FilePath $exe -ArgumentList $argv
"#;
    let mut args = vec![
        "powershell.exe".to_string(),
        "-NoProfile".to_string(),
        "-ExecutionPolicy".to_string(),
        "Bypass".to_string(),
        "-Command".to_string(),
        script.to_string(),
        gui_exe.display().to_string(),
        "start".to_string(),
        "--always-new-process".to_string(),
        "--".to_string(),
    ];
    args.extend(shell_args.iter().map(|arg| arg.to_string()));
    args
}
