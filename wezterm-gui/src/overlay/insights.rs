//! Insights overlay — read-only dashboard for the active pane.
//!
//! Bound to `KeyAssignment::ShowInsights` (default `Ctrl+Shift+I`).
//! Surfaces information that's already in memory — shell type,
//! cwd, ghost-text commit history, MCP activity — without needing
//! a network round-trip or an AI model.  Press `q` / `Esc` /
//! `Ctrl+C` to dismiss.

use mux::termwiztermtab::TermWizTerminal;
use termwiz::cell::{AttributeChange, CellAttributes, Intensity};
use termwiz::color::AnsiColor;
use termwiz::input::{InputEvent, KeyCode, KeyEvent};
use termwiz::surface::Change;
use termwiz::terminal::Terminal;

/// Static snapshot of pane / runtime state passed in to the
/// overlay. We compute this once on the GUI thread before spawning
/// the overlay so the overlay loop doesn't have to take any locks
/// the main thread also holds.
pub struct InsightsSnapshot {
    pub pane_id: u64,
    pub shell_name: String,
    pub cwd: Option<String>,
    pub cols: usize,
    pub rows: usize,
    pub recent_commits: Vec<String>,
    pub top_commits: Vec<(String, u32)>,
    pub mcp_input_count: u64,
    pub seconds_since_last_input: Option<f32>,
    pub recent_audit: Vec<String>,
    pub agents_seen: usize,
    pub pending_suggestions: usize,
    pub pending_confirmations: usize,
}

pub fn show_insights_overlay(
    mut term: TermWizTerminal,
    snap: InsightsSnapshot,
) -> anyhow::Result<()> {
    term.no_grab_mouse_in_raw_mode();
    term.render(&[Change::Title("Insights".to_string())])?;

    let mut changes = vec![
        Change::ClearScreen(termwiz::color::ColorAttribute::Default),
        Change::CursorVisibility(termwiz::surface::CursorVisibility::Hidden),
    ];

    // Header
    push_attr(&mut changes, AnsiColor::Aqua, true);
    changes.push(Change::Text(format!(
        " ✦ Unterm Insights · pane #{} · {} · {}×{}\r\n",
        snap.pane_id, snap.shell_name, snap.cols, snap.rows,
    )));
    reset_attr(&mut changes);
    changes.push(Change::Text("\r\n".to_string()));

    // CWD
    section_header(&mut changes, "Working directory");
    let cwd_text = snap.cwd.as_deref().unwrap_or("(unknown)");
    changes.push(Change::Text(format!("  {}\r\n\r\n", cwd_text)));

    // Recent commands
    section_header(&mut changes, "Recent commands (ghost-text history)");
    if snap.recent_commits.is_empty() {
        push_attr(&mut changes, AnsiColor::Silver, false);
        changes.push(Change::Text(
            "  (none yet — type a command and press Enter)\r\n".to_string(),
        ));
        reset_attr(&mut changes);
    } else {
        for (i, cmd) in snap.recent_commits.iter().take(10).enumerate() {
            changes.push(Change::Text(format!("  {:>2}. {}\r\n", i + 1, cmd)));
        }
    }
    changes.push(Change::Text("\r\n".to_string()));

    // Top commands by frequency
    section_header(&mut changes, "Top commands");
    if snap.top_commits.is_empty() {
        push_attr(&mut changes, AnsiColor::Silver, false);
        changes.push(Change::Text("  (no repeated commands yet)\r\n".to_string()));
        reset_attr(&mut changes);
    } else {
        for (cmd, count) in snap.top_commits.iter().take(5) {
            push_attr(&mut changes, AnsiColor::Yellow, false);
            changes.push(Change::Text(format!("  {:>3}× ", count)));
            reset_attr(&mut changes);
            changes.push(Change::Text(format!("{}\r\n", cmd)));
        }
    }
    changes.push(Change::Text("\r\n".to_string()));

    // MCP activity
    section_header(&mut changes, "MCP activity");
    changes.push(Change::Text(format!(
        "  PTY writes from agents: {}\r\n",
        snap.mcp_input_count
    )));
    match snap.seconds_since_last_input {
        Some(s) if s < 60.0 => {
            changes.push(Change::Text(format!("  Last write: {:.1}s ago\r\n", s)));
        }
        Some(s) => {
            changes.push(Change::Text(format!(
                "  Last write: {:.0}min ago\r\n",
                s / 60.0
            )));
        }
        None => {
            changes.push(Change::Text("  Last write: (never)\r\n".to_string()));
        }
    }
    changes.push(Change::Text(format!(
        "  Agents identified this session: {}\r\n",
        snap.agents_seen,
    )));
    changes.push(Change::Text(format!(
        "  Pending suggestions: {}    Pending confirmations: {}\r\n",
        snap.pending_suggestions, snap.pending_confirmations,
    )));
    changes.push(Change::Text("\r\n".to_string()));

    // Recent audit
    section_header(&mut changes, "Recent MCP audit entries");
    if snap.recent_audit.is_empty() {
        push_attr(&mut changes, AnsiColor::Silver, false);
        changes.push(Change::Text("  (no audit entries yet)\r\n".to_string()));
        reset_attr(&mut changes);
    } else {
        for line in snap.recent_audit.iter().take(8) {
            changes.push(Change::Text(format!("  {}\r\n", line)));
        }
    }
    changes.push(Change::Text("\r\n".to_string()));

    // Footer / dismiss hint
    push_attr(&mut changes, AnsiColor::Silver, false);
    changes.push(Change::Text(
        " Press q · Esc · Ctrl+C to close \r\n".to_string(),
    ));
    reset_attr(&mut changes);

    term.render(&changes)?;

    loop {
        match term.poll_input(None) {
            Ok(Some(InputEvent::Key(KeyEvent { key, .. }))) => match key {
                KeyCode::Char('q')
                | KeyCode::Char('Q')
                | KeyCode::Escape
                | KeyCode::Char('\u{03}') => return Ok(()),
                _ => {}
            },
            Ok(Some(InputEvent::Resized { .. })) => {
                // Overlay redraw would be nicer but the snapshot
                // is already cached — re-render the same content.
                term.render(&changes)?;
            }
            Ok(Some(_)) => {}
            Ok(None) => return Ok(()),
            Err(_) => return Ok(()),
        }
    }
}

fn section_header(out: &mut Vec<Change>, title: &str) {
    push_attr(out, AnsiColor::Lime, true);
    out.push(Change::Text(format!(" {}\r\n", title)));
    reset_attr(out);
}

fn push_attr(out: &mut Vec<Change>, color: AnsiColor, bold: bool) {
    out.push(Change::AllAttributes(CellAttributes::default()));
    out.push(AttributeChange::Foreground(color.into()).into());
    if bold {
        out.push(AttributeChange::Intensity(Intensity::Bold).into());
    }
}

fn reset_attr(out: &mut Vec<Change>) {
    out.push(Change::AllAttributes(CellAttributes::default()));
}
