use crate::scripting::guiwin::GuiWin;
use chrono::prelude::*;
use futures::FutureExt;
use log::Level;
use mux::termwiztermtab::TermWizTerminal;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use termwiz::cell::{AttributeChange, CellAttributes, Intensity};
use termwiz::color::AnsiColor;
use termwiz::input::{InputEvent, KeyCode, KeyEvent, Modifiers};
use termwiz::lineedit::*;
use termwiz::surface::Change;
use termwiz::terminal::Terminal;

lazy_static::lazy_static! {
    static ref LATEST_LOG_ENTRY: Mutex<Option<DateTime<Local>>> = Mutex::new(None);
}

pub fn show_debug_overlay(
    mut term: TermWizTerminal,
    gui_win: GuiWin,
    opengl_info: String,
    connection_info: String,
) -> anyhow::Result<()> {
    term.no_grab_mouse_in_raw_mode();

    // The overlay was a Lua REPL that also tailed the log. With no
    // interpreter to talk to, the log is what it is for.
    fn print_new_log_entries(term: &mut TermWizTerminal) -> termwiz::Result<()> {
        let entries = env_bootstrap::ringlog::get_entries();
        let mut changes = vec![];
        for entry in entries {
            if let Some(latest) = LATEST_LOG_ENTRY.lock().unwrap().as_ref() {
                if entry.then <= *latest {
                    // already seen this one
                    continue;
                }
            }
            LATEST_LOG_ENTRY.lock().unwrap().replace(entry.then);

            changes.push(Change::AllAttributes(CellAttributes::default()));
            changes.push(Change::Text(entry.then.format("%H:%M:%S%.3f ").to_string()));

            changes.push(
                AttributeChange::Foreground(match entry.level {
                    Level::Error => AnsiColor::Maroon.into(),
                    Level::Warn => AnsiColor::Red.into(),
                    Level::Info => AnsiColor::Green.into(),
                    Level::Debug => AnsiColor::Blue.into(),
                    Level::Trace => AnsiColor::Fuchsia.into(),
                })
                .into(),
            );
            changes.push(Change::Text(entry.level.as_str().to_string()));
            changes.push(Change::AllAttributes(CellAttributes::default()));
            changes.push(AttributeChange::Intensity(Intensity::Bold).into());
            changes.push(Change::Text(format!(" {}", entry.target)));
            changes.push(Change::AllAttributes(CellAttributes::default()));
            changes.push(Change::Text(format!(
                " > {}\r\n",
                entry.msg.replace("\n", "\r\n")
            )));
        }
        term.render(&changes)
    }

    let version = config::wezterm_version();
    let triple = config::wezterm_target_triple();

    term.render(&[Change::Text(format!(
        "Debug Overlay\r\n\
         unterm version: {version} {triple}\r\n\
         Window Environment: {connection_info}\r\n\
         {opengl_info}\r\n\
         Press ESC or CTRL-D to exit\r\n",
    ))])?;

    loop {
        print_new_log_entries(&mut term)?;
        // Polling rather than blocking so new log lines appear while the
        // overlay is just sitting open.
        match term.poll_input(Some(std::time::Duration::from_millis(200)))? {
            Some(InputEvent::Key(KeyEvent {
                key: KeyCode::Escape,
                ..
            }))
            | Some(InputEvent::Key(KeyEvent {
                key: KeyCode::Char('d'),
                modifiers: Modifiers::CTRL,
            })) => return Ok(()),
            _ => {}
        }
    }
}
